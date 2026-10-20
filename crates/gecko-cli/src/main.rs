//! `gecko` — the agent-first CLI and `gecko serve`. See `context/cli.md`.

mod serve;

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use gecko_core::metrics::Dir;
use gecko_core::optimize::{Algo, Params};
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
    /// Count n-grams of text files (or directories of .txt files) into data/corpora/<out>.gkc.
    Build {
        sources: Vec<PathBuf>,
        #[arg(long, default_value = "english")]
        out: String,
        #[arg(long, default_value = gecko_core::corpus::DEFAULT_CHARSET)]
        charset: String,
    },
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
        let layout = self.store.layout(spec, &l.geometry.name)?;
        layout.check_geometry(&l.geometry)?;
        Ok(layout)
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
    layout.rows(g).iter().map(|r| r.chars().map(|c| c.to_string()).collect::<Vec<_>>().join(" ")).collect::<Vec<_>>().join("\n  ")
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
                ctx.print(json!({"schema": SCHEMA, "layout": layout.name, "rows": layout.rows(&l.geometry),
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
        Cmd::Compare { layouts, inputs } => {
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
                    rows.iter().map(|(lay, b)| json!({"layout": lay.name, "rows": lay.rows(&l.geometry), "breakdown": b})).collect();
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
                ctx.print(json!({"schema": SCHEMA, "layout": layout.name, "rows": layout.rows(&l.geometry), "analysis": a}));
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
            let algo = Algo::parse(&algo).with_context(|| format!("unknown algo '{algo}' (hc, sa, ga)"))?;
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
            let cancel = AtomicBool::new(false);
            let quiet = ctx.json;
            let res = run::execute(&prep, &spec, &cancel, |p| {
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
                    let layout = Layout::from_chars("best", &l.geometry.name, "", b.chars.chars().collect())?;
                    let (sc, st) = score_layout(&l, &layout)?;
                    println!("\nbest\n  {}\n", layout_rows(&layout, &l.geometry));
                    print_breakdown(&breakdown(&sc, &st));
                }
            }
        }
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
                    let c = gecko_core::optimize::Ctx { scorer: &sc, movable: (0..sc.n).collect(), start: sc.identity() };
                    let o = gecko_core::optimize::run(&c, &params, std::time::Duration::from_secs(3600), &AtomicBool::new(false), |_| {});
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
        Cmd::Corpus { cmd: CorpusCmd::Build { sources, out, charset } } => {
            if sources.is_empty() {
                bail!("give at least one text file or directory");
            }
            let mut c = Corpus::new(&out, &charset);
            let mut files = Vec::new();
            for s in &sources {
                if s.is_dir() {
                    let mut v: Vec<_> =
                        std::fs::read_dir(s)?.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "txt")).collect();
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
            let path = ctx.store.corpus_path(&out);
            std::fs::create_dir_all(path.parent().unwrap())?;
            std::fs::write(&path, c.to_bytes())?;
            let v = json!({"schema": SCHEMA, "corpus": out, "path": path, "files": files.len(), "chars": c.total_chars(), "hash": gecko_core::hash::hex(c.hash())});
            if ctx.json {
                ctx.print(v);
            } else {
                println!("wrote {} ({} files, {} chars)", path.display(), files.len(), c.total_chars());
            }
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
