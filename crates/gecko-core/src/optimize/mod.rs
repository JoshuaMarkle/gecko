//! Discrete layout optimizers and the multi-threaded runner. Algorithms contain only search
//! logic; the runner owns budgets, threads, progress and top-K. See `context/algorithms/`.

pub mod anneal;
pub mod bls;
pub mod genetic;
pub mod hill_climb;
pub mod map_elites;
pub mod memetic;
pub mod moves;
pub mod nsga2;
pub mod tabu;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::rng::Rng;
use crate::scorer::{Scorer, State};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Algo {
    /// Steepest-descent hill climbing with random restarts.
    Hc,
    /// Simulated annealing with restarts and a steepest-descent polish.
    Sa,
    /// Genetic algorithm (order crossover, swap mutation, tournament selection).
    Ga,
    /// Island genetic algorithm with local refinement of every child.
    Memetic,
    /// Breakout local search: descend, kick with adaptive strength, descend again.
    Bls,
    /// Search-space exploration: landscape probe + BLS with exclusion restarts, writing an
    /// atlas of every local optimum (`crate::explore`).
    Explore,
    /// Robust tabu search (Taillard) with an incrementally updated Δ matrix.
    Tabu,
    /// MAP-Elites quality-diversity search.
    Qd,
    /// NSGA-II multiobjective search.
    Nsga2,
    /// Random-weight simulated annealing with a Pareto archive.
    Rwsa,
}

impl Algo {
    pub const ALL: [Algo; 10] =
        [Algo::Hc, Algo::Sa, Algo::Ga, Algo::Memetic, Algo::Bls, Algo::Explore, Algo::Tabu, Algo::Qd, Algo::Nsga2, Algo::Rwsa];
    pub fn name(self) -> &'static str {
        match self {
            Algo::Hc => "hc",
            Algo::Sa => "sa",
            Algo::Ga => "ga",
            Algo::Memetic => "memetic",
            Algo::Bls => "bls",
            Algo::Explore => "explore",
            Algo::Tabu => "tabu",
            Algo::Qd => "qd",
            Algo::Nsga2 => "nsga2",
            Algo::Rwsa => "rwsa",
        }
    }
    pub fn parse(s: &str) -> Option<Algo> {
        Algo::ALL.into_iter().find(|a| a.name() == s)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Params {
    pub algo: Algo,
    pub seed: u64,
    pub threads: usize,
    /// Wall-clock budget in milliseconds (all workers run in parallel).
    pub time_ms: Option<u64>,
    /// Total evaluation budget, split evenly across independent workers, or counted over
    /// synchronized island generations for memetic. Deterministic, unlike time.
    pub evals: Option<u64>,
    /// Number of distinct best layouts to keep.
    pub top: usize,
    #[serde(default)]
    pub sa: anneal::SaParams,
    #[serde(default)]
    pub ga: genetic::GaParams,
    #[serde(default)]
    pub memetic: memetic::MemeticParams,
    #[serde(default)]
    pub bls: bls::BlsParams,
    #[serde(default)]
    pub explore: crate::explore::ExploreParams,
    #[serde(default)]
    pub tabu: tabu::TabuParams,
    /// Extra neighbourhoods for every steepest descent (all algorithms).
    #[serde(default)]
    pub moves: moves::MoveParams,
    #[serde(default)]
    pub qd: map_elites::QdParams,
    #[serde(default)]
    pub nsga2: nsga2::NsgaParams,
    /// C11: search the board's alternative fingers after the layout search (`run::execute`).
    #[serde(default, skip_serializing_if = "crate::fingering::FingeringParams::is_off")]
    pub fingering: crate::fingering::FingeringParams,
}

impl Default for Params {
    fn default() -> Self {
        Params {
            algo: Algo::Sa,
            seed: 1,
            threads: 1,
            time_ms: Some(5_000),
            evals: None,
            top: 10,
            sa: Default::default(),
            ga: Default::default(),
            memetic: Default::default(),
            bls: Default::default(),
            explore: Default::default(),
            tabu: Default::default(),
            moves: Default::default(),
            qd: Default::default(),
            nsga2: Default::default(),
            fingering: Default::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Scored {
    pub score: f64,
    /// `sym[slot]` in the scorer's symbol ids.
    pub sym: Vec<u8>,
}

/// How a local-search algorithm left one local optimum for the next (LON edge label).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Kick {
    /// `strength` random swaps.
    Swaps = 0,
    /// Two whole same-size columns exchanged.
    Columns = 1,
}

/// Receives search events outside the hot loop (once per local optimum / kick), e.g. the
/// explore archive. Called concurrently from all workers.
pub trait Observer: Sync {
    /// `st` is a local optimum (or, with `constrained`, the end of an exclusion descent).
    /// Returns an id for edges.
    fn optimum(&self, st: &State, constrained: bool) -> u32;
    /// A kick from optimum `from` led (after descent) to optimum `to`.
    fn edge(&self, from: u32, to: u32, kick: Kick, strength: u8);
    /// Distinct good layouts (`sym[slot]`) that exclusion restarts must stay away from.
    fn exclusions(&self) -> Arc<Vec<Vec<u8>>> {
        Arc::new(Vec::new())
    }
    /// A layout (`sym[slot]`) to mutate next, e.g. a random MAP-Elites elite.
    fn parent(&self, _rng: &mut Rng) -> Option<Vec<u8>> {
        None
    }
}

/// Search context shared by all workers (read-only).
#[derive(Clone)]
pub struct Ctx<'a> {
    pub scorer: &'a Scorer,
    /// Symbols allowed to move (not on a pinned slot).
    pub movable: Vec<usize>,
    pub start: State,
    pub observer: Option<&'a dyn Observer>,
    /// Neighbourhoods used by [`steepest_descent`] (set from [`Params::moves`] by [`run`]).
    pub moves: moves::MoveParams,
    /// Same-finger column pairs for the column move ([`moves::column_pairs`]).
    pub col_pairs: Vec<(Vec<usize>, Vec<usize>)>,
}

impl<'a> Ctx<'a> {
    /// Layer keys never move (they define the layered slot structure), nor does Shift.
    pub fn new(scorer: &'a Scorer, mut movable: Vec<usize>, start: State) -> Self {
        movable.retain(|&s| !crate::layout::is_fixed_key(scorer.chars[s]));
        let col_pairs = moves::column_pairs(scorer, &movable, &start);
        Ctx { scorer, movable, start, observer: None, moves: Default::default(), col_pairs }
    }

    pub fn with_observer(mut self, observer: &'a dyn Observer) -> Self {
        self.observer = Some(observer);
        self
    }

    pub fn with_moves(mut self, moves: moves::MoveParams) -> Self {
        self.moves = moves;
        self
    }

    /// A random layout: movable symbols shuffled among their slots; pinned symbols stay.
    pub fn random_state(&self, rng: &mut Rng) -> State {
        let mut sym = self.start.sym.clone();
        let mut slots: Vec<usize> = self.movable.iter().map(|&s| self.start.pos[s] as usize).collect();
        let mut syms: Vec<u8> = slots.iter().map(|&p| sym[p]).collect();
        rng.shuffle(&mut syms);
        slots.sort_unstable();
        for (p, s) in slots.iter().zip(syms) {
            sym[*p] = s;
        }
        self.scorer.state_from_sym(sym)
    }

    /// A uniformly random pair of distinct movable symbols.
    #[inline]
    pub fn random_pair(&self, rng: &mut Rng) -> (usize, usize) {
        let m = self.movable.len();
        let i = rng.below(m);
        let mut j = rng.below(m - 1);
        if j >= i {
            j += 1;
        }
        (self.movable[i], self.movable[j])
    }
}

pub struct StepInfo {
    pub evals: u64,
}

pub trait Optimizer {
    fn init(&mut self, ctx: &Ctx, rng: &mut Rng);
    /// Do a bounded batch of work.
    fn step(&mut self, ctx: &Ctx, rng: &mut Rng) -> StepInfo;
    /// Called once when the budget ends (e.g. polish the in-progress solution).
    fn finish(&mut self, ctx: &Ctx, rng: &mut Rng);
    fn top(&self) -> &TopK;
    /// Best layout so far for progress reports (may be mid-search, not yet in `top`).
    fn live_best(&self) -> Option<(f64, &[u8])> {
        self.top().best().map(|s| (s.score, s.sym.as_slice()))
    }
}

/// Best distinct layouts seen, ascending by score.
#[derive(Clone, Debug, Default)]
pub struct TopK {
    pub k: usize,
    pub items: Vec<Scored>,
}

impl TopK {
    pub fn new(k: usize) -> Self {
        TopK { k: k.max(1), items: Vec::new() }
    }
    pub fn best(&self) -> Option<&Scored> {
        self.items.first()
    }
    pub fn best_score(&self) -> f64 {
        self.items.first().map(|s| s.score).unwrap_or(f64::INFINITY)
    }
    pub fn offer(&mut self, score: f64, sym: &[u8]) {
        if self.items.len() >= self.k && score >= self.items.last().unwrap().score {
            return;
        }
        if self.items.iter().any(|s| s.sym == sym) {
            return;
        }
        let i = self.items.partition_point(|s| s.score <= score);
        self.items.insert(i, Scored { score, sym: sym.to_vec() });
        self.items.truncate(self.k);
    }
    pub fn merge(&mut self, other: &TopK) {
        for s in &other.items {
            self.offer(s.score, &s.sym);
        }
    }
}

/// Steepest descent until no improving swap exists, then (per `ctx.moves`) until no improving
/// column exchange, 3-cycle or swap pair exists either; each improvement found by a larger
/// move resumes the swap descent. Returns evaluations used.
pub fn steepest_descent(ctx: &Ctx, st: &mut State) -> u64 {
    let mut evals = 0;
    loop {
        evals += swap_descent(ctx, st);
        let mv = &ctx.moves;
        if ctx.movable.len() < 3 || !mv.any() {
            return evals;
        }
        let tries: [(bool, moves::MoveFn); 3] =
            [(mv.columns, moves::column_move), (mv.cycle3, moves::cycle3_move), (mv.depth2, moves::depth2_move)];
        let mut improved = false;
        for (on, f) in tries {
            if on {
                let (e, ok) = f(ctx, st);
                evals += e;
                if ok {
                    improved = true;
                    break;
                }
            }
        }
        if !improved {
            return evals;
        }
    }
}

/// Steepest descent over swaps only. Returns evaluations used.
pub fn swap_descent(ctx: &Ctx, st: &mut State) -> u64 {
    steepest_descent_capped(ctx, st, usize::MAX)
}

/// Swap descent with a cap on improving moves.
pub fn steepest_descent_capped(ctx: &Ctx, st: &mut State, max_moves: usize) -> u64 {
    let sc = ctx.scorer;
    let m = &ctx.movable;
    let mut evals = 0;
    for _ in 0..max_moves {
        let mut best = (0, 0, -1e-9);
        for i in 0..m.len() {
            for j in (i + 1)..m.len() {
                let d = sc.delta(st, m[i], m[j]);
                evals += 1;
                if d < best.2 {
                    best = (m[i], m[j], d);
                }
            }
        }
        if best.2 >= -1e-9 {
            return evals;
        }
        sc.apply(st, best.0, best.1, best.2);
    }
    evals
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Progress {
    pub elapsed_ms: u64,
    pub evals: u64,
    pub best: f64,
    /// Best layout so far as slot-ordered chars.
    pub best_chars: String,
    /// Atlas counters during an explore run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explore: Option<ExploreProgress>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ExploreProgress {
    /// `probe` (landscape sampling) or `search`.
    pub phase: String,
    /// Distinct optima in the archive.
    pub optima: u64,
    /// Descents that ended in an archived optimum (incl. repeats).
    pub hits: u64,
    pub edges: u64,
    /// Times the atlas files were rewritten (the web refetches when this changes).
    pub version: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Outcome {
    /// Parameters actually used (auto values resolved).
    pub params: Params,
    pub top: Vec<Scored>,
    pub evals: u64,
    pub elapsed_ms: u64,
    pub evals_per_sec: f64,
    /// Pareto objective metadata and non-dominated candidates (empty for scalar algorithms).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub objectives: Vec<nsga2::Objective>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub front: Vec<nsga2::Point>,
    /// Adaptive fidelity (profile `fidelity`), when the search used it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fidelity: Option<FidelityReport>,
}

/// How an adaptive-fidelity search went (`context/objective.md`, Adaptive fidelity).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct FidelityReport {
    /// Trigrams per level, the last being the full table.
    pub levels: Vec<usize>,
    /// Cheap new bests the full tables confirmed / rejected.
    pub confirmed: u64,
    pub rejected: u64,
    /// The level each worker ended on.
    pub final_level: Vec<usize>,
}

/// Fill in auto parameters (SA steps = 0) from the budget. With an evaluation budget this is
/// deterministic; with a time budget it uses a quick single-thread rate measurement.
fn resolve_auto(ctx: &Ctx, params: &Params) -> Params {
    let mut p = params.clone();
    if p.algo == Algo::Sa && p.sa.steps == 0 {
        let threads = p.threads.max(1) as u64;
        let per_worker = match (p.evals, p.time_ms) {
            (Some(e), _) => e / threads,
            (None, Some(ms)) => {
                let mut rng = Rng::new(p.seed);
                let t = Instant::now();
                let mut sink = 0.0;
                for _ in 0..2000 {
                    let (a, b) = ctx.random_pair(&mut rng);
                    sink += ctx.scorer.delta(&ctx.start, a, b);
                }
                std::hint::black_box(sink);
                let rate = 2000.0 / t.elapsed().as_secs_f64().max(1e-9);
                // Parallel workers share cores, caches and power; assume half the single-thread rate.
                let scale = if threads > 1 { 0.5 } else { 1.0 };
                (rate * scale * ms as f64 / 1000.0) as u64
            }
            (None, None) => 1_000_000,
        };
        p.sa.steps = (per_worker / anneal::AUTO_RESTARTS).max(10_000);
    }
    p
}

/// External control of a running search: cancel (stop early, results kept) and pause.
/// Paused time does not count against a time budget or toward `elapsed_ms`.
#[derive(Default)]
pub struct Control {
    cancel: AtomicBool,
    paused: AtomicBool,
    /// (start of the current pause, total of finished pauses)
    pause: Mutex<(Option<Instant>, Duration)>,
}

impl Control {
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }
    pub fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
    pub fn set_paused(&self, on: bool) {
        let mut p = self.pause.lock().unwrap();
        match (on, p.0) {
            (true, None) => p.0 = Some(Instant::now()),
            (false, Some(t)) => *p = (None, p.1 + t.elapsed()),
            _ => {}
        }
        self.paused.store(on, Ordering::SeqCst);
    }
    pub fn paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }
    /// Total paused time so far, including a pause in progress.
    pub fn paused_for(&self) -> Duration {
        let p = self.pause.lock().unwrap();
        p.1 + p.0.map_or(Duration::ZERO, |t| t.elapsed())
    }
    /// Time since `t0` excluding pauses.
    fn active(&self, t0: Instant) -> Duration {
        t0.elapsed().saturating_sub(self.paused_for())
    }
}

fn make(algo: Algo, p: &Params) -> Box<dyn Optimizer + Send> {
    // Dynamic dispatch is per step (a batch of thousands of evaluations), never per evaluation.
    match algo {
        Algo::Hc => Box::new(hill_climb::HillClimb::new(p.top)),
        Algo::Sa => Box::new(anneal::Anneal::new(p.sa.clone(), p.top)),
        Algo::Ga => Box::new(genetic::Genetic::new(p.ga.clone(), p.top)),
        Algo::Memetic => unreachable!("memetic uses the synchronized island runner"),
        Algo::Bls => Box::new(bls::Bls::new(p.bls.clone(), p.top, false)),
        Algo::Explore => Box::new(bls::Bls::new(p.bls.clone(), p.top, true)),
        Algo::Tabu => Box::new(tabu::Tabu::new(p.tabu.clone(), p.top)),
        Algo::Qd => Box::new(map_elites::MapElites::new(p.qd.clone(), p.top)),
        Algo::Nsga2 | Algo::Rwsa => unreachable!("Pareto algorithms use their population runner"),
    }
}

/// Run `params.threads` workers until the budget ends (independent restarts, or synchronized
/// island generations for memetic). `on_progress` is called
/// from the calling thread about every `interval`. `control` cancels or pauses the run.
pub fn run(ctx: &Ctx, params: &Params, interval: Duration, control: &Control, mut on_progress: impl FnMut(&Progress)) -> Outcome {
    if params.algo == Algo::Memetic {
        return memetic::run(ctx, params, interval, control, on_progress);
    }
    if matches!(params.algo, Algo::Nsga2 | Algo::Rwsa) {
        return nsga2::run(ctx, params, interval, control, on_progress);
    }
    let threads = params.threads.max(1);
    // Adaptive fidelity: coarse scorers with fewer trigrams, then the exact one. Observers
    // (explore, MAP-Elites) record scores, so they always search exactly.
    let fid = ctx.scorer.profile.fidelity.filter(|_| ctx.observer.is_none());
    let coarse: Vec<Scorer> =
        fid.map(|f| f.levels(ctx.scorer.trigram_count()).into_iter().map(|k| ctx.scorer.coarse(k)).collect()).unwrap_or_default();
    let levels: Vec<Ctx> = coarse
        .iter()
        .map(|sc| Ctx { scorer: sc, start: sc.state_from_sym(ctx.start.sym.clone()), ..ctx.clone() })
        .chain([ctx.clone()])
        .map(|c| c.with_moves(params.moves.clone()))
        .collect();
    let params = &resolve_auto(&levels[0], params);
    let ctx = &ctx.clone().with_moves(params.moves.clone());
    let (levels, top_level) = (&levels, levels.len() - 1);
    let t0 = Instant::now();
    let budget = params.time_ms.map(Duration::from_millis);
    let per_worker_evals = params.evals.map(|e| (e / threads as u64).max(1));
    let total_evals = AtomicU64::new(0);
    let (tx, rx) = mpsc::channel::<(usize, f64, Vec<u8>)>();
    let stop = AtomicBool::new(false);

    let mut merged = TopK::new(params.top);
    let level_sizes: Vec<usize> = levels.iter().map(|c| c.scorer.trigram_count()).collect();
    let mut fidelity: Option<FidelityReport> = None;
    std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for w in 0..threads {
            let tx = tx.clone();
            let (total_evals, stop) = (&total_evals, &stop);
            handles.push(scope.spawn(move || {
                let mut rng = crate::rng::Rng::for_worker(params.seed, w as u64);
                let mut opt = make(params.algo, params);
                let mut wctx = levels[0].clone();
                opt.init(&wctx, &mut rng);
                let mut evals = 0u64;
                let mut last_best = f64::INFINITY;
                // Fidelity: level, best full score so far, unconfirmed cheap bests at this level.
                let (mut level, mut confirmed, mut misses) = (0, f64::INFINITY, 0u32);
                let mut exact = TopK::new(params.top);
                let mut report = FidelityReport::default();
                loop {
                    if stop.load(Ordering::Relaxed) || control.cancelled() {
                        break;
                    }
                    if control.paused() {
                        std::thread::sleep(Duration::from_millis(20));
                        continue;
                    }
                    if let Some(b) = budget {
                        // Cheap wall-clock check first; only subtract pauses near the end.
                        if t0.elapsed() >= b && control.active(t0) >= b {
                            break;
                        }
                    }
                    if let Some(e) = per_worker_evals {
                        if evals >= e {
                            break;
                        }
                    }
                    let info = opt.step(&wctx, &mut rng);
                    evals += info.evals;
                    total_evals.fetch_add(info.evals, Ordering::Relaxed);
                    let Some((b, sym)) = opt.live_best() else { continue };
                    if b >= last_best {
                        continue;
                    }
                    last_best = b;
                    if top_level == 0 {
                        let _ = tx.send((w, b, sym.to_vec()));
                        continue;
                    }
                    // Confirm on the full tables before accepting a new best.
                    let s = if level == top_level { b } else { ctx.scorer.state_from_sym(sym.to_vec()).score };
                    if s < confirmed - 1e-12 {
                        confirmed = s;
                        exact.offer(s, sym);
                        report.confirmed += 1;
                        let _ = tx.send((w, s, sym.to_vec()));
                    } else if level < top_level {
                        report.rejected += 1;
                        misses += 1;
                        if misses >= fid.map_or(u32::MAX, |f| f.misses) {
                            // The cheap score misleads here: continue one level up from the best.
                            offer_exact(ctx.scorer, opt.top(), &mut exact);
                            (level, misses, last_best) = (level + 1, 0, f64::INFINITY);
                            let sc = levels[level].scorer;
                            let from = exact.best().map_or_else(|| levels[level].start.sym.clone(), |s| s.sym.clone());
                            wctx = Ctx { start: sc.state_from_sym(from), ..levels[level].clone() };
                            opt = make(params.algo, params);
                            opt.init(&wctx, &mut rng);
                        }
                    }
                }
                opt.finish(&wctx, &mut rng);
                if top_level == 0 {
                    if let Some(b) = opt.top().best() {
                        let _ = tx.send((w, b.score, b.sym.clone()));
                    }
                    return (opt.top().clone(), None);
                }
                // Results are local optima of the exact objective: polish each on the full tables.
                offer_exact(ctx.scorer, opt.top(), &mut exact);
                for s in exact.items.clone() {
                    let mut st = ctx.scorer.state_from_sym(s.sym);
                    let e = steepest_descent(ctx, &mut st);
                    total_evals.fetch_add(e, Ordering::Relaxed);
                    exact.offer(st.score, &st.sym);
                }
                if let Some(b) = exact.best() {
                    let _ = tx.send((w, b.score, b.sym.clone()));
                }
                report.final_level.push(level);
                (exact, Some(report))
            }));
        }
        drop(tx);
        let mut best: Option<(f64, Vec<u8>)> = None;
        let mut last_emit = Instant::now().checked_sub(interval).unwrap_or_else(Instant::now);
        let mut emit = |best: &Option<(f64, Vec<u8>)>, force: bool, last_emit: &mut Instant| {
            if let Some((s, sym)) = best {
                if force || last_emit.elapsed() >= interval {
                    *last_emit = Instant::now();
                    on_progress(&Progress {
                        elapsed_ms: control.active(t0).as_millis() as u64,
                        evals: total_evals.load(Ordering::Relaxed),
                        best: *s,
                        best_chars: sym.iter().map(|&x| ctx.scorer.chars[x as usize]).collect(),
                        explore: None,
                    });
                }
            }
        };
        loop {
            match rx.recv_timeout(interval) {
                Ok((_w, s, sym)) => {
                    if best.as_ref().is_none_or(|b| s < b.0) {
                        best = Some((s, sym));
                    }
                    emit(&best, false, &mut last_emit);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => emit(&best, true, &mut last_emit),
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        for h in handles {
            if let Ok((t, r)) = h.join() {
                merged.merge(&t);
                if let Some(r) = r {
                    let f = fidelity.get_or_insert_with(|| FidelityReport { levels: level_sizes.clone(), ..Default::default() });
                    f.confirmed += r.confirmed;
                    f.rejected += r.rejected;
                    f.final_level.extend(r.final_level);
                }
            }
        }
        stop.store(true, Ordering::Relaxed);
        emit(&best, true, &mut last_emit);
    });

    // Re-score from scratch so reported scores never carry accumulated delta drift.
    let mut top: Vec<Scored> = merged
        .items
        .into_iter()
        .map(|s| {
            let st = ctx.scorer.state_from_sym(s.sym);
            Scored { score: st.score, sym: st.sym }
        })
        .collect();
    top.sort_by(|a, b| a.score.total_cmp(&b.score));
    let elapsed = control.active(t0);
    let evals = total_evals.load(Ordering::Relaxed);
    Outcome {
        params: params.clone(),
        top,
        evals,
        elapsed_ms: elapsed.as_millis() as u64,
        evals_per_sec: evals as f64 / elapsed.as_secs_f64().max(1e-9),
        objectives: Vec::new(),
        front: Vec::new(),
        fidelity,
    }
}

/// Offer `top` (scored by a coarse scorer) to `exact` with full scores.
fn offer_exact(full: &Scorer, top: &TopK, exact: &mut TopK) {
    for s in &top.items {
        exact.offer(full.state_from_sym(s.sym.clone()).score, &s.sym);
    }
}
