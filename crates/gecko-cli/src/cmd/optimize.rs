//! `gecko optimize`: run one search algorithm and write a run directory.

use anyhow::{Context, Result};
use gecko_core::optimize::memetic::LocalSearch;
use gecko_core::optimize::{Algo, Control, Params};
use gecko_core::report::breakdown;
use gecko_core::run::{self, RunResult, RunSpec};
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
        Ok(p)
    }
}

pub fn run(ctx: &Ctx, a: OptimizeArgs) -> Result<()> {
    let params = a.params()?;
    let l = ctx.load(&a.inputs)?;
    let holdout = a.holdout.as_ref().map(|h| ctx.store.corpus(h)).transpose()?;
    let start = ctx.start(a.from.as_deref(), &l)?;
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
    if ctx.json {
        ctx.print(json!({"schema": SCHEMA, "run": res}));
        return Ok(());
    }
    eprintln!();
    print_result(&res, &l)
}

fn print_result(res: &RunResult, l: &Loaded) -> Result<()> {
    println!("run {}  {} evals in {:.1}s ({:.2} M evals/s)", res.id, res.evals, res.elapsed_ms as f64 / 1000.0, res.evals_per_sec / 1e6);
    if !res.front.is_empty() {
        println!(
            "Pareto front: {} layouts over {}",
            res.front.len(),
            res.objectives.iter().map(|o| o.id.as_str()).collect::<Vec<_>>().join(", ")
        );
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
        let layout = Layout::from_flat("best", &l.geometry, b.chars.chars().collect())?;
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
