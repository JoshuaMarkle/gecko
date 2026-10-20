//! Discrete layout optimizers and the multi-threaded runner. Algorithms contain only search
//! logic; the runner owns budgets, threads, progress and top-K. See `context/algorithms/`.

pub mod anneal;
pub mod genetic;
pub mod hill_climb;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Mutex};
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
}

impl Algo {
    pub const ALL: [Algo; 3] = [Algo::Hc, Algo::Sa, Algo::Ga];
    pub fn name(self) -> &'static str {
        match self {
            Algo::Hc => "hc",
            Algo::Sa => "sa",
            Algo::Ga => "ga",
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
    /// Total evaluation budget, split evenly across workers. Deterministic, unlike time.
    pub evals: Option<u64>,
    /// Number of distinct best layouts to keep.
    pub top: usize,
    #[serde(default)]
    pub sa: anneal::SaParams,
    #[serde(default)]
    pub ga: genetic::GaParams,
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
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Scored {
    pub score: f64,
    /// `sym[slot]` in the scorer's symbol ids.
    pub sym: Vec<u8>,
}

/// Search context shared by all workers (read-only).
pub struct Ctx<'a> {
    pub scorer: &'a Scorer,
    /// Symbols allowed to move (not on a pinned slot).
    pub movable: Vec<usize>,
    pub start: State,
}

impl Ctx<'_> {
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

/// Steepest descent until no improving swap exists. Returns evaluations used.
pub fn steepest_descent(ctx: &Ctx, st: &mut State) -> u64 {
    let sc = ctx.scorer;
    let m = &ctx.movable;
    let mut evals = 0;
    loop {
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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Progress {
    pub elapsed_ms: u64,
    pub evals: u64,
    pub best: f64,
    /// Best layout so far as slot-ordered chars.
    pub best_chars: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Outcome {
    /// Parameters actually used (auto values resolved).
    pub params: Params,
    pub top: Vec<Scored>,
    pub evals: u64,
    pub elapsed_ms: u64,
    pub evals_per_sec: f64,
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
    }
}

/// Run `params.threads` independent workers until the budget ends. `on_progress` is called
/// from the calling thread about every `interval`. `control` cancels or pauses the run.
pub fn run(ctx: &Ctx, params: &Params, interval: Duration, control: &Control, mut on_progress: impl FnMut(&Progress)) -> Outcome {
    let threads = params.threads.max(1);
    let params = &resolve_auto(ctx, params);
    let t0 = Instant::now();
    let budget = params.time_ms.map(Duration::from_millis);
    let per_worker_evals = params.evals.map(|e| (e / threads as u64).max(1));
    let total_evals = AtomicU64::new(0);
    let (tx, rx) = mpsc::channel::<(usize, f64, Vec<u8>)>();
    let stop = AtomicBool::new(false);

    let mut merged = TopK::new(params.top);
    std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for w in 0..threads {
            let tx = tx.clone();
            let (total_evals, stop) = (&total_evals, &stop);
            handles.push(scope.spawn(move || {
                let mut rng = crate::rng::Rng::for_worker(params.seed, w as u64);
                let mut opt = make(params.algo, params);
                opt.init(ctx, &mut rng);
                let mut evals = 0u64;
                let mut last_best = f64::INFINITY;
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
                    let info = opt.step(ctx, &mut rng);
                    evals += info.evals;
                    total_evals.fetch_add(info.evals, Ordering::Relaxed);
                    if let Some((b, sym)) = opt.live_best() {
                        if b < last_best {
                            last_best = b;
                            let _ = tx.send((w, b, sym.to_vec()));
                        }
                    }
                }
                opt.finish(ctx, &mut rng);
                if let Some(b) = opt.top().best() {
                    let _ = tx.send((w, b.score, b.sym.clone()));
                }
                opt.top().clone()
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
            if let Ok(t) = h.join() {
                merged.merge(&t);
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
    Outcome { params: params.clone(), top, evals, elapsed_ms: elapsed.as_millis() as u64, evals_per_sec: evals as f64 / elapsed.as_secs_f64().max(1e-9) }
}
