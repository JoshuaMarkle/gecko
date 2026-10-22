//! `gecko layouts import | export | svg | import-cmini | likes`: interop with other analyzers,
//! layout images and the cmini community bundle.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::Subcommand;
use gecko_core::interop::community;
use gecko_core::interop::import::{export, import, Format};
use gecko_core::interop::svg::{board_svg, layer_heat, KeyColor, SvgOptions};
use gecko_core::{Layout, SCHEMA};
use serde_json::json;

use crate::ctx::{score_layout, Ctx, Inputs};

#[derive(Subcommand)]
pub enum LayoutsCmd {
    /// Convert an Oxeylyzer (.dof or v1 grid), genkey or cmini layout file to layout TOML.
    /// Prints the TOML unless --out.
    Import {
        file: PathBuf,
        /// oxeylyzer | genkey | cmini | auto (guess from the file).
        #[arg(long, default_value = "auto")]
        from: String,
        /// Write data/layouts/<out>.toml.
        #[arg(long)]
        out: Option<String>,
        /// Overwrite an existing data/layouts/<out>.toml.
        #[arg(long)]
        force: bool,
        /// Layout name (default: the file's name field, else the file stem).
        #[arg(long)]
        name: Option<String>,
        /// Target geometry (default: ansi-30 for staggered sources, ortho-30 for ortho).
        #[arg(long, short)]
        geometry: Option<String>,
    },
    /// Write a layout's 30 alpha keys as an Oxeylyzer .dof, genkey or cmini file (stdout).
    Export {
        layout: String,
        /// oxeylyzer | genkey | cmini.
        #[arg(long)]
        to: String,
        #[command(flatten)]
        inputs: Inputs,
    },
    /// Draw a layout on its board as a standalone SVG (the web Export tab's image): every layer,
    /// a title with the score. Prints the SVG unless --out.
    Svg {
        layout: String,
        /// plain | fingers | heat (keystroke share under the corpus).
        #[arg(long, default_value = "plain")]
        color: String,
        /// Base layer only.
        #[arg(long)]
        base_only: bool,
        /// No title or score line.
        #[arg(long)]
        no_title: bool,
        /// Write the SVG to this file.
        #[arg(long)]
        out: Option<PathBuf>,
        #[command(flatten)]
        inputs: Inputs,
    },
    /// Import a cmini checkout (layouts/*.json, likes.json, authors.json) into
    /// data/community/cmini.json: 30-key cores, likes, duplicates and mirrors marked.
    ImportCmini { dir: PathBuf },
    /// Rank the imported cmini layouts with GECKO and report the Spearman correlation with likes.
    Likes {
        /// Rows of the best-by-GECKO and most-liked tables.
        #[arg(long, default_value_t = 10)]
        top: usize,
        /// Keep duplicates and mirrors (default: one layout per group).
        #[arg(long)]
        dups: bool,
        #[command(flatten)]
        inputs: Inputs,
    },
}

impl LayoutsCmd {
    pub fn run(self, ctx: &Ctx) -> Result<()> {
        match self {
            LayoutsCmd::Import { file, from, out, force, name, geometry } => {
                import_file(ctx, &file, &from, out.as_deref(), force, name, geometry.as_deref())
            }
            LayoutsCmd::Export { layout, to, inputs } => {
                let f = Format::parse(&to)?;
                let g = ctx.store.geometry(inputs.geometry.as_deref().unwrap_or(&ctx.store.defaults.geometry))?;
                let lay = ctx.store.layout(&layout, &g.name)?.fit(&g)?;
                let text = export(f, &lay, &g)?;
                if ctx.json {
                    ctx.print(json!({"schema": SCHEMA, "format": f.name(), "layout": lay.name, "text": text}));
                } else {
                    print!("{}", if text.ends_with('\n') { text } else { text + "\n" });
                }
                Ok(())
            }
            LayoutsCmd::Svg { layout, color, base_only, no_title, out, inputs } => {
                svg(ctx, &layout, KeyColor::parse(&color)?, !base_only, !no_title, out.as_deref(), &inputs)
            }
            LayoutsCmd::ImportCmini { dir } => import_cmini(ctx, &dir),
            LayoutsCmd::Likes { top, dups, inputs } => likes(ctx, top, dups, &inputs),
        }
    }
}

fn svg(ctx: &Ctx, spec: &str, color: KeyColor, layers: bool, title: bool, out: Option<&std::path::Path>, inputs: &Inputs) -> Result<()> {
    let l = ctx.load(inputs)?;
    let lay = ctx.layout(spec, &l)?;
    let (sc, st) = score_layout(&l, &lay)?;
    let a = gecko_core::report::analyze(&sc, &st, 0);
    let heat = layer_heat(&l.geometry, &lay, &a)?;
    let sub = format!("{} · {} · {} · score {:.3}", l.geometry.name, l.corpus.name, l.profile.name, a.breakdown.score);
    let o = SvgOptions { color, layers, title: title.then(|| (lay.name.clone(), sub)) };
    let text = board_svg(&l.geometry, &lay, &heat, &o);
    if let Some(path) = out {
        ctx.write_file(path, &text)?;
    }
    if ctx.json {
        let mut v = json!({"schema": SCHEMA, "layout": lay.name, "geometry": l.geometry.name, "score": a.breakdown.score});
        match out {
            Some(p) => v["path"] = json!(p),
            None => v["svg"] = json!(text),
        }
        ctx.print(v);
    } else if out.is_none() {
        println!("{text}");
    }
    Ok(())
}

fn import_file(
    ctx: &Ctx,
    file: &std::path::Path,
    from: &str,
    out: Option<&str>,
    force: bool,
    name: Option<String>,
    geometry: Option<&str>,
) -> Result<()> {
    let text = std::fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
    let stem = file.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let mut imp = import(Format::parse(from)?, &text, &stem)?;
    if let Some(n) = name {
        imp.name = n;
    }
    let lf = imp.to_file(geometry);
    // The layout must load on its geometry (fills added on boards with thumb keys).
    let g = ctx.store.geometry(&lf.geometry)?;
    Layout::from_rows(&lf.name, &lf.geometry, &lf.source, &lf.rows)?.fit(&g)?.check_geometry(&g)?;
    for w in &imp.warnings {
        eprintln!("warning: {w}");
    }
    let toml_text = toml::to_string(&lf)?;
    let path = match out {
        Some(o) => {
            if o.is_empty() || o.contains(['/', '\\']) || o.starts_with('.') {
                bail!("--out takes a layout name, not a path: '{o}'");
            }
            let p = ctx.store.data.join("layouts").join(format!("{o}.toml"));
            if p.exists() && !force {
                bail!("{} exists (use --force to overwrite)", p.display());
            }
            ctx.write_file(&p, &toml_text)?;
            Some(p)
        }
        None => None,
    };
    if ctx.json {
        ctx.print(json!({"schema": SCHEMA, "name": lf.name, "geometry": lf.geometry, "source": lf.source,
            "rows": lf.rows, "warnings": imp.warnings, "path": path.map(|p| p.display().to_string())}));
    } else if path.is_none() {
        print!("{toml_text}");
    }
    Ok(())
}

fn import_cmini(ctx: &Ctx, dir: &std::path::Path) -> Result<()> {
    let b = community::import_cmini_dir(dir)?;
    let path = ctx.store.community_path();
    ctx.write_file(&path, &b.to_json())?;
    let mut reasons: std::collections::BTreeMap<String, usize> = Default::default();
    for sk in &b.skipped {
        *reasons.entry(sk.kind().to_string()).or_default() += 1;
    }
    let dups = b.layouts.iter().filter(|l| l.dup_of.is_some() && !l.mirror).count();
    let mirrors = b.layouts.iter().filter(|l| l.mirror).count();
    let liked = b.unique().filter(|l| l.likes > 0).count();
    if ctx.json {
        ctx.print(json!({"schema": SCHEMA, "path": path, "imported": b.layouts.len(), "unique": b.unique().count(),
            "duplicates": dups, "mirrors": mirrors, "liked": liked, "skipped": b.skipped.len(), "skip_reasons": reasons}));
        return Ok(());
    }
    println!(
        "{} imported ({} unique, {} liked), {} duplicates, {} mirrors, {} skipped",
        b.layouts.len(),
        b.unique().count(),
        liked,
        dups,
        mirrors,
        b.skipped.len()
    );
    for (r, n) in &reasons {
        println!("  skipped {n:>5}  {r}");
    }
    Ok(())
}

fn likes(ctx: &Ctx, top: usize, dups: bool, inputs: &Inputs) -> Result<()> {
    let l = ctx.load(inputs)?;
    let b = ctx.store.community()?;
    let r = if dups {
        community::likes_report(&l.geometry, &l.corpus, &l.profile, b.layouts.iter())?
    } else {
        community::likes_report(&l.geometry, &l.corpus, &l.profile, b.unique())?
    };
    if ctx.json {
        ctx.print(json!({"schema": SCHEMA, "report": r}));
        return Ok(());
    }
    println!("{} cmini layouts on {} · {} · {}", r.all.n, r.geometry, r.profile, r.corpus);
    println!("Spearman ρ (GECKO goodness vs likes; > 0 = agree)");
    let rho = |v: f64| if v.is_finite() { format!("{v:+.3}") } else { "-".into() };
    println!("  all          n={:<5} ρ={}", r.all.n, rho(r.all.rho));
    println!("  liked only   n={:<5} ρ={}", r.liked.n, rho(r.liked.rho));
    for (bd, a) in &r.boards {
        println!("  {:<12} n={:<5} ρ={}", bd, a.n, rho(a.rho));
    }
    println!(
        "mean GECKO rank percentile: liked {:.1}%, unliked {:.1}% (0 = best)",
        100.0 * r.liked_percentile,
        100.0 * r.unliked_percentile
    );
    let mut ms = r.metrics.clone();
    ms.sort_by(|a, b| b.1.total_cmp(&a.1));
    println!("metric goodness vs likes (ρ): {}", ms.iter().map(|(id, v)| format!("{id} {v:+.2}")).collect::<Vec<_>>().join(", "));
    println!("\nbest by GECKO");
    for s in r.layouts.iter().take(top) {
        println!("  {:>5} {:<32} {:>8.3} {:>4} likes  {}", s.rank, s.name, s.score, s.likes, s.board);
    }
    let mut liked: Vec<_> = r.layouts.iter().collect();
    liked.sort_by(|a, b| b.likes.cmp(&a.likes).then(a.rank.cmp(&b.rank)));
    println!("\nmost liked (GECKO rank of {})", r.all.n);
    for s in liked.iter().take(top) {
        println!("  {:>5} {:<32} {:>8.3} {:>4} likes  {}", s.rank, s.name, s.score, s.likes, s.board);
    }
    Ok(())
}
