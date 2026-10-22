//! `gecko bench` (algorithm leaderboard), `gecko verify` (re-run a run), `gecko perf` (scorer speed).

use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use gecko_core::optimize::{self, Algo, Control, Params};
use gecko_core::run;
use gecko_core::{Scorer, SCHEMA};
use serde_json::json;

use crate::ctx::{git_commit, parse_duration, parse_moves, score_layout, threads, Ctx, Inputs};

#[derive(clap::Args)]
pub struct BenchArgs {
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
}

#[derive(clap::Args)]
pub struct VerifyArgs {
    /// run:<id> (or just <id>).
    run: String,
}

#[derive(clap::Args)]
pub struct PerfArgs {
    #[arg(long, default_value = "qwerty")]
    layout: String,
    /// Append {date, commit, full/s, delta/s} to bench/history.jsonl.
    #[arg(long)]
    record: bool,
    #[command(flatten)]
    inputs: Inputs,
}

pub fn bench(ctx: &Ctx, a: BenchArgs) -> Result<()> {
    let l = ctx.load(&a.inputs)?;
    let start = ctx.start(None, &l)?;
    let time_ms = parse_duration(&a.time)?;
    let threads = threads(a.threads);
    let sc = Scorer::new(&l.geometry, &l.corpus, &l.profile, &start)?;
    let mut variants = a.moves.split('/').map(parse_moves).collect::<Result<Vec<_>>>()?;
    if let Some(w) = a.depth2_width {
        variants.iter_mut().for_each(|m| m.depth2_width = w);
    }
    let mut out = Vec::new();
    for name in a.algos.split(',') {
        let algo = Algo::parse(name.trim()).with_context(|| format!("unknown algo '{name}'"))?;
        for mv in &variants {
            let mut scores = Vec::new();
            let mut eps = Vec::new();
            for seed in 1..=a.seeds {
                let params =
                    Params { algo, seed, threads, time_ms: Some(time_ms), evals: None, top: 1, moves: mv.clone(), ..Default::default() };
                let c = optimize::Ctx::new(&sc, (0..sc.n).collect(), sc.identity());
                let o = optimize::run(&c, &params, Duration::from_secs(3600), &Control::default(), |_| {});
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
        return Ok(());
    }
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
    Ok(())
}

pub fn verify(ctx: &Ctx, a: VerifyArgs) -> Result<()> {
    let id = a.run.strip_prefix("run:").unwrap_or(&a.run);
    let dir = ctx.store.runs.join(id);
    eprintln!("verify {id}  (re-running)");
    let v = run::verify(&ctx.store, &dir)?;
    if ctx.json {
        ctx.print(json!(v));
    } else {
        for c in &v.checks {
            if c.ok {
                println!("ok   {:<9} {}", c.name, c.actual);
            } else {
                println!("FAIL {:<9} expected {}\n               got {}", c.name, c.expected, c.actual);
            }
        }
        println!("{}", if v.ok { "reproduced" } else { "NOT reproduced" });
    }
    if !v.ok {
        bail!("run {id} did not reproduce");
    }
    Ok(())
}

/// Calls of `f` per second over `ms` milliseconds (`f` returns how many operations it did).
fn rate(ms: u128, mut f: impl FnMut() -> Result<u64>) -> Result<f64> {
    let t = Instant::now();
    let mut k = 0u64;
    while t.elapsed().as_millis() < ms {
        k += f()?;
    }
    Ok(k as f64 / t.elapsed().as_secs_f64())
}

pub fn perf(ctx: &Ctx, a: PerfArgs) -> Result<()> {
    let l = ctx.load(&a.inputs)?;
    let layout = ctx.layout(&a.layout, &l)?;
    let (sc, mut st) = score_layout(&l, &layout)?;
    let mut rng = gecko_core::rng::Rng::new(1);
    let mut sink = 0.0;
    let full = rate(500, || {
        sink += sc.full(&st);
        Ok(1)
    })?;
    let delta = rate(1000, || {
        for _ in 0..1000 {
            let a = rng.below(sc.n);
            let b = (a + 1 + rng.below(sc.n - 1)) % sc.n;
            let d = sc.delta(&st, a, b);
            if d < 0.0 {
                sc.apply(&mut st, a, b, d);
            }
        }
        Ok(1000)
    })?;
    // Geometry search: a scorer built from scratch vs re-costed after one key moves.
    let build = rate(500, || {
        sink += Scorer::new(&l.geometry, &l.corpus, &l.profile, &layout)?.n as f64;
        Ok(1)
    })?;
    let (mut moved, mut g) = (sc.clone(), l.geometry.clone());
    let mut k = 0usize;
    let rebuild = rate(500, || {
        let i = k % g.len();
        g.key[i].pos[0] += if k.is_multiple_of(2) { 0.01 } else { -0.01 };
        sink += moved.set_geometry(&g)? as f64;
        k += 1;
        Ok(1)
    })?;
    std::hint::black_box(sink);
    let recorded = if a.record {
        let entry = json!({
            "date": run::now_stamp().1,
            "commit": git_commit(&ctx.store.root),
            "layout": a.layout,
            "geometry": l.geometry.name,
            "corpus": l.corpus.name,
            "profile": l.profile.name,
            "full_per_sec": full.round() as u64,
            "delta_per_sec": delta.round() as u64,
            "build_per_sec": build.round() as u64,
            "rebuild_per_sec": rebuild.round() as u64,
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
            "build_per_sec": build, "rebuild_per_sec": rebuild,
            "recorded": recorded.as_ref().map(|p| p.display().to_string())}));
        return Ok(());
    }
    println!("full evaluation  {:>12.0} /s  ({:.2} µs)", full, 1e6 / full);
    println!("swap delta       {:>12.0} /s  ({:.3} µs)", delta, 1e6 / delta);
    println!("scorer build     {:>12.0} /s  ({:.2} ms)", build, 1e3 / build);
    println!("1-key rebuild    {:>12.0} /s  ({:.2} ms)", rebuild, 1e3 / rebuild);
    if let Some(p) = recorded {
        println!("recorded in {}", p.display());
    }
    Ok(())
}
