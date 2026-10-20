//! Simulated annealing with exponential cooling, data-driven T0, restarts and a
//! steepest-descent polish. `context/algorithms/simulated-annealing.md`.

use serde::{Deserialize, Serialize};

use super::{steepest_descent, Ctx, Optimizer, StepInfo, TopK};
use crate::rng::Rng;
use crate::scorer::State;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SaParams {
    /// Swap attempts per restart (the length of one cooling schedule). 0 = auto: about
    /// `AUTO_RESTARTS` restarts per worker within the run's budget.
    pub steps: u64,
    /// Target acceptance probability of an average worsening swap at the start.
    pub accept0: f64,
    /// T_end / T0.
    pub t_end_ratio: f64,
}

impl Default for SaParams {
    fn default() -> Self {
        SaParams { steps: 0, accept0: 0.5, t_end_ratio: 2e-3 }
    }
}

const BATCH: u64 = 20_000;
/// Target restarts per worker when `steps` is auto.
pub const AUTO_RESTARTS: u64 = 4;

pub struct Anneal {
    p: SaParams,
    top: TopK,
    t0: f64,
    cur: Option<State>,
    best: Option<State>,
    i: u64,
    restarts: u64,
}

impl Anneal {
    pub fn new(p: SaParams, top: usize) -> Self {
        Anneal { p, top: TopK::new(top), t0: 1.0, cur: None, best: None, i: 0, restarts: 0 }
    }

    fn end_restart(&mut self, ctx: &Ctx) -> u64 {
        let mut evals = 0;
        if let Some(mut b) = self.best.take() {
            evals += steepest_descent(ctx, &mut b);
            self.top.offer(b.score, &b.sym);
        }
        self.cur = None;
        evals
    }
}

impl Optimizer for Anneal {
    fn init(&mut self, ctx: &Ctx, rng: &mut Rng) {
        // T0 from data: mean worsening of random swaps on random layouts.
        let (mut sum, mut cnt) = (0.0, 0u32);
        for _ in 0..8 {
            let st = ctx.random_state(rng);
            for _ in 0..64 {
                let (a, b) = ctx.random_pair(rng);
                let d = ctx.scorer.delta(&st, a, b);
                if d > 0.0 {
                    sum += d;
                    cnt += 1;
                }
            }
        }
        let mean = if cnt > 0 { sum / cnt as f64 } else { 1.0 };
        self.t0 = mean / (1.0 / self.p.accept0.clamp(1e-6, 0.999)).ln();
    }

    fn step(&mut self, ctx: &Ctx, rng: &mut Rng) -> StepInfo {
        if ctx.movable.len() < 2 {
            self.top.offer(ctx.start.score, &ctx.start.sym);
            return StepInfo { evals: BATCH };
        }
        let mut evals = 0;
        if self.cur.is_none() {
            let st = if self.restarts == 0 { ctx.start.clone() } else { ctx.random_state(rng) };
            self.best = Some(st.clone());
            self.cur = Some(st);
            self.i = 0;
            self.restarts += 1;
        }
        let sc = ctx.scorer;
        let steps = self.p.steps.max(1);
        let ln_ratio = self.p.t_end_ratio.max(1e-12).ln();
        let cur = self.cur.as_mut().unwrap();
        let best = self.best.as_mut().unwrap();
        let start_i = self.i;
        let end = (self.i + BATCH).min(steps);
        while self.i < end {
            // Temperature updated every 256 steps; exp() is the costly part.
            let t = self.t0 * (ln_ratio * self.i as f64 / steps as f64).exp();
            let chunk_end = (self.i + 256).min(end);
            while self.i < chunk_end {
                let (a, b) = ctx.random_pair(rng);
                let d = sc.delta(cur, a, b);
                if d <= 0.0 || rng.f64() < (-d / t).exp() {
                    sc.apply(cur, a, b, d);
                    if cur.score < best.score - 1e-12 {
                        best.clone_from(cur);
                    }
                }
                self.i += 1;
            }
        }
        evals += end - start_i;
        if self.i >= steps {
            evals += self.end_restart(ctx);
        }
        StepInfo { evals }
    }

    fn finish(&mut self, ctx: &Ctx, _rng: &mut Rng) {
        self.end_restart(ctx);
    }

    fn top(&self) -> &TopK {
        &self.top
    }

    fn live_best(&self) -> Option<(f64, &[u8])> {
        let t = self.top.best().map(|s| (s.score, s.sym.as_slice()));
        match (&self.best, t) {
            (Some(b), Some(t)) if b.score < t.0 => Some((b.score, b.sym.as_slice())),
            (Some(b), None) => Some((b.score, b.sym.as_slice())),
            (_, t) => t,
        }
    }
}
