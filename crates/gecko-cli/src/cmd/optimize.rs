//! `gecko optimize`: run one search algorithm and write a run directory.

use anyhow::{Context, Result};
use gecko_core::optimize::memetic::LocalSearch;
use gecko_core::optimize::{Algo, Control, Params};
use gecko_core::report::breakdown;
use gecko_core::run::{self, RunResult, RunSpec};
use gecko_core::profile::Fidelity;
use gecko_core::{Layout, SCHEMA};
use serde_json::json;

use crate::ctx::{parse_moves, score_layout, threads, time_budget, Ctx, Inputs, Loaded};
use crate::fmt::{layout_rows, print_breakdown};

#[derive(clap::Args)]
pub struct OptimizeArgs {
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
    #[arg(long, conflicts_with = "keyset")]
    from: Option<String>,
    /// Build the symbol set and start layout from the board and corpus instead of --from:
    /// alpha, shift (Shift-paired symbol keys) or layer (symbol layer). See `gecko keyset`.
    #[arg(long)]
    keyset: Option<String>,
    #[command(flatten)]
    keyset_opts: super::keyset::KeysetOpts,
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
    /// Adaptive fidelity: search on the K most frequent trigrams, confirm on the full tables
    /// (overrides the profile's `fidelity.top_k`; 0 = exact search).
    #[arg(long)]
    fidelity: Option<usize>,
    /// Also search the board's alternative fingers (keys' `alt`) after the layout search.
    #[arg(long)]
    fingering: bool,
    /// Fingering search: SA swaps per candidate fingering (default 300000).
    #[arg(long)]
    fingering_evals: Option<u64>,
    /// Also score the results on this corpus, which the search never sees (L06).
    #[arg(long)]
    holdout: Option<String>,
    #[command(flatten)]
    inputs: Inputs,
}

impl OptimizeArgs {
    /// Search parameters: defaults, overridden by every flag that was given.
    fn params(&self) -> Result<Params> {
        let algo = Algo::parse(&self.algo)
            .with_context(|| format!("unknown algo '{}' (hc, sa, ga, memetic, bls, explore, tabu, qd, nsga2, rwsa)", self.algo))?;
        let mut p = Params {
            algo,
            seed: self.seed,
            threads: threads(self.threads),
            time_ms: time_budget(self.time.as_deref(), self.evals, "10s")?,
            evals: self.evals,
            top: self.top,
            ..Default::default()
        };
        fn set<T: Clone>(dst: &mut T, v: &Option<T>) {
            if let Some(v) = v {
                *dst = v.clone();
            }
        }
        set(&mut p.sa.steps, &self.sa_steps);
        set(&mut p.ga.pop_size, &self.ga_pop);
        set(&mut p.nsga2.pop_size, &self.ga_pop);
        set(&mut p.memetic.ga.pop_size, &self.memetic_pop);
        if let Some(local) = &self.local_search {
            p.memetic.local_search = if local == "tabu" { LocalSearch::Tabu } else { LocalSearch::Hc };
        }
        set(&mut p.memetic.local_steps, &self.local_steps);
        set(&mut p.memetic.islands, &self.islands);
        set(&mut p.memetic.migration_interval, &self.migration_interval);
        set(&mut p.memetic.migrants, &self.migrants);
        set(&mut p.nsga2.objectives, &self.objectives);
        set(&mut p.nsga2.generations, &self.nsga_generations);
        set(&mut p.nsga2.archive_size, &self.front_size);
        if let Some(m) = &self.moves {
            p.moves = parse_moves(m)?;
        }
        set(&mut p.moves.depth2_width, &self.depth2_width);
        p.fingering.search = self.fingering;
        set(&mut p.fingering.evals, &self.fingering_evals);
        Ok(p)
    }
}

pub fn run(ctx: &Ctx, a: OptimizeArgs) -> Result<()> {
    let params = a.params()?;
    let mut l = ctx.load(&a.inputs)?;
    if let Some(k) = a.fidelity {
        l.profile.fidelity = (k > 0).then(|| l.profile.fidelity.map_or(Fidelity::new(k), |f| Fidelity { top_k: k, ..f }));
    }
    let holdout = a.holdout.as_ref().map(|h| ctx.store.corpus(h)).transpose()?;
    let start = match &a.keyset {
        Some(mode) => {
            let (ks, placed) = a.keyset_opts.start(ctx, &mut l, mode, &a.pin, a.seed, threads(a.threads))?;
            let layout = placed.as_ref().map_or_else(|| ks.layout.clone(), |p| p.layout.clone());
            if !ctx.json {
                super::keyset::print_keyset(&ks, &layout, &l, placed.as_ref());
            }
            layout
        }
        None => ctx.start(a.from.as_deref(), &l)?,
    };
    let spec = RunSpec {
        geometry: &l.geometry,
        corpus: &l.corpus,
        profile: &l.profile,
        start: &start,
        pins: &a.pin,
        params,
        holdout: holdout.as_ref(),
    };
    let prep = run::prepare(&ctx.store.runs, &spec)?;
    eprintln!("run {}  ({} · {} threads · {})", prep.config.id, spec.params.algo.name(), spec.params.threads, l.profile.name);
    let quiet = ctx.json;
    let res = run::execute(&prep, &spec, &Control::default(), |p| {
        if !quiet {
            eprint!("\r  {:>6.1}s  {:>12} evals  best {:.4}  {}", p.elapsed_ms as f64 / 1000.0, p.evals, p.best, p.best_chars);
        }
    })?;
    if res.geometry.starts_with("run:") {
        // The fingering search changed fingers: show the results on the chosen board.
        l.geometry = ctx.store.geometry(&res.geometry)?;
    }
    let ties = near_ties(&res, &l);
    if ctx.json {
        ctx.print(json!({"schema": SCHEMA, "run": res, "near_ties": ties}));
        return Ok(());
    }
    eprintln!();
    print_result(&res, &l, &start.shifted)?;
    if !ties.is_empty() {
        println!("\nnear-tie: {} other result(s) within {NEAR_TIE} of #1 but a different arrangement:", ties.len());
        for t in &ties {
            println!("  #{:<2} +{:.3}  {:>2} keys differ  {}", t.rank, t.gap, t.keys, t.chars);
        }
        println!("  Weight uncertainty moves scores more than this; treat them as one tier (gecko sweep, compare --robust).");
    }
    Ok(())
}

/// Score gap under which two optima count as tied. Default profile, `gecko sweep`
/// (`bench/sweep-2026-10-22/`): halving any one weight costs the optimum up to 0.4, and
/// σ = 0.5 log-normal weight draws a median 1.1.
const NEAR_TIE: f64 = 0.4;

/// Keys two results may differ in and still be the same arrangement (a swap or two).
const SAME_KEYS: usize = 4;

#[derive(serde::Serialize)]
struct NearTie {
    rank: usize,
    /// Score above #1.
    gap: f64,
    /// Keys that differ from #1 (mirror-invariant).
    keys: usize,
    chars: String,
}

/// Results within [`NEAR_TIE`] of the best that differ from it in more than [`SAME_KEYS`] keys.
fn near_ties(res: &RunResult, l: &Loaded) -> Vec<NearTie> {
    let Some(first) = res.best.first() else { return Vec::new() };
    let mirror = gecko_core::robust::mirror_slots(&l.geometry);
    let a: Vec<char> = first.chars.chars().collect();
    let mirror = mirror.filter(|m| m.len() == a.len());
    res.best
        .iter()
        .skip(1)
        .filter(|r| r.score - first.score <= NEAR_TIE)
        .filter_map(|r| {
            let b: Vec<char> = r.chars.chars().collect();
            let keys = if a.len() == b.len() { gecko_core::robust::key_distance(&a, &b, mirror.as_deref()) } else { usize::MAX };
            (keys > SAME_KEYS && keys != usize::MAX).then(|| NearTie {
                rank: r.rank,
                gap: r.score - first.score,
                keys,
                chars: r.chars.clone(),
            })
        })
        .collect()
}

fn print_result(res: &RunResult, l: &Loaded, shifted: &[String]) -> Result<()> {
    println!("run {}  {} evals in {:.1}s ({:.2} M evals/s)", res.id, res.evals, res.elapsed_ms as f64 / 1000.0, res.evals_per_sec / 1e6);
    if !res.front.is_empty() {
        println!(
            "Pareto front: {} layouts over {}",
            res.front.len(),
            res.objectives.iter().map(|o| o.id.as_str()).collect::<Vec<_>>().join(", ")
        );
    }
    if let Some(f) = &res.fidelity {
        let last = f.levels.len().saturating_sub(1);
        println!(
            "fidelity: levels {} trigrams; {} cheap bests confirmed, {} rejected; workers ended on level {}",
            f.levels.iter().map(|k| k.to_string()).collect::<Vec<_>>().join(" → "),
            f.confirmed,
            f.rejected,
            f.final_level.iter().map(|&l| if l == last { "full".to_string() } else { l.to_string() }).collect::<Vec<_>>().join(",")
        );
    }
    if let Some(f) = &res.fingering {
        println!(
            "fingering: {} candidates, {} reassignment(s); best layout {:.3} on the given fingering → {:.3}",
            f.candidates, f.steps, f.base_score, f.score
        );
        for c in &f.changes {
            println!("  key {} (row {}, col {}): {} → {} (effort {})", c.key, c.row, c.col, c.from.name(), c.to.name(), c.effort);
        }
        if !f.changes.is_empty() {
            println!("  results are scored on that fingering: geometry {}", res.geometry);
        }
    }
    for r in res.best.iter().take(5) {
        let held = r.holdout.map_or(String::new(), |h| format!("  (holdout {h:.3})"));
        println!("#{:<2} {:>8.3}  {}{held}", r.rank, r.score, r.rows.join(" "));
    }
    if let Some(h) = &res.holdout {
        println!(
            "holdout {}: start {:.3} → best {:.3} (search {:.3} → {:.3}); keeps {:.1}% of the gain{}",
            h.corpus,
            h.start,
            h.best,
            h.start_search,
            h.best_search,
            100.0 * h.retained,
            h.rank_corr.map_or(String::new(), |r| format!(", rank ρ {r:.2} over the top {}", res.best.len()))
        );
    }
    if let Some(b) = res.best.first() {
        let layout = Layout::from_flat("best", &l.geometry, b.chars.chars().collect())?.with_shifted(shifted.to_vec())?;
        let (sc, st) = score_layout(l, &layout)?;
        println!("\nbest\n  {}\n", layout_rows(&layout, &l.geometry));
        print_breakdown(&breakdown(&sc, &st));
    }
    if let Some(b) = &res.bound {
        println!(
            "\nlower bound ({}, {} symbols, {} iterations, {} ms)\n  total  {:>9.3} (plain GL {:.3}) ≤ optimum; best {:.3} is within {:.1}%\n  pairs  {:>9.3} (plain GL {:.3}) ≤ optimum; best {:.3} is within {:.1}%  (mono + bigram/skipgram part only)",
            b.method, b.symbols, b.iters, b.elapsed_ms, b.total.lower, b.total.gilmore_lawler, b.total.value, b.total.gap_pct,
            b.pair.lower, b.pair.gilmore_lawler, b.pair.value, b.pair.gap_pct
        );
    }
    Ok(())
}
