//! `gecko features`, `gecko judge`, `gecko sweep`: per-metric costs for analysis, independent
//! judges, and how far the optimum moves with the weights (`context/cli.md`).

use std::io::{BufRead, IsTerminal, Write};

use anyhow::{bail, Result};
use gecko_core::metrics::{index_of, Dir, METRICS};
use gecko_core::rng::Rng;
use gecko_core::robust::{score_of, sweep as run_sweep, SweepSpec, UnitCosts};
use gecko_core::{hash, Layout, SCHEMA};
use serde_json::json;

use crate::ctx::{threads, Ctx, Inputs, Loaded};

#[derive(clap::Args)]
pub struct FeaturesArgs {
    /// Layout specs (reference name, .toml, run:<id>[#rank], cmini:<name>, raw:<chars>). None:
    /// one spec per stdin line, or every reference layout when stdin is a terminal.
    layouts: Vec<String>,
    /// Add every community (cmini) layout that fits the geometry.
    #[arg(long)]
    community: bool,
    /// Add N random arrangements of the reference layout's symbols.
    #[arg(long, default_value_t = 0)]
    random: usize,
    #[arg(long, default_value_t = 1)]
    seed: u64,
    #[command(flatten)]
    inputs: Inputs,
}

#[derive(clap::Args)]
pub struct JudgeArgs {
    /// Layout specs (default: every reference layout).
    layouts: Vec<String>,
    /// Also give each judge's percentile among the community (cmini) layouts (0 = best).
    #[arg(long)]
    community: bool,
    #[command(flatten)]
    inputs: Inputs,
}

#[derive(clap::Args)]
pub struct SweepArgs {
    /// SA evaluations per search.
    #[arg(long, default_value_t = 5_000_000)]
    evals: u64,
    /// SA seeds per weight vector (best kept).
    #[arg(long, default_value_t = 1)]
    seeds: u64,
    /// Random log-normal weight vectors besides drop / half / double of each weight.
    #[arg(long, default_value_t = 20)]
    draws: usize,
    /// Log-normal σ of the random draws (0.5 ≈ each weight within ×0.37–×2.7 at 95%).
    #[arg(long, default_value_t = 0.5)]
    sigma: f64,
    #[arg(long, default_value_t = 1)]
    seed: u64,
    /// Worker threads per search (default: all cores).
    #[arg(long)]
    threads: Option<usize>,
    /// Layouts within this many keys of each other (mirror-invariant) form one family.
    #[arg(long, default_value_t = 10)]
    family_keys: usize,
    /// Start layout (also defines the symbol set).
    #[arg(long)]
    from: Option<String>,
    /// Symbols that keep their start position, e.g. ",./;".
    #[arg(long, default_value = "")]
    pin: String,
    /// Write the full report (every optimum and weight vector) as JSON to this file.
    #[arg(long)]
    out: Option<std::path::PathBuf>,
    #[command(flatten)]
    inputs: Inputs,
}

/// The judges `gecko judge` reports, with their direction (true = higher is better).
const JUDGES: [(&str, bool); 6] = [("typing_time", false), ("engram", false), ("engram_formula", false), ("home_row", true), ("effort", false), ("sfb", false)];

/// Every community layout that fits the geometry, as (spec, layout).
fn community(ctx: &Ctx, l: &Loaded) -> Result<Vec<(String, Layout)>> {
    let b = ctx.store.community()?;
    Ok(b.layouts
        .iter()
        .filter_map(|c| Some((format!("cmini:{}", c.name), c.layout(&l.geometry.name).ok()?.fit(&l.geometry).ok()?)))
        .collect())
}

/// Natural-unit value of metric `i` from its unit cost (rewards are stored negated).
fn value(costs: &[f64], i: usize) -> f64 {
    if METRICS[i].dir == Dir::Max {
        -costs[i]
    } else {
        costs[i]
    }
}

pub fn features(ctx: &Ctx, a: FeaturesArgs) -> Result<()> {
    let l = ctx.load(&a.inputs)?;
    let mut specs = a.layouts.clone();
    if specs.is_empty() && !a.community && a.random == 0 {
        if std::io::stdin().is_terminal() {
            specs = ctx.store.list("layouts");
        } else {
            for line in std::io::stdin().lock().lines() {
                let s = line?.trim().to_string();
                if !s.is_empty() && !s.starts_with('#') {
                    specs.push(s);
                }
            }
        }
    }
    let mut layouts: Vec<(String, Layout)> = Vec::new();
    for s in &specs {
        match ctx.layout(s, &l) {
            Ok(x) => layouts.push((s.clone(), x)),
            Err(e) => eprintln!("skip {s}: {e}"),
        }
    }
    if a.community {
        layouts.extend(community(ctx, &l)?);
    }
    if a.random > 0 {
        let base = ctx.start(None, &l)?;
        let mut rng = Rng::new(a.seed);
        for k in 0..a.random {
            let mut chars = base.chars.clone();
            rng.shuffle(&mut chars);
            layouts.push((format!("random:{}", k + 1), Layout::from_flat("random", &l.geometry, chars)?));
        }
    }
    let mut unit = UnitCosts::new(&l.geometry, &l.corpus, &l.profile);
    let out = std::io::stdout();
    let mut out = out.lock();
    for (spec, layout) in &layouts {
        let c = match unit.of(layout) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("skip {spec}: {e}");
                continue;
            }
        };
        let costs: serde_json::Map<String, serde_json::Value> =
            METRICS.iter().zip(&c).filter(|(d, _)| d.dir != Dir::Info).map(|(d, v)| (d.id.to_string(), json!(v))).collect();
        let line =
            json!({"layout": spec, "chars": layout.chars.iter().collect::<String>(), "score": score_of(&c, &l.profile), "costs": costs});
        if writeln!(out, "{line}").is_err() {
            break;
        }
    }
    Ok(())
}

pub fn judge(ctx: &Ctx, a: JudgeArgs) -> Result<()> {
    let l = ctx.load(&a.inputs)?;
    let names = if a.layouts.is_empty() { ctx.store.list("layouts") } else { a.layouts };
    let mut unit = UnitCosts::new(&l.geometry, &l.corpus, &l.profile);
    let idx: Vec<usize> = JUDGES.iter().map(|(id, _)| index_of(id).expect("judge metric")).collect();
    let mut rows = Vec::new();
    for n in &names {
        let c = unit.of(&ctx.layout(n, &l)?)?;
        rows.push((n.clone(), score_of(&c, &l.profile), idx.iter().map(|&i| value(&c, i)).collect::<Vec<f64>>()));
    }
    rows.sort_by(|x, y| x.1.total_cmp(&y.1));
    // Community values per judge, for percentiles.
    let pool: Option<Vec<Vec<f64>>> = if a.community {
        let mut cols = vec![Vec::new(); JUDGES.len()];
        for (_, lay) in community(ctx, &l)? {
            if let Ok(c) = unit.of(&lay) {
                for (k, &i) in idx.iter().enumerate() {
                    cols[k].push(value(&c, i));
                }
            }
        }
        if cols[0].is_empty() {
            bail!("no community layout fits {}", l.geometry.name);
        }
        Some(cols)
    } else {
        None
    };
    // Share of community layouts strictly better than v on judge k (0 = best, 100 = worst).
    let pct = |k: usize, v: f64| {
        pool.as_ref().map(|p| {
            let better = p[k].iter().filter(|&&x| if JUDGES[k].1 { x > v } else { x < v }).count();
            100.0 * better as f64 / p[k].len() as f64
        })
    };
    if ctx.json {
        let v: Vec<_> = rows
            .iter()
            .map(|(n, s, vals)| {
                let j: serde_json::Map<String, serde_json::Value> = JUDGES
                    .iter()
                    .enumerate()
                    .map(|(k, (id, _))| (id.to_string(), json!({"value": vals[k], "percentile": pct(k, vals[k])})))
                    .collect();
                json!({"layout": n, "score": s, "judges": j})
            })
            .collect();
        ctx.print(json!({"schema": SCHEMA, "geometry": l.geometry.name, "profile": l.profile.name, "corpus": l.corpus.name,
            "community": pool.as_ref().map(|p| p[0].len()), "layouts": v}));
        return Ok(());
    }
    println!(
        "judges on {} · {}; score = profile {}{}",
        l.geometry.name,
        l.corpus.name,
        l.profile.name,
        pool.as_ref().map_or(String::new(), |p| format!("; (percentile among {} cmini layouts, 0 = best)", p[0].len()))
    );
    print!("{:<16} {:>8}", "layout", "score");
    let w = if pool.is_some() { 13 } else { 8 };
    for (id, _) in JUDGES {
        print!(" {:>w$}", id);
    }
    println!();
    for (n, s, vals) in &rows {
        print!("{:<16} {:>8.3}", n, s);
        for (k, v) in vals.iter().enumerate() {
            let cell = match pct(k, *v) {
                Some(p) => format!("{v:.2} ({p:.0})"),
                None => format!("{v:.2}"),
            };
            print!(" {:>w$}", cell);
        }
        println!();
    }
    println!("typing_time (% of a median keystroke interval) · engram / engram_formula discomfort % · home_row % (higher better) · effort · sfb %");
    Ok(())
}

pub fn sweep(ctx: &Ctx, a: SweepArgs) -> Result<()> {
    let l = ctx.load(&a.inputs)?;
    let start = ctx.start(a.from.as_deref(), &l)?;
    let flat = start.flat(&l.geometry)?.1;
    let movable: Vec<usize> = (0..flat.len()).filter(|&s| !a.pin.contains(flat[s])).collect();
    let spec = SweepSpec {
        sigma: a.sigma,
        draws: a.draws,
        seed: a.seed,
        evals: a.evals,
        seeds: a.seeds,
        threads: threads(a.threads),
        family_keys: a.family_keys,
    };
    let quiet = ctx.json;
    let r = run_sweep(&l.geometry, &l.corpus, &l.profile, &start, &movable, &spec, |done, total| {
        if !quiet {
            eprint!("\r  search {done}/{total}");
        }
    })?;
    if !quiet {
        eprintln!();
    }
    let inputs = json!({
        "geometry": {"name": l.geometry.name, "hash": hash::hex(l.geometry.hash())},
        "corpus": {"name": l.corpus.name, "hash": hash::hex(l.corpus.hash())},
        "profile": {"name": l.profile.name, "hash": hash::hex(l.profile.hash())},
        "start": start.name, "pins": a.pin,
    });
    let report = json!({"schema": SCHEMA, "inputs": inputs, "sweep": r});
    if let Some(path) = &a.out {
        ctx.write_file(path, &serde_json::to_string_pretty(&report)?)?;
    }
    if ctx.json {
        ctx.print(report);
        return Ok(());
    }
    let base = &r.rows[0];
    let e01 = |row: &gecko_core::robust::SweepRow| row.judges.get("typing_time").copied().unwrap_or(f64::NAN);
    println!(
        "base optimum ({}): {:.3}  typing_time {:.3}\n  {}",
        r.profile,
        base.score,
        e01(base),
        crate::fmt::chars_rows(&base.chars, &l.geometry)
    );
    println!("\nfamilies (layouts within {} keys, mirror-invariant), by how many weight vectors land in them:", a.family_keys);
    for f in r.families.iter().take(8) {
        let tag = if f.family == base.family { "  ← base" } else { "" };
        println!("  {:>3} vectors  best {:.3}  {}{tag}", f.count, f.best_score, f.best);
    }
    println!(
        "\n{} random draws (σ {}): regret median {:.3}, p90 {:.3}; keys moved median {}, p90 {}",
        a.draws, a.sigma, r.draw_regret[0], r.draw_regret[1], r.draw_keys_moved[0], r.draw_keys_moved[1]
    );
    println!("regret = what keeping the base optimum costs if those weights were right (in their own score units)");
    println!("\n{:<5} {:>8}  {:>22}  {:>26}", "id", "weight", "keys moved drop/½/×2", "regret drop/½/×2");
    let mut sens = r.sensitivity.clone();
    sens.sort_by(|x, y| y.regret.iter().sum::<f64>().total_cmp(&x.regret.iter().sum::<f64>()));
    for s in &sens {
        println!(
            "{:<5} {:>8}  {:>22}  {:>26}",
            s.id,
            s.weight,
            format!("{} / {} / {}", s.keys_moved[0], s.keys_moved[1], s.keys_moved[2]),
            format!("{:.3} / {:.3} / {:.3}", s.regret[0], s.regret[1], s.regret[2])
        );
    }
    Ok(())
}
