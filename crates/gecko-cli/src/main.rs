//! `gecko` — the agent-first CLI and `gecko serve`. See `context/cli.md`.

mod explore_cmd;
mod serve;
mod web_assets;

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use gecko_core::corpus::WordList;
use gecko_core::metrics::Dir;
use gecko_core::optimize::{Algo, Control, Params};
use gecko_core::report::{analyze, breakdown, worst_words, Breakdown};
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
        /// Also list the worst real words per metric (frequency × cost); needs a word-import
        /// corpus (`<corpus>.words.tsv`), e.g. `-c shai`.
        #[arg(long)]
        words: bool,
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
        /// GA/NSGA-II: population size.
        #[arg(long)]
        ga_pop: Option<usize>,
        /// Memetic: layouts per island (default 24).
        #[arg(long)]
        memetic_pop: Option<usize>,
        /// Memetic refinement: hc or tabu.
        #[arg(long, value_parser = ["hc", "tabu"])]
        local_search: Option<String>,
        /// Memetic: maximum improving HC moves or tabu iterations per child.
        #[arg(long)]
        local_steps: Option<usize>,
        /// Memetic: number of islands (default one per worker).
        #[arg(long)]
        islands: Option<usize>,
        /// Memetic: generations between ring migrations (0 disables).
        #[arg(long)]
        migration_interval: Option<u64>,
        /// Memetic: best individuals sent per migration.
        #[arg(long)]
        migrants: Option<usize>,
        /// Pareto objectives, e.g. B01,T03,M01 (NSGA-II or random-weight SA).
        #[arg(long, value_delimiter = ',')]
        objectives: Option<Vec<String>>,
        /// NSGA-II generations (0 = use the time/evaluation budget).
        #[arg(long)]
        nsga_generations: Option<usize>,
        /// Maximum diverse Pareto archive size.
        #[arg(long)]
        front_size: Option<usize>,
        /// Extra descent neighbourhoods: comma list of columns, cycle3, depth2, all (or swap).
        #[arg(long)]
        moves: Option<String>,
        /// Depth-2 moves: first swaps tried per optimum (0 = all).
        #[arg(long)]
        depth2_width: Option<usize>,
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
    /// MAP-Elites quality-diversity map: a grid over behaviour axes (roll/alternation ratio,
    /// index load, hand balance, or any metric id), best layout per cell. Writes a qd run.
    Qd {
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
        /// 1–3 axes, each name or name:lo:hi (roll_ratio, index_load, pinky_load, hand_balance,
        /// or a metric id). Without a range, the seed descents set it.
        #[arg(long, value_delimiter = ',', default_value = "roll_ratio,index_load")]
        axes: Vec<String>,
        /// Bins per axis.
        #[arg(long, default_value_t = 16)]
        bins: usize,
        /// Random descents that seed the grid.
        #[arg(long)]
        seeds: Option<usize>,
        /// Improving moves after each mutation (0 = descend to a local optimum).
        #[arg(long)]
        descent: Option<usize>,
        /// Print an existing qd run instead of searching.
        #[arg(long)]
        show: Option<String>,
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
        /// Neighbourhood variants, separated by '/', each benched with every algo, e.g.
        /// `swap/cycle3/columns,depth2`.
        #[arg(long, default_value = "swap")]
        moves: String,
        /// Depth-2 moves: first swaps tried per optimum (0 = all).
        #[arg(long)]
        depth2_width: Option<usize>,
        #[command(flatten)]
        inputs: Inputs,
    },
    /// Re-run a saved --evals run from its config and check it reproduces best.json.
    Verify {
        /// run:<id> (or just <id>).
        run: String,
    },
    /// Single-thread scorer speed: full evaluations/s and swap deltas/s.
    Perf {
        #[arg(long, default_value = "qwerty")]
        layout: String,
        /// Append {date, commit, full/s, delta/s} to bench/history.jsonl.
        #[arg(long)]
        record: bool,
        #[command(flatten)]
        inputs: Inputs,
    },
    /// Search the rules of an adaptive (magic) key on a fixed layout: greedily add the
    /// `previous → output` rule that lowers the score most, then drop rules that stopped paying.
    Magic {
        /// Layout with a `[[magic]]` key (or use --on).
        layout: String,
        /// Which adaptive key (1 = the first `[[magic]]`).
        #[arg(long, default_value_t = 1)]
        key: usize,
        #[arg(long, default_value_t = 32)]
        max_rules: usize,
        /// Accept a rule only when it lowers the score by more than this.
        #[arg(long, default_value_t = 0.01)]
        min_gain: f64,
        /// Extend the key's existing rules instead of starting from none.
        #[arg(long)]
        keep: bool,
        /// Put a new magic key (`★`) on this symbol's key; the symbol is dropped.
        #[arg(long)]
        on: Option<String>,
        /// Write the result to data/layouts/<out>.toml (or to a path ending in .toml).
        #[arg(long)]
        out: Option<String>,
        #[command(flatten)]
        inputs: Inputs,
    },
    /// Corpus tools.
    Corpus {
        #[command(subcommand)]
        cmd: CorpusCmd,
    },
    /// Geometry tools.
    Geometry {
        #[command(subcommand)]
        cmd: GeometryCmd,
    },
    /// Import layouts from other analyzers, export them back.
    Layouts {
        #[command(subcommand)]
        cmd: LayoutsCmd,
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
        /// Override the web app directory (default: embedded UI, or <root>/web/dist without embed-web).
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
        /// Symbols to count, or `default` / `code` (adds digits, all ASCII punctuation, Enter) /
        /// `nav` (code plus arrows `←↓↑→`).
        #[arg(long, default_value = "default")]
        charset: String,
        /// File extensions to read from directories, comma-separated (e.g. `rs,py,ts`).
        #[arg(long, default_value = "txt")]
        ext: String,
        /// Fold diacritics to ASCII letters first (`é` → `e`, `œ` → `oe`), for non-English text.
        #[arg(long)]
        fold: bool,
    },
    /// Weighted mix of corpora with the same charset (each normalized first), e.g.
    /// `gecko corpus mix code-rs:1 code-py:1 --out code`.
    Mix {
        /// `<corpus>[:weight]` (weight 1 when omitted).
        parts: Vec<String>,
        #[arg(long)]
        out: String,
    },
    /// Add arrow keys `←↓↑→` to a corpus from a cursor-movement model (text has none):
    /// `gecko corpus nav code --share 0.03 --out code-nav`.
    Nav {
        base: String,
        /// Arrow share of all keystrokes.
        #[arg(long, default_value_t = 0.03)]
        share: f64,
        #[arg(long, default_value_t = 1)]
        seed: u64,
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
        /// Fold diacritics to ASCII letters first (`é` → `e`, `œ` → `oe`), for non-English text.
        #[arg(long)]
        fold: bool,
    },
}

#[derive(Subcommand)]
enum GeometryCmd {
    /// Import a keyboard-layout-editor.com JSON file (or its raw data) as
    /// data/geometries/<out>.toml. Fingers, rows, home keys and efforts are guesses to review.
    ImportKle {
        file: PathBuf,
        #[arg(long)]
        out: String,
        /// Overwrite an existing geometry.
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum LayoutsCmd {
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
    /// Fit non-negative spread-unit weights to a judge metric (e.g. E05), or use
    /// --target likes for pairwise cmini ranking with 20% of layouts held out.
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

/// Short commit hash of `root`, with `-dirty` if the work tree has changes; "unknown" without git.
fn git_commit(root: &std::path::Path) -> String {
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    match git(&["rev-parse", "--short", "HEAD"]) {
        Some(c) if git(&["status", "--porcelain"]).is_some_and(|s| !s.is_empty()) => format!("{c}-dirty"),
        Some(c) => c,
        None => "unknown".into(),
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
    let rows =
        |rs: &[String]| rs.iter().map(|r| r.chars().map(|c| c.to_string()).collect::<Vec<_>>().join(" ")).collect::<Vec<_>>().join("\n  ");
    let mut out = rows(&layout.rows(g));
    let modes = layout.holders().unwrap_or_default();
    for (li, (ly, rs)) in layout.layers.iter().zip(layout.layer_rows(g)).enumerate() {
        let mode = modes.get(li).filter(|h| h.1 != gecko_core::LayerMode::Hold).map_or(String::new(), |h| format!(", {}", h.1.name()));
        out += &format!("\n  [{}{mode}]\n  {}", ly.name, rows(&rs));
    }
    for m in &layout.magic {
        out += &format!("\n  {} {}{}", m.key, gecko_core::magic::format_rules(&m.rules), if m.repeat { " (else repeat)" } else { "" });
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
    let repo_root = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    // Portable release bundles keep gecko.toml/data beside the executable. Current-directory
    // discovery still wins, so launching in a project retains its inputs and runs.
    let executable_root =
        std::env::current_exe().ok().and_then(|p| p.parent().map(std::path::Path::to_path_buf)).filter(|p| p.join("gecko.toml").is_file());
    let fallback = executable_root.as_deref().unwrap_or(&repo_root);
    let store = match &cli.root {
        Some(r) => Store::open(r)?,
        None => Store::discover(&std::env::current_dir()?, Some(fallback))?,
    };
    let ctx = Ctx { store, json: cli.json };
    match cli.cmd {
        Cmd::Score { layout, inputs } => {
            let l = ctx.load(&inputs)?;
            let layout = ctx.layout(&layout, &l)?;
            let (sc, st) = score_layout(&l, &layout)?;
            let b = breakdown(&sc, &st);
            if ctx.json {
                ctx.print(json!({"schema": SCHEMA, "layout": layout.name, "rows": layout.all_rows(&l.geometry), "magic": layout.magic,
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
        Cmd::Analyze { layout, top, words, inputs } => {
            let l = ctx.load(&inputs)?;
            let layout = ctx.layout(&layout, &l)?;
            let (sc, st) = score_layout(&l, &layout)?;
            let a = analyze(&sc, &st, top);
            let ww = if words { Some(worst_words(&sc, &st, &ctx.store.words(&l.corpus.name)?, top)) } else { None };
            if ctx.json {
                let mut v = json!({"schema": SCHEMA, "layout": layout.name, "rows": layout.all_rows(&l.geometry), "analysis": a});
                if let Some(ww) = &ww {
                    v["worst_words"] = json!(ww);
                }
                ctx.print(v);
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
                if let Some(ww) = &ww {
                    println!("\nworst words  (word share%  — % of the metric's word cost)");
                    for m in ww {
                        let s: Vec<String> = m.words.iter().map(|w| format!("{} {:.1}", w.word, w.share)).collect();
                        println!("  {:<16} {}", m.name, s.join(" · "));
                    }
                }
            }
        }
        Cmd::Optimize {
            algo,
            time,
            evals,
            seed,
            threads,
            pin,
            from,
            top,
            sa_steps,
            ga_pop,
            memetic_pop,
            local_search,
            local_steps,
            islands,
            migration_interval,
            migrants,
            objectives,
            nsga_generations,
            front_size,
            moves,
            depth2_width,
            inputs,
        } => {
            let l = ctx.load(&inputs)?;
            let algo =
                Algo::parse(&algo).with_context(|| format!("unknown algo '{algo}' (hc, sa, ga, memetic, bls, explore, tabu, qd, nsga2, rwsa)"))?;
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
                params.nsga2.pop_size = p;
            }
            if let Some(pop) = memetic_pop {
                params.memetic.ga.pop_size = pop;
            }
            if let Some(local) = local_search {
                params.memetic.local_search = if local == "tabu" {
                    gecko_core::optimize::memetic::LocalSearch::Tabu
                } else {
                    gecko_core::optimize::memetic::LocalSearch::Hc
                };
            }
            if let Some(steps) = local_steps {
                params.memetic.local_steps = steps;
            }
            if let Some(n) = islands {
                params.memetic.islands = n;
            }
            if let Some(n) = migration_interval {
                params.memetic.migration_interval = n;
            }
            if let Some(n) = migrants {
                params.memetic.migrants = n;
            }
            if let Some(ids) = objectives { params.nsga2.objectives = ids; }
            if let Some(generations) = nsga_generations { params.nsga2.generations = generations; }
            if let Some(size) = front_size { params.nsga2.archive_size = size; }
            if let Some(m) = moves {
                params.moves = parse_moves(&m)?;
            }
            if let Some(w) = depth2_width {
                params.moves.depth2_width = w;
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
                if !res.front.is_empty() { println!("Pareto front: {} layouts over {}", res.front.len(), res.objectives.iter().map(|o| o.id.as_str()).collect::<Vec<_>>().join(", ")); }
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
        Cmd::Explore { time, evals, seed, threads, pin, from, samples, max_optima, inputs } => {
            explore_cmd::explore(&ctx, explore_cmd::ExploreArgs { time, evals, seed, threads, pin, from, samples, max_optima, inputs })?
        }
        Cmd::Qd { time, evals, seed, threads, pin, from, axes, bins, seeds, descent, show, inputs } => {
            explore_cmd::qd(&ctx, explore_cmd::QdArgs { time, evals, seed, threads, pin, from, axes, bins, seeds, descent, show, inputs })?
        }
        Cmd::Landscape { samples, walks, walk_len, seed, from, inputs } => {
            explore_cmd::landscape(&ctx, &inputs, from.as_deref(), samples, walks, walk_len, seed)?
        }
        Cmd::Atlas { run, top, cut, pool, inputs } => explore_cmd::atlas_cmd(&ctx, &run, &inputs, top, cut, pool)?,
        Cmd::Bench { algos, seeds, time, threads, moves, depth2_width, inputs } => {
            let l = ctx.load(&inputs)?;
            let start = ctx.layout(&ctx.store.defaults.reference, &l)?;
            let time_ms = parse_duration(&time)?;
            let threads = threads.unwrap_or_else(|| std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1));
            let sc = Scorer::new(&l.geometry, &l.corpus, &l.profile, &start)?;
            let mut variants = moves.split('/').map(parse_moves).collect::<Result<Vec<_>>>()?;
            if let Some(w) = depth2_width {
                variants.iter_mut().for_each(|m| m.depth2_width = w);
            }
            let mut out = Vec::new();
            for a in algos.split(',') {
                let algo = Algo::parse(a.trim()).with_context(|| format!("unknown algo '{a}'"))?;
                for mv in &variants {
                    let mut scores = Vec::new();
                    let mut eps = Vec::new();
                    for seed in 1..=seeds {
                        let params = Params {
                            algo,
                            seed,
                            threads,
                            time_ms: Some(time_ms),
                            evals: None,
                            top: 1,
                            moves: mv.clone(),
                            ..Default::default()
                        };
                        let c = gecko_core::optimize::Ctx::new(&sc, (0..sc.n).collect(), sc.identity());
                        let o = gecko_core::optimize::run(&c, &params, std::time::Duration::from_secs(3600), &Control::default(), |_| {});
                        scores.push(o.top.first().map(|s| s.score).unwrap_or(f64::NAN));
                        eps.push(o.evals_per_sec);
                        if !ctx.json {
                            eprintln!("  {} [{}] seed {seed}: {:.4}", algo.name(), mv.name(), scores.last().unwrap());
                        }
                    }
                    let n = scores.len() as f64;
                    let mean = scores.iter().sum::<f64>() / n;
                    let best = scores.iter().cloned().fold(f64::INFINITY, f64::min);
                    let sd = (scores.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / n).sqrt();
                    out.push(json!({"algo": algo.name(), "moves": mv.name(), "mean": mean, "best": best, "stdev": sd, "evals_per_sec": eps.iter().sum::<f64>() / n, "scores": scores}));
                }
            }
            if ctx.json {
                ctx.print(json!({"schema": SCHEMA, "time_ms": time_ms, "threads": threads, "profile": l.profile.name, "results": out}));
            } else {
                println!("{:<5} {:<22} {:>9} {:>9} {:>8} {:>10}", "algo", "moves", "mean", "best", "stdev", "Mevals/s");
                for r in &out {
                    println!(
                        "{:<5} {:<22} {:>9.4} {:>9.4} {:>8.4} {:>10.2}",
                        r["algo"].as_str().unwrap(),
                        r["moves"].as_str().unwrap(),
                        r["mean"].as_f64().unwrap(),
                        r["best"].as_f64().unwrap(),
                        r["stdev"].as_f64().unwrap(),
                        r["evals_per_sec"].as_f64().unwrap() / 1e6
                    );
                }
            }
        }
        Cmd::Verify { run } => {
            let id = run.strip_prefix("run:").unwrap_or(&run);
            let dir = ctx.store.runs.join(id);
            eprintln!("verify {id}  (re-running)");
            let v = run::verify(&ctx.store, &dir)?;
            if ctx.json {
                ctx.print(json!(v));
            } else {
                for c in &v.checks {
                    let mark = if c.ok { "ok  " } else { "FAIL" };
                    if c.ok {
                        println!("{mark} {:<9} {}", c.name, c.actual);
                    } else {
                        println!("{mark} {:<9} expected {}\n               got {}", c.name, c.expected, c.actual);
                    }
                }
                println!("{}", if v.ok { "reproduced" } else { "NOT reproduced" });
            }
            if !v.ok {
                bail!("run {id} did not reproduce");
            }
        }
        Cmd::Perf { layout: spec, record, inputs } => {
            let l = ctx.load(&inputs)?;
            let layout = ctx.layout(&spec, &l)?;
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
            let recorded = if record {
                let entry = json!({
                    "date": run::now_stamp().1,
                    "commit": git_commit(&ctx.store.root),
                    "layout": spec,
                    "geometry": l.geometry.name,
                    "corpus": l.corpus.name,
                    "profile": l.profile.name,
                    "full_per_sec": full.round() as u64,
                    "delta_per_sec": delta.round() as u64,
                });
                let path = ctx.store.root.join("bench/history.jsonl");
                std::fs::create_dir_all(path.parent().unwrap())?;
                let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&path)?;
                use std::io::Write;
                writeln!(f, "{}", serde_json::to_string(&entry)?)?;
                Some(path)
            } else {
                None
            };
            if ctx.json {
                ctx.print(json!({"schema": SCHEMA, "full_per_sec": full, "delta_per_sec": delta,
                    "recorded": recorded.as_ref().map(|p| p.display().to_string())}));
            } else {
                println!("full evaluation  {:>12.0} /s  ({:.2} µs)", full, 1e6 / full);
                println!("swap delta       {:>12.0} /s  ({:.3} µs)", delta, 1e6 / delta);
                if let Some(p) = recorded {
                    println!("recorded in {}", p.display());
                }
            }
        }
        Cmd::Magic { layout, key, max_rules, min_gain, keep, on, out, inputs } => {
            let l = ctx.load(&inputs)?;
            let mut layout = ctx.layout(&layout, &l)?;
            if let Some(sym) = on {
                let sym = gecko_core::layout::parse_symbols(&sym)?;
                let [c] = sym[..] else { bail!("--on takes one symbol") };
                let slot = layout.chars.iter().position(|&x| x == c).with_context(|| format!("'{c}' is not on the base layer"))?;
                layout.chars[slot] = gecko_core::magic::MAGIC;
                let mut magic = layout.magic.clone();
                magic.push(gecko_core::magic::Magic { key: gecko_core::magic::MAGIC, ..Default::default() });
                layout = layout.with_magic(magic)?;
                layout.name = format!("{} + magic", layout.name);
            }
            let opts = gecko_core::magic::SearchOpts { key: key.saturating_sub(1), max_rules, keep, min_gain };
            let t0 = std::time::Instant::now();
            let r = gecko_core::magic::search_rules(&l.geometry, &l.corpus, &l.profile, &layout, &opts)?;
            let ms = t0.elapsed().as_millis() as u64;
            let (sc, st) = score_layout(&l, &r.layout)?;
            let b = breakdown(&sc, &st);
            if let Some(name) = &out {
                let path =
                    if name.ends_with(".toml") { PathBuf::from(name) } else { ctx.store.data.join("layouts").join(format!("{name}.toml")) };
                std::fs::write(&path, r.layout.to_toml(&l.geometry)).with_context(|| format!("writing {}", path.display()))?;
                if !ctx.json {
                    println!("wrote {}", path.display());
                }
            }
            let m = &r.layout.magic[opts.key];
            if ctx.json {
                ctx.print(json!({"schema": SCHEMA, "layout": r.layout.name, "rows": r.layout.all_rows(&l.geometry), "key": m.key,
                    "rules": gecko_core::magic::format_rules(&m.rules), "start_score": r.start_score, "score": r.score,
                    "steps": r.steps, "evals": r.evals, "elapsed_ms": ms, "magic": r.layout.magic, "breakdown": b,
                    "geometry": l.geometry.name, "profile": l.profile.name, "corpus": l.corpus.name}));
            } else {
                println!(
                    "{}  ({} · {} · {})  {} evals in {} ms",
                    r.layout.name, l.geometry.name, l.profile.name, l.corpus.name, r.evals, ms
                );
                for s in &r.steps {
                    println!("  {:<8} {:>9.3}", s.rule, s.score);
                }
                println!("score {:.3} -> {:.3}\n  {}\n", r.start_score, r.score, layout_rows(&r.layout, &l.geometry));
                print_breakdown(&b);
            }
        }
        Cmd::Corpus { cmd: CorpusCmd::Build { sources, out, charset, ext, fold } } => {
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
                let text = String::from_utf8_lossy(&bytes);
                c.add_text(&if fold { gecko_core::corpus::fold_diacritics(&text) } else { text.into_owned() });
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
        Cmd::Corpus { cmd: CorpusCmd::Nav { base, share, seed, out } } => {
            write_corpus(&ctx, &ctx.store.corpus(&base)?.with_nav(&out, share, seed)?, 1)?;
        }
        Cmd::Corpus { cmd: CorpusCmd::Import { dir, out, charset, fold } } => {
            let mut c = Corpus::new(&out, gecko_core::corpus::charset_named(&charset));
            let words = dir.join("words.json");
            let files = if words.is_file() {
                let mut e = c.word_edges();
                let counts: Vec<_> = read_counts(&words)?
                    .into_iter()
                    .map(|(w, n)| (if fold { gecko_core::corpus::fold_diacritics(&w) } else { w }, n))
                    .collect();
                for (w, n) in &counts {
                    c.add_word(w, *n, &mut e);
                }
                c.add_word_boundaries(&e);
                let list = WordList::from_counts(counts, gecko_core::corpus::WORDS_KEEP);
                let path = ctx.store.words_path(&out);
                std::fs::create_dir_all(path.parent().unwrap())?;
                std::fs::write(&path, list.to_tsv())?;
                eprintln!("wrote {} ({} words)", path.display(), list.words.len());
                1
            } else {
                let names = ["monograms.json", "bigrams.json", "trigrams.json"];
                for f in names {
                    for (g, n) in read_counts(&dir.join(f))? {
                        c.add_ngram(&if fold { gecko_core::corpus::fold_diacritics(&g) } else { g }, n);
                    }
                }
                names.len()
            };
            write_corpus(&ctx, &c, files)?;
        }
        Cmd::Geometry { cmd: GeometryCmd::ImportKle { file, out, force } } => {
            let text = std::fs::read_to_string(&file).with_context(|| format!("reading {}", file.display()))?;
            let g = gecko_core::kle::import(&out, &text)?;
            let path = ctx.store.data.join("geometries").join(format!("{out}.toml"));
            if path.exists() && !force {
                bail!("{} exists (use --force to overwrite)", path.display());
            }
            std::fs::create_dir_all(path.parent().unwrap())?;
            std::fs::write(&path, g.to_toml()).with_context(|| format!("writing {}", path.display()))?;
            let fingers: Vec<&str> = g.key.iter().map(|k| k.finger.name()).collect();
            if ctx.json {
                ctx.print(json!({"schema": SCHEMA, "geometry": g.name, "path": path, "keys": g.len(), "rows": g.row_lengths(),
                    "fingers": fingers, "hash": gecko_core::hash::hex(g.hash())}));
            } else {
                println!("wrote {} ({} keys, rows {:?})", path.display(), g.len(), g.row_lengths());
                println!("fingers, rows, home keys and efforts are guesses: review the file");
            }
        }
        Cmd::Layouts { cmd: LayoutsCmd::Import { file, from, out, force, name, geometry } } => {
            use gecko_core::import::{import, Format};
            let text = std::fs::read_to_string(&file).with_context(|| format!("reading {}", file.display()))?;
            let stem = file.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            let mut imp = import(Format::parse(&from)?, &text, &stem)?;
            if let Some(n) = name {
                imp.name = n;
            }
            let lf = imp.to_file(geometry.as_deref());
            // The layout must load on its geometry (fills added on boards with thumb keys).
            let g = ctx.store.geometry(&lf.geometry)?;
            Layout::from_rows(&lf.name, &lf.geometry, &lf.source, &lf.rows)?.fit(&g)?.check_geometry(&g)?;
            for w in &imp.warnings {
                eprintln!("warning: {w}");
            }
            let toml_text = toml::to_string(&lf)?;
            let path = match &out {
                Some(o) => {
                    if o.is_empty() || o.contains(['/', '\\']) || o.starts_with('.') {
                        bail!("--out takes a layout name, not a path: '{o}'");
                    }
                    let p = ctx.store.data.join("layouts").join(format!("{o}.toml"));
                    if p.exists() && !force {
                        bail!("{} exists (use --force to overwrite)", p.display());
                    }
                    std::fs::write(&p, &toml_text).with_context(|| format!("writing {}", p.display()))?;
                    Some(p)
                }
                None => None,
            };
            if ctx.json {
                ctx.print(json!({"schema": SCHEMA, "name": lf.name, "geometry": lf.geometry, "source": lf.source,
                    "rows": lf.rows, "warnings": imp.warnings, "path": path.map(|p| p.display().to_string())}));
            } else if let Some(p) = path {
                println!("wrote {}", p.display());
            } else {
                print!("{toml_text}");
            }
        }
        Cmd::Layouts { cmd: LayoutsCmd::Export { layout, to, inputs } } => {
            use gecko_core::import::{export, Format};
            let f = Format::parse(&to)?;
            let g = ctx.store.geometry(inputs.geometry.as_deref().unwrap_or(&ctx.store.defaults.geometry))?;
            let lay = ctx.store.layout(&layout, &g.name)?.fit(&g)?;
            let text = export(f, &lay, &g)?;
            if ctx.json {
                ctx.print(json!({"schema": SCHEMA, "format": f.name(), "layout": lay.name, "text": text}));
            } else {
                print!("{}", if text.ends_with('\n') { text } else { text + "\n" });
            }
        }
        Cmd::Layouts { cmd: LayoutsCmd::ImportCmini { dir } } => {
            let b = gecko_core::community::import_cmini_dir(&dir)?;
            let path = ctx.store.community_path();
            std::fs::create_dir_all(path.parent().unwrap())?;
            std::fs::write(&path, b.to_json()).with_context(|| format!("writing {}", path.display()))?;
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
            } else {
                println!("wrote {}", path.display());
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
            }
        }
        Cmd::Layouts { cmd: LayoutsCmd::Likes { top, dups, inputs } } => {
            let l = ctx.load(&inputs)?;
            let b = ctx.store.community()?;
            let r = if dups {
                gecko_core::community::likes_report(&l.geometry, &l.corpus, &l.profile, b.layouts.iter())?
            } else {
                gecko_core::community::likes_report(&l.geometry, &l.corpus, &l.profile, b.unique())?
            };
            if ctx.json {
                ctx.print(json!({"schema": SCHEMA, "report": r}));
            } else {
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
                println!(
                    "metric goodness vs likes (ρ): {}",
                    ms.iter().map(|(id, v)| format!("{id} {v:+.2}")).collect::<Vec<_>>().join(", ")
                );
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
            }
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
            if target.eq_ignore_ascii_case("likes") {
                if !layouts.is_empty() {
                    bail!("--target likes uses the community bundle; --layouts is only for judge-metric fits");
                }
                let bundle = ctx.store.community()?;
                let fit = gecko_core::calibrate::fit_preferences(&l.geometry, &l.corpus, &l.profile, &bundle, seed)?;
                let mut p = l.profile.clone();
                p.name = out.clone().unwrap_or_else(|| "fit-likes".into());
                p.description = format!(
                    "Pairwise cmini likes fit, seed {seed}, {} train/{} held-out layouts; held-out accuracy {}. Opt-in popularity model.",
                    fit.training_layouts.len(), fit.held_out_layouts.len(),
                    fit.held_out_accuracy.map_or_else(|| "unavailable".into(), |a| format!("{:.1}%", 100.0 * a))
                );
                p.weights = fit.weights.clone();
                p.scale = fit.scales.clone();
                p.scale_basis = format!("cmini training layouts; seed {seed}; {}", fit.input_hashes["community"]);
                if ctx.json {
                    ctx.print(json!({"schema": SCHEMA, "fit": fit, "profile": p}));
                } else {
                    eprintln!("likes: {} training pairs, {} held-out pairs; accuracy {}", fit.training_pairs, fit.held_out_pairs,
                        fit.held_out_accuracy.map_or_else(|| "unavailable".into(), |a| format!("{:.1}%", 100.0 * a)));
                }
                emit_profile(&ctx, &p, out.as_deref())?;
                return Ok(());
            }
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

fn parse_moves(s: &str) -> Result<gecko_core::optimize::moves::MoveParams> {
    gecko_core::optimize::moves::MoveParams::parse(s)
        .with_context(|| format!("unknown moves '{s}' (comma list of swap, columns, cycle3, depth2, all)"))
}
