//! `gecko` — the agent-first CLI and `gecko serve`. See `context/cli.md`.

mod explore_cmd;
mod serve;

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use gecko_core::metrics::Dir;
use gecko_core::optimize::{Algo, Control, Params};
use gecko_core::report::{analyze, breakdown, Breakdown};
use gecko_core::run::{self, RunSpec};
use gecko_core::store::Store;
use gecko_core::{Corpus, Layout, Scorer, SCHEMA};
use serde_json::json;

#[derive(Parser)]
#[command(name = "gecko", version, about = "Find the best keyboard layouts.")]
struct Cli {
    /// Repo root holding gecko.toml (default: search upward from the current directory).
    #[arg(long, global = true)]
    root: Option<PathBuf>,
    /// Print JSON (stable schema) instead of tables.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(clap::Args, Clone)]
struct Inputs {
    /// Geometry name or path (default from gecko.toml).
    #[arg(long, short)]
    geometry: Option<String>,
    /// Profile name or path.
    #[arg(long, short)]
    profile: Option<String>,
    /// Compiled corpus name.
    #[arg(long, short)]
    corpus: Option<String>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Score one layout with a per-metric breakdown.
    Score {
        /// Layout: reference name, .toml path, run:<id>[#rank], or raw:<chars>.
        layout: String,
        #[command(flatten)]
        inputs: Inputs,
    },
    /// Rank several layouts side by side (all reference layouts when none are given).
    Compare {
        layouts: Vec<String>,
        /// Robustness report: score and rank on each of these corpora (comma-separated).
        #[arg(long, value_delimiter = ',')]
        corpora: Vec<String>,
        #[command(flatten)]
        inputs: Inputs,
    },
    /// Finger loads, heat, and the worst same-finger and stretch n-grams.
    Analyze {
        layout: String,
        #[arg(long, default_value_t = 10)]
        top: usize,
        #[command(flatten)]
        inputs: Inputs,
    },
    /// Search for better layouts. Always writes a run directory.
    Optimize {
        #[arg(long, default_value = "sa")]
        algo: String,
        /// Time budget, e.g. 30s, 2m, 500ms.
        #[arg(long)]
        time: Option<String>,
        /// Evaluation budget (deterministic; overrides --time).
        #[arg(long)]
        evals: Option<u64>,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Worker threads (default: all cores).
        #[arg(long)]
        threads: Option<usize>,
        /// Symbols that keep their start position, e.g. ",./;".
        #[arg(long, default_value = "")]
        pin: String,
        /// Start layout (also defines the symbol set).
        #[arg(long)]
        from: Option<String>,
        #[arg(long, default_value_t = 10)]
        top: usize,
        /// SA: swap attempts per restart.
        #[arg(long)]
        sa_steps: Option<u64>,
        /// GA: population size.
        #[arg(long)]
        ga_pop: Option<usize>,
        #[command(flatten)]
        inputs: Inputs,
    },
    /// Map the space of good layouts: landscape probe + breakout local search with exclusion
    /// restarts, archiving every local optimum. Writes an atlas run directory.
    Explore {
        /// Time budget, e.g. 2m (default 60s).
        #[arg(long)]
        time: Option<String>,
        /// Evaluation budget (deterministic with --threads 1).
        #[arg(long)]
        evals: Option<u64>,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long)]
        threads: Option<usize>,
        #[arg(long, default_value = "")]
        pin: String,
        #[arg(long)]
        from: Option<String>,
        /// Random layouts measured by the landscape probe.
        #[arg(long)]
        samples: Option<usize>,
        /// Archive cap (the worst optima beyond it are dropped).
        #[arg(long)]
        max_optima: Option<usize>,
        #[command(flatten)]
        inputs: Inputs,
    },
    /// Landscape probe without searching: random-layout score distribution, correlation
    /// length, and where each reference layout sits among random layouts.
    Landscape {
        #[arg(long, default_value_t = 4000)]
        samples: usize,
        #[arg(long, default_value_t = 64)]
        walks: usize,
        #[arg(long, default_value_t = 400)]
        walk_len: usize,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Layout whose symbol set is probed (default: the reference layout).
        #[arg(long)]
        from: Option<String>,
        #[command(flatten)]
        inputs: Inputs,
    },
    /// Read an explore atlas: summary, ranked optima, families and consensus. With -p, re-rank
    /// the archive under another profile (no new search).
    Atlas {
        /// Run id of an explore run.
        run: String,
        #[arg(long, default_value_t = 10)]
        top: usize,
        /// Family radius: slots that may differ from the family's leader.
        #[arg(long, default_value_t = 8)]
        cut: usize,
        /// How many of the best optima are grouped into families.
        #[arg(long, default_value_t = 300)]
        pool: usize,
        #[command(flatten)]
        inputs: Inputs,
    },
    /// Algorithm leaderboard: each algorithm over several seeds with the same budget.
    Bench {
        #[arg(long, default_value = "hc,sa,ga")]
        algos: String,
        #[arg(long, default_value_t = 3)]
        seeds: u64,
        #[arg(long, default_value = "5s")]
        time: String,
        #[arg(long)]
        threads: Option<usize>,
        #[command(flatten)]
        inputs: Inputs,
    },
    /// Single-thread scorer speed: full evaluations/s and swap deltas/s.
    Perf {
        #[arg(long, default_value = "qwerty")]
        layout: String,
        #[command(flatten)]
        inputs: Inputs,
    },
    /// Corpus tools.
    Corpus {
        #[command(subcommand)]
        cmd: CorpusCmd,
    },
    /// Profile calibration tools.
    Profile {
        #[command(subcommand)]
        cmd: ProfileCmd,
    },
    /// List layouts, geometries, profiles, corpora, runs or metrics.
    List { what: String },
    /// Serve the web UI and JSON API.
    Serve {
        #[arg(long, default_value_t = 7777)]
        port: u16,
        /// Directory with the built web app (default: <root>/web/dist).
        #[arg(long)]
        web: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum CorpusCmd {
    /// Count n-grams of text files (or directories, searched recursively for files with the
    /// `--ext` extensions) into data/corpora/<out>.gkc.
    Build {
        sources: Vec<PathBuf>,
        #[arg(long, default_value = "english")]
        out: String,
        /// Symbols to count, or `default` / `code` (adds digits, all ASCII punctuation, Enter).
        #[arg(long, default_value = "default")]
        charset: String,
        /// File extensions to read from directories, comma-separated (e.g. `rs,py,ts`).
        #[arg(long, default_value = "txt")]
        ext: String,
    },
    /// Weighted mix of corpora with the same charset (each normalized first), e.g.
    /// `gecko corpus mix code-rs:1 code-py:1 --out code`.
    Mix {
        /// `<corpus>[:weight]` (weight 1 when omitted).
        parts: Vec<String>,
        #[arg(long)]
        out: String,
    },
    /// Import precomputed counts from a cmini-style directory: words.json (preferred), else
    /// monograms/bigrams/trigrams.json. Writes data/corpora/<out>.gkc.
    Import {
        dir: PathBuf,
        #[arg(long)]
        out: String,
        #[arg(long, default_value = "default")]
        charset: String,
    },
}

#[derive(Subcommand)]
enum ProfileCmd {
    /// Measure each metric's spread over reference layouts and re-express the profile's weights
    /// in spread units (same objective, comparable weights). Prints TOML unless --out.
    Normalize {
        /// Layouts to measure spread on (default: all reference layouts).
        #[arg(long, value_delimiter = ',')]
        layouts: Vec<String>,
        /// Write data/profiles/<out>.toml.
        #[arg(long)]
        out: Option<String>,
        #[command(flatten)]
        inputs: Inputs,
    },
    /// Fit non-negative spread-unit weights so the other metrics predict a judge metric (e.g. E05)
    /// over layouts sampled around the references. Prints TOML unless --out.
    Fit {
        #[arg(long, default_value = "E05")]
        target: String,
        #[arg(long, value_delimiter = ',')]
        layouts: Vec<String>,
        #[arg(long)]
        out: Option<String>,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[command(flatten)]
        inputs: Inputs,
    },
}

/// Print a profile as TOML, or write it to data/profiles/<out>.toml.
fn emit_profile(ctx: &Ctx, p: &gecko_core::Profile, out: Option<&str>) -> Result<()> {
    // Profile numbers are f32; print each at its shortest f32 decimal, not the widened f64.
    fn tidy(v: &mut toml::Value) {
        match v {
            toml::Value::Float(f) => *f = format!("{}", *f as f32).parse().unwrap_or(*f),
            toml::Value::Array(a) => a.iter_mut().for_each(tidy),
            toml::Value::Table(t) => t.iter_mut().for_each(|(_, v)| tidy(v)),
            _ => {}
        }
    }
    let mut v = toml::Value::try_from(p)?;
    tidy(&mut v);
    let text = toml::to_string(&v)?;
    match out {
        Some(name) => {
            let path = ctx.store.data.join("profiles").join(format!("{name}.toml"));
            std::fs::write(&path, &text).with_context(|| format!("writing {}", path.display()))?;
            if !ctx.json {
                println!("wrote {}", path.display());
            }
        }
        None if !ctx.json => print!("{text}"),
        None => {}
    }
    Ok(())
}

fn reference_layouts(ctx: &Ctx, l: &Loaded, names: Vec<String>) -> Result<(Vec<Layout>, String)> {
    let names = if names.is_empty() { ctx.store.list("layouts") } else { names };
    let layouts = names.iter().map(|n| ctx.layout(n, l)).collect::<Result<Vec<_>>>()?;
    let basis = format!("stdev over {} layouts ({}) on {} / {}", layouts.len(), names.join(", "), l.corpus.name, l.geometry.name);
    Ok((layouts, basis))
}

/// Files under `dir` (recursively, skipping hidden directories) with one of `exts`.
fn walk(dir: &std::path::Path, exts: &[&str], out: &mut Vec<PathBuf>) -> Result<()> {
    for e in std::fs::read_dir(dir)?.flatten() {
        let p = e.path();
        if p.is_dir() {
            if !e.file_name().to_string_lossy().starts_with('.') {
                walk(&p, exts, out)?;
            }
        } else if p.extension().is_some_and(|x| exts.iter().any(|e| x == *e)) {
            out.push(p);
        }
    }
    Ok(())
}

/// A JSON object of `"gram": count` pairs.
fn read_counts(path: &std::path::Path) -> Result<Vec<(String, u64)>> {
    let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let map: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&text).with_context(|| format!("{}: expected an object of counts", path.display()))?;
    map.into_iter()
        .map(|(k, v)| {
            let n = v.as_u64().or_else(|| v.as_f64().map(|f| f.round() as u64));
            n.map(|n| (k.clone(), n)).with_context(|| format!("{}: count for '{k}' is not a number", path.display()))
        })
        .collect()
}

fn write_corpus(ctx: &Ctx, c: &Corpus, files: usize) -> Result<()> {
    let path = ctx.store.corpus_path(&c.name);
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(&path, c.to_bytes())?;
    let v = json!({"schema": SCHEMA, "corpus": c.name, "path": path, "files": files, "chars": c.total_chars(), "hash": gecko_core::hash::hex(c.hash())});
    if ctx.json {
        ctx.print(v);
    } else {
        println!("wrote {} ({} files, {} chars)", path.display(), files, c.total_chars());
    }
    Ok(())
}

fn parse_duration(s: &str) -> Result<u64> {
    let s = s.trim();
    let (num, mul) = if let Some(v) = s.strip_suffix("ms") {
        (v, 1.0)
    } else if let Some(v) = s.strip_suffix('s') {
        (v, 1000.0)
    } else if let Some(v) = s.strip_suffix('m') {
        (v, 60_000.0)
    } else if let Some(v) = s.strip_suffix('h') {
        (v, 3_600_000.0)
    } else {
        (s, 1000.0)
    };
    let v: f64 = num.parse().with_context(|| format!("bad duration '{s}' (use e.g. 30s, 2m, 500ms)"))?;
    Ok((v * mul) as u64)
}

struct Ctx {
    store: Store,
    json: bool,
}

struct Loaded {
    geometry: gecko_core::Geometry,
    profile: gecko_core::Profile,
    corpus: Corpus,
}

impl Ctx {
    fn load(&self, i: &Inputs) -> Result<Loaded> {
        let d = &self.store.defaults;
        Ok(Loaded {
            geometry: self.store.geometry(i.geometry.as_deref().unwrap_or(&d.geometry))?,
            profile: self.store.profile(i.profile.as_deref().unwrap_or(&d.profile))?,
            corpus: self.store.corpus(i.corpus.as_deref().unwrap_or(&d.corpus))?,
        })
    }

    fn layout(&self, spec: &str, l: &Loaded) -> Result<Layout> {
        Ok(self.store.layout(spec, &l.geometry.name)?.fit(&l.geometry)?)
    }

    fn print(&self, v: serde_json::Value) {
        println!("{}", serde_json::to_string_pretty(&v).unwrap());
    }
}

fn score_layout(l: &Loaded, layout: &Layout) -> Result<(Scorer, gecko_core::State)> {
    let sc = Scorer::new(&l.geometry, &l.corpus, &l.profile, layout)?;
    let st = sc.identity();
    Ok((sc, st))
}

fn fmt_val(v: f64) -> String {
    if v.abs() >= 100.0 {
        format!("{v:.1}")
    } else {
        format!("{v:.2}")
    }
}

fn print_breakdown(b: &Breakdown) {
    println!("{:<5} {:<18} {:>9} {:>7} {:>9}", "id", "metric", "value", "weight", "score");
    for m in &b.metrics {
        let unit = if m.unit == "%" { "%" } else { "" };
        let dir = match m.dir {
            Dir::Min => "",
            Dir::Max => " +",
            Dir::Info => " i",
        };
        println!(
            "{:<5} {:<18} {:>8}{:1} {:>7} {:>9}",
            m.id,
            format!("{}{dir}", m.name),
            fmt_val(m.value),
            unit,
            if m.weight == 0.0 { "-".into() } else { format!("{}", m.weight) },
            if m.weighted == 0.0 { "-".into() } else { format!("{:.3}", m.weighted) }
        );
    }
    println!("{:<5} {:<18} {:>36.3}", "", "score (lower=better)", b.score);
}

fn layout_rows(layout: &Layout, g: &gecko_core::Geometry) -> String {
    let rows = |rs: &[String]| rs.iter().map(|r| r.chars().map(|c| c.to_string()).collect::<Vec<_>>().join(" ")).collect::<Vec<_>>().join("\n  ");
    let mut out = rows(&layout.rows(g));
    let modes = layout.holders().unwrap_or_default();
    for (li, (ly, rs)) in layout.layers.iter().zip(layout.layer_rows(g)).enumerate() {
        let mode = modes.get(li).filter(|h| h.1 != gecko_core::LayerMode::Hold).map_or(String::new(), |h| format!(", {}", h.1.name()));
        out += &format!("\n  [{}{mode}]\n  {}", ly.name, rows(&rs));
    }
    out
}

/// `compare --corpora`: each layout's score and rank per corpus, and how far its rank moves (L05).
fn robustness(ctx: &Ctx, layouts: Vec<String>, corpora: &[String], inputs: &Inputs) -> Result<()> {
    let base = ctx.load(inputs)?;
    let names = if layouts.is_empty() { ctx.store.list("layouts") } else { layouts };
    let layouts: Vec<Layout> = names.iter().map(|n| ctx.layout(n, &base)).collect::<Result<_>>()?;
    let nl = layouts.len();
    // score[c][i], sfb[c][i], rank[c][i] (1 = best)
    let mut score = vec![vec![0.0; nl]; corpora.len()];
    let mut sfb = score.clone();
    let mut rank = vec![vec![0usize; nl]; corpora.len()];
    for (ci, cn) in corpora.iter().enumerate() {
        let l = Loaded { geometry: base.geometry.clone(), profile: base.profile.clone(), corpus: ctx.store.corpus(cn)? };
        for (i, lay) in layouts.iter().enumerate() {
            let (sc, st) = score_layout(&l, lay)?;
            let b = breakdown(&sc, &st);
            score[ci][i] = b.score;
            sfb[ci][i] = b.metrics.iter().find(|m| m.id == "B01").map_or(0.0, |m| m.value);
        }
        let mut order: Vec<usize> = (0..nl).collect();
        order.sort_by(|&a, &b| score[ci][a].total_cmp(&score[ci][b]));
        for (r, &i) in order.iter().enumerate() {
            rank[ci][i] = r + 1;
        }
    }
    let nc = corpora.len() as f64;
    let mean_rank = |i: usize| rank.iter().map(|r| r[i] as f64).sum::<f64>() / nc;
    let rank_spread = |i: usize| rank.iter().map(|r| r[i]).max().unwrap() - rank.iter().map(|r| r[i]).min().unwrap();
    let mut order: Vec<usize> = (0..nl).collect();
    order.sort_by(|&a, &b| mean_rank(a).total_cmp(&mean_rank(b)).then(score[0][a].total_cmp(&score[0][b])));
    if ctx.json {
        let v: Vec<_> = order
            .iter()
            .map(|&i| {
                let per: serde_json::Map<String, serde_json::Value> = corpora
                    .iter()
                    .enumerate()
                    .map(|(c, n)| (n.clone(), json!({"score": score[c][i], "rank": rank[c][i], "sfb": sfb[c][i]})))
                    .collect();
                json!({"layout": layouts[i].name, "corpora": per, "mean_rank": mean_rank(i), "rank_spread": rank_spread(i)})
            })
            .collect();
        ctx.print(
            json!({"schema": SCHEMA, "geometry": base.geometry.name, "profile": base.profile.name, "corpora": corpora, "layouts": v}),
        );
        return Ok(());
    }
    println!("score (rank) [SFB %] per corpus, sorted by mean rank; spread = worst rank - best rank");
    print!("{:<14}", "layout");
    for c in corpora {
        print!(" {:>22}", c);
    }
    println!(" {:>9} {:>6}", "mean rank", "spread");
    for &i in &order {
        print!("{:<14}", layouts[i].name);
        for c in 0..corpora.len() {
            print!(" {:>22}", format!("{:.3} ({:>2}) [{:.2}]", score[c][i], rank[c][i], sfb[c][i]));
        }
        println!(" {:>9.1} {:>6}", mean_rank(i), rank_spread(i));
    }
    Ok(())
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = real_main(cli) {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn real_main(cli: Cli) -> Result<()> {
    let fallback = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let store = match &cli.root {
        Some(r) => Store::open(r)?,
        None => Store::discover(&std::env::current_dir()?, Some(&fallback))?,
    };
    let ctx = Ctx { store, json: cli.json };
    match cli.cmd {
        Cmd::Score { layout, inputs } => {
            let l = ctx.load(&inputs)?;
            let layout = ctx.layout(&layout, &l)?;
            let (sc, st) = score_layout(&l, &layout)?;
            let b = breakdown(&sc, &st);
            if ctx.json {
                ctx.print(json!({"schema": SCHEMA, "layout": layout.name, "rows": layout.all_rows(&l.geometry),
                    "geometry": l.geometry.name, "profile": l.profile.name, "corpus": l.corpus.name, "breakdown": b}));
            } else {
                println!(
                    "{}  ({} · {} · {})\n  {}\n",
                    layout.name,
                    l.geometry.name,
                    l.profile.name,
                    l.corpus.name,
                    layout_rows(&layout, &l.geometry)
                );
                print_breakdown(&b);
            }
        }
        Cmd::Compare { layouts, corpora, inputs } if !corpora.is_empty() => robustness(&ctx, layouts, &corpora, &inputs)?,
        Cmd::Compare { layouts, inputs, .. } => {
            let l = ctx.load(&inputs)?;
            let names = if layouts.is_empty() { ctx.store.list("layouts") } else { layouts };
            let mut rows = Vec::new();
            for n in &names {
                let layout = ctx.layout(n, &l)?;
                let (sc, st) = score_layout(&l, &layout)?;
                rows.push((layout, breakdown(&sc, &st)));
            }
            rows.sort_by(|a, b| a.1.score.total_cmp(&b.1.score));
            if ctx.json {
                let v: Vec<_> =
                    rows.iter().map(|(lay, b)| json!({"layout": lay.name, "rows": lay.all_rows(&l.geometry), "breakdown": b})).collect();
                ctx.print(json!({"schema": SCHEMA, "geometry": l.geometry.name, "profile": l.profile.name, "corpus": l.corpus.name, "layouts": v}));
            } else {
                let cols = ["B01", "S01", "B04", "B05", "M01", "T03", "T06", "M04"];
                print!("{:<4} {:<14} {:>8}", "#", "layout", "score");
                for c in cols {
                    let d = gecko_core::metrics::def(c).unwrap();
                    print!(" {:>8}", d.name.chars().take(8).collect::<String>());
                }
                println!();
                for (i, (lay, b)) in rows.iter().enumerate() {
                    print!("{:<4} {:<14} {:>8.3}", i + 1, lay.name, b.score);
                    for c in cols {
                        let m = b.metrics.iter().find(|m| m.id == c).unwrap();
                        print!(" {:>8}", fmt_val(m.value));
                    }
                    println!();
                }
            }
        }
        Cmd::Analyze { layout, top, inputs } => {
            let l = ctx.load(&inputs)?;
            let layout = ctx.layout(&layout, &l)?;
            let (sc, st) = score_layout(&l, &layout)?;
            let a = analyze(&sc, &st, top);
            if ctx.json {
                ctx.print(json!({"schema": SCHEMA, "layout": layout.name, "rows": layout.all_rows(&l.geometry), "analysis": a}));
            } else {
                println!("{}\n  {}\n", layout.name, layout_rows(&layout, &l.geometry));
                print_breakdown(&a.breakdown);
                println!("\nfinger load %  (target)");
                for i in 0..10 {
                    if a.finger_load[i] > 0.0 || a.finger_target[i] > 0.0 {
                        println!(
                            "  {:<3} {:>5.1}  ({:>4.1})  {}",
                            a.fingers[i],
                            a.finger_load[i],
                            a.finger_target[i],
                            "#".repeat((a.finger_load[i] * 2.0) as usize)
                        );
                    }
                }
                for (title, v) in [("SFB", &a.worst_sfb), ("SFS", &a.worst_sfs), ("LSB", &a.worst_lsb), ("scissors", &a.worst_scissors)] {
                    let s: Vec<String> = v.iter().map(|n| format!("{} {:.2}", n.ngram, n.freq)).collect();
                    println!("\nworst {title}: {}", s.join(" · "));
                }
            }
        }
        Cmd::Optimize { algo, time, evals, seed, threads, pin, from, top, sa_steps, ga_pop, inputs } => {
            let l = ctx.load(&inputs)?;
            let algo = Algo::parse(&algo).with_context(|| format!("unknown algo '{algo}' (hc, sa, ga, bls, explore)"))?;
            let start = ctx.layout(from.as_deref().unwrap_or(&ctx.store.defaults.reference), &l)?;
            let mut params = Params {
                algo,
                seed,
                threads: threads.unwrap_or_else(|| std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)),
                time_ms: if evals.is_some() { None } else { Some(parse_duration(time.as_deref().unwrap_or("10s"))?) },
                evals,
                top,
                ..Default::default()
            };
            if let Some(s) = sa_steps {
                params.sa.steps = s;
            }
            if let Some(p) = ga_pop {
                params.ga.pop_size = p;
            }
            let spec = RunSpec { geometry: &l.geometry, corpus: &l.corpus, profile: &l.profile, start: &start, pins: &pin, params };
            let prep = run::prepare(&ctx.store.runs, &spec)?;
            eprintln!("run {}  ({} · {} threads · {})", prep.config.id, algo.name(), spec.params.threads, l.profile.name);
            let control = Control::default();
            let quiet = ctx.json;
            let res = run::execute(&prep, &spec, &control, |p| {
                if !quiet {
                    eprint!("\r  {:>6.1}s  {:>12} evals  best {:.4}  {}", p.elapsed_ms as f64 / 1000.0, p.evals, p.best, p.best_chars);
                }
            })?;
            if !quiet {
                eprintln!();
            }
            if ctx.json {
                ctx.print(json!({"schema": SCHEMA, "run": res}));
            } else {
                println!(
                    "run {}  {} evals in {:.1}s ({:.2} M evals/s)",
                    res.id,
                    res.evals,
                    res.elapsed_ms as f64 / 1000.0,
                    res.evals_per_sec / 1e6
                );
                for r in res.best.iter().take(5) {
                    println!("#{:<2} {:>8.3}  {}", r.rank, r.score, r.rows.join(" "));
                }
                if let Some(b) = res.best.first() {
                    let layout = Layout::from_flat("best", &l.geometry, b.chars.chars().collect())?;
                    let (sc, st) = score_layout(&l, &layout)?;
                    println!("\nbest\n  {}\n", layout_rows(&layout, &l.geometry));
                    print_breakdown(&breakdown(&sc, &st));
                }
            }
        }
        Cmd::Explore { time, evals, seed, threads, pin, from, samples, max_optima, inputs } => explore_cmd::explore(
            &ctx,
            explore_cmd::ExploreArgs { time, evals, seed, threads, pin, from, samples, max_optima, inputs },
        )?,
        Cmd::Landscape { samples, walks, walk_len, seed, from, inputs } => {
            explore_cmd::landscape(&ctx, &inputs, from.as_deref(), samples, walks, walk_len, seed)?
        }
        Cmd::Atlas { run, top, cut, pool, inputs } => explore_cmd::atlas_cmd(&ctx, &run, &inputs, top, cut, pool)?,
        Cmd::Bench { algos, seeds, time, threads, inputs } => {
            let l = ctx.load(&inputs)?;
            let start = ctx.layout(&ctx.store.defaults.reference, &l)?;
            let time_ms = parse_duration(&time)?;
            let threads = threads.unwrap_or_else(|| std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1));
            let sc = Scorer::new(&l.geometry, &l.corpus, &l.profile, &start)?;
            let mut out = Vec::new();
            for a in algos.split(',') {
                let algo = Algo::parse(a.trim()).with_context(|| format!("unknown algo '{a}'"))?;
                let mut scores = Vec::new();
                let mut eps = Vec::new();
                for seed in 1..=seeds {
                    let params = Params { algo, seed, threads, time_ms: Some(time_ms), evals: None, top: 1, ..Default::default() };
                    let c = gecko_core::optimize::Ctx::new(&sc, (0..sc.n).collect(), sc.identity());
                    let o = gecko_core::optimize::run(&c, &params, std::time::Duration::from_secs(3600), &Control::default(), |_| {});
                    scores.push(o.top.first().map(|s| s.score).unwrap_or(f64::NAN));
                    eps.push(o.evals_per_sec);
                    if !ctx.json {
                        eprintln!("  {} seed {seed}: {:.4}", algo.name(), scores.last().unwrap());
                    }
                }
                let n = scores.len() as f64;
                let mean = scores.iter().sum::<f64>() / n;
                let best = scores.iter().cloned().fold(f64::INFINITY, f64::min);
                let sd = (scores.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / n).sqrt();
                out.push(json!({"algo": algo.name(), "mean": mean, "best": best, "stdev": sd, "evals_per_sec": eps.iter().sum::<f64>() / n, "scores": scores}));
            }
            if ctx.json {
                ctx.print(json!({"schema": SCHEMA, "time_ms": time_ms, "threads": threads, "profile": l.profile.name, "results": out}));
            } else {
                println!("{:<5} {:>9} {:>9} {:>8} {:>10}", "algo", "mean", "best", "stdev", "Mevals/s");
                for r in &out {
                    println!(
                        "{:<5} {:>9.4} {:>9.4} {:>8.4} {:>10.2}",
                        r["algo"].as_str().unwrap(),
                        r["mean"].as_f64().unwrap(),
                        r["best"].as_f64().unwrap(),
                        r["stdev"].as_f64().unwrap(),
                        r["evals_per_sec"].as_f64().unwrap() / 1e6
                    );
                }
            }
        }
        Cmd::Perf { layout, inputs } => {
            let l = ctx.load(&inputs)?;
            let layout = ctx.layout(&layout, &l)?;
            let (sc, mut st) = score_layout(&l, &layout)?;
            let mut rng = gecko_core::rng::Rng::new(1);
            let t = std::time::Instant::now();
            let mut k = 0u64;
            let mut sink = 0.0;
            while t.elapsed().as_millis() < 500 {
                sink += sc.full(&st);
                k += 1;
            }
            let full = k as f64 / t.elapsed().as_secs_f64();
            let t = std::time::Instant::now();
            let mut k = 0u64;
            while t.elapsed().as_millis() < 1000 {
                for _ in 0..1000 {
                    let a = rng.below(sc.n);
                    let b = (a + 1 + rng.below(sc.n - 1)) % sc.n;
                    let d = sc.delta(&st, a, b);
                    if d < 0.0 {
                        sc.apply(&mut st, a, b, d);
                    }
                }
                k += 1000;
            }
            let delta = k as f64 / t.elapsed().as_secs_f64();
            std::hint::black_box(sink);
            if ctx.json {
                ctx.print(json!({"schema": SCHEMA, "full_per_sec": full, "delta_per_sec": delta}));
            } else {
                println!("full evaluation  {:>12.0} /s  ({:.2} µs)", full, 1e6 / full);
                println!("swap delta       {:>12.0} /s  ({:.3} µs)", delta, 1e6 / delta);
            }
        }
        Cmd::Corpus { cmd: CorpusCmd::Build { sources, out, charset, ext } } => {
            if sources.is_empty() {
                bail!("give at least one text file or directory");
            }
            let mut c = Corpus::new(&out, gecko_core::corpus::charset_named(&charset));
            let exts: Vec<&str> = ext.split(',').map(str::trim).collect();
            let mut files = Vec::new();
            for s in &sources {
                if s.is_dir() {
                    let mut v = Vec::new();
                    walk(s, &exts, &mut v)?;
                    v.sort();
                    files.extend(v);
                } else {
                    files.push(s.clone());
                }
            }
            for f in &files {
                let bytes = std::fs::read(f).with_context(|| format!("reading {}", f.display()))?;
                c.add_text(&String::from_utf8_lossy(&bytes));
            }
            write_corpus(&ctx, &c, files.len())?;
        }
        Cmd::Corpus { cmd: CorpusCmd::Mix { parts, out } } => {
            let loaded = parts
                .iter()
                .map(|p| {
                    let (name, w) = p.split_once(':').unwrap_or((p, "1"));
                    Ok((ctx.store.corpus(name)?, w.parse::<f64>().with_context(|| format!("bad weight in '{p}'"))?))
                })
                .collect::<Result<Vec<_>>>()?;
            let refs: Vec<(&Corpus, f64)> = loaded.iter().map(|(c, w)| (c, *w)).collect();
            write_corpus(&ctx, &Corpus::mix(&out, &refs)?, parts.len())?;
        }
        Cmd::Corpus { cmd: CorpusCmd::Import { dir, out, charset } } => {
            let mut c = Corpus::new(&out, gecko_core::corpus::charset_named(&charset));
            let words = dir.join("words.json");
            let files = if words.is_file() {
                let mut e = c.word_edges();
                for (w, n) in read_counts(&words)? {
                    c.add_word(&w, n, &mut e);
                }
                c.add_word_boundaries(&e);
                1
            } else {
                let names = ["monograms.json", "bigrams.json", "trigrams.json"];
                for f in names {
                    for (g, n) in read_counts(&dir.join(f))? {
                        c.add_ngram(&g, n);
                    }
                }
                names.len()
            };
            write_corpus(&ctx, &c, files)?;
        }
        Cmd::Profile { cmd: ProfileCmd::Normalize { layouts, out, inputs } } => {
            let l = ctx.load(&inputs)?;
            let (refs, basis) = reference_layouts(&ctx, &l, layouts)?;
            let scales = gecko_core::calibrate::spread_scales(&l.geometry, &l.corpus, &l.profile, &refs)?;
            let mut p = l.profile.clone();
            for (id, w) in p.weights.iter_mut() {
                let eff = l.profile.effective_weight(id);
                *w = match scales.get(id) {
                    Some(s) => round4(eff * s),
                    None => eff,
                };
            }
            p.scale = scales.into_iter().filter(|(id, _)| p.weights.contains_key(id)).map(|(k, v)| (k, round4(v))).collect();
            p.scale_basis = basis;
            if ctx.json {
                ctx.print(json!({"schema": SCHEMA, "profile": p}));
            }
            emit_profile(&ctx, &p, out.as_deref())?;
        }
        Cmd::Profile { cmd: ProfileCmd::Fit { target, layouts, out, seed, inputs } } => {
            let l = ctx.load(&inputs)?;
            let (refs, basis) = reference_layouts(&ctx, &l, layouts)?;
            let fit = gecko_core::calibrate::fit_to(&l.geometry, &l.corpus, &l.profile, &refs, &target, seed)?;
            let mut p = l.profile.clone();
            p.name = out.clone().unwrap_or_else(|| format!("fit-{}", target.to_lowercase()));
            p.description = format!("Weights fitted to predict {target} (R² = {:.3}, {} samples, seed {seed}).", fit.r2, fit.samples);
            p.weights = fit.weights.iter().map(|(k, v)| (k.clone(), round4(*v))).collect();
            p.scale = fit.scales.iter().filter(|(k, _)| p.weights.contains_key(*k)).map(|(k, v)| (k.clone(), round4(*v))).collect();
            p.scale_basis = basis;
            if ctx.json {
                ctx.print(json!({"schema": SCHEMA, "fit": fit, "profile": p}));
            } else {
                eprintln!("{target}: R² = {:.3} over {} sampled layouts", fit.r2, fit.samples);
            }
            emit_profile(&ctx, &p, out.as_deref())?;
        }
        Cmd::List { what } => {
            let items: serde_json::Value = match what.as_str() {
                "layouts" | "geometries" | "profiles" | "corpora" => json!(ctx.store.list(&what)),
                "runs" => json!(run::list(&ctx.store.runs)),
                "metrics" => json!(gecko_core::metrics::METRICS),
                _ => bail!("list what? layouts | geometries | profiles | corpora | runs | metrics"),
            };
            if ctx.json {
                ctx.print(json!({"schema": SCHEMA, "kind": what, "items": items}));
            } else if what == "runs" {
                for r in run::list(&ctx.store.runs) {
                    println!(
                        "{:<34} {:<4} {:<9} {:>9} {}",
                        r.id,
                        r.algo,
                        r.profile,
                        r.best_score.map(|s| format!("{s:.3}")).unwrap_or("-".into()),
                        r.best.unwrap_or_default()
                    );
                }
            } else if what == "metrics" {
                for m in gecko_core::metrics::METRICS {
                    println!("{:<4} {:<18} {:?}  {}", m.id, m.name, m.dir, m.desc);
                }
            } else {
                for i in items.as_array().unwrap() {
                    println!("{}", i.as_str().unwrap());
                }
            }
        }
        Cmd::Serve { port, web } => {
            let web = web.unwrap_or_else(|| ctx.store.root.join("web/dist"));
            serve::serve(ctx.store, port, web)?;
        }
    }
    Ok(())
}

/// Round to 4 significant digits so generated profiles stay readable.
fn round4(v: f32) -> f32 {
    if v == 0.0 || !v.is_finite() {
        return v;
    }
    let m = 10f32.powi(3 - v.abs().log10().floor() as i32);
    (v * m).round() / m
}
