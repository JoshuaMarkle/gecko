//! Robust tabu search (Taillard 1991): every iteration takes the best non-tabu swap, even a
//! worsening one; moving a symbol back to a slot it recently left is tabu for a randomized
//! tenure. Aspiration allows a tabu move that gives a new best; a move unused for a long time
//! is forced (diversification). The full Δ matrix of all swaps is kept and updated
//! incrementally (`Scorer::delta_shift`). `context/algorithms/tabu-search.md`.

use serde::{Deserialize, Serialize};

use super::{steepest_descent, Ctx, Optimizer, StepInfo, TopK};
use crate::rng::Rng;
use crate::scorer::State;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct TabuParams {
    /// Tenure range as fractions of the movable symbol count (Taillard: 0.9–1.1).
    pub tenure_min: f64,
    pub tenure_max: f64,
    /// Iterations a (symbol, slot) placement may go unused before it is forced, as a multiple
    /// of m² (m = movable symbols). Taillard uses 5. 0 disables diversification.
    pub diversify: f64,
    /// Restart from a random layout after this many iterations without improving the
    /// restart's best, as a multiple of m. 0 = never restart.
    pub stagnation: f64,
    /// Recompute the Δ matrix and score from scratch every this many iterations (drift guard).
    pub refresh: u32,
}

impl Default for TabuParams {
    fn default() -> Self {
        TabuParams { tenure_min: 0.9, tenure_max: 1.1, diversify: 5.0, stagnation: 100.0, refresh: 1000 }
    }
}

/// Iterations per `step` (each is one scan of all m(m-1)/2 swaps).
const BATCH: u32 = 32;

pub struct Tabu {
    p: TabuParams,
    top: TopK,
    cur: Option<State>,
    /// Best state since the last restart, and the worker's best overall (for progress).
    best: Option<State>,
    best_score: f64,
    best_sym: Vec<u8>,
    /// Upper triangle by movable index: `lin[i * m + j]` = `delta_linear(movable[i], movable[j])`.
    lin: Vec<f64>,
    /// `tabu[sym * n + slot]`: iteration until which moving `sym` to `slot` is tabu.
    tabu: Vec<u64>,
    iter: u64,
    since_refresh: u32,
    since_improve: u64,
    restarts: u64,
}

impl Tabu {
    pub fn new(p: TabuParams, top: usize) -> Self {
        Tabu {
            p,
            top: TopK::new(top),
            cur: None,
            best: None,
            best_score: f64::INFINITY,
            best_sym: Vec::new(),
            lin: Vec::new(),
            tabu: Vec::new(),
            iter: 0,
            since_refresh: 0,
            since_improve: 0,
            restarts: 0,
        }
    }

    fn restart(&mut self, ctx: &Ctx, rng: &mut Rng) {
        let st = if self.restarts == 0 { ctx.start.clone() } else { ctx.random_state(rng) };
        self.restarts += 1;
        let m = ctx.movable.len();
        self.lin.resize(m * m, 0.0);
        self.tabu.clear();
        self.tabu.resize(ctx.scorer.n * ctx.scorer.n, 0);
        self.iter = 0;
        self.since_refresh = 0;
        self.since_improve = 0;
        rebuild(&mut self.lin, ctx, &st);
        self.best = Some(st.clone());
        self.cur = Some(st);
    }

    /// One iteration: scan all swaps, take the best allowed one, update the Δ matrix.
    /// Returns evaluations (swaps looked at).
    fn iterate(&mut self, ctx: &Ctx, rng: &mut Rng) -> u64 {
        let sc = ctx.scorer;
        let mv = &ctx.movable;
        let m = mv.len();
        let n = sc.n;
        let cur = self.cur.as_mut().unwrap();
        let best = self.best.as_mut().unwrap();
        let it = self.iter;
        let div = (self.p.diversify * (m * m) as f64) as u64;
        // (i, j, delta) of the chosen move, and the best delta of any move (local-optimum test).
        let mut pick = (0, 0, f64::INFINITY);
        let mut aspired_pick = false;
        let mut min_any = f64::INFINITY;
        for i in 0..m {
            let (a, pa) = (mv[i], cur.pos[mv[i]] as usize);
            for j in (i + 1)..m {
                let (b, pb) = (mv[j], cur.pos[mv[j]] as usize);
                let d = self.lin[i * m + j] + sc.delta_load(cur, a, b);
                min_any = min_any.min(d);
                let (ta, tb) = (self.tabu[a * n + pb], self.tabu[b * n + pa]);
                // Tabu only if both symbols would return to recently left slots (Taillard).
                let allowed = ta <= it || tb <= it;
                // Aspiration: a new best, or both placements unused for a long time.
                let aspired = cur.score + d < best.score - 1e-9 || (div > 0 && ta + div < it && tb + div < it);
                // Aspired moves beat all others; among the rest only allowed ones count.
                let take = if aspired { !aspired_pick || d < pick.2 } else { !aspired_pick && allowed && d < pick.2 };
                if take {
                    pick = (i, j, d);
                    aspired_pick |= aspired;
                }
            }
        }
        let evals = (m * (m - 1) / 2) as u64;
        if min_any >= -1e-9 {
            // A local optimum: worth keeping.
            self.top.offer(cur.score, &cur.sym);
        }
        if pick.2 == f64::INFINITY {
            // Every move is tabu and none aspires: take a random one.
            let (mut i, mut j) = (rng.below(m), rng.below(m - 1));
            if j >= i {
                j += 1;
            }
            if i > j {
                std::mem::swap(&mut i, &mut j);
            }
            pick = (i, j, self.lin[i * m + j] + sc.delta_load(cur, mv[i], mv[j]));
        }
        let (i, j, d) = pick;
        let (r, s) = (mv[i], mv[j]);
        let (pr, ps) = (cur.pos[r] as usize, cur.pos[s] as usize);
        let span = (self.p.tenure_max - self.p.tenure_min).max(0.0) * m as f64;
        let mut tenure = || (self.p.tenure_min * m as f64 + rng.f64() * span).round().max(1.0) as u64;
        self.tabu[r * n + pr] = it + tenure();
        self.tabu[s * n + ps] = it + tenure();
        sc.apply(cur, r, s, d);
        self.iter += 1;

        // Δ matrix: rows/columns of r and s from scratch, everything else incrementally.
        self.since_refresh += 1;
        if self.since_refresh >= self.p.refresh.max(1) {
            self.since_refresh = 0;
            cur.score = sc.full(cur);
            rebuild(&mut self.lin, ctx, cur);
        } else {
            for x in 0..m {
                let u = mv[x];
                for y in (x + 1)..m {
                    let v = mv[y];
                    let k = x * m + y;
                    if x == i || x == j || y == i || y == j {
                        self.lin[k] = sc.delta_linear(cur, u, v);
                    } else {
                        self.lin[k] += sc.delta_shift(cur, r, s, u, v);
                    }
                }
            }
        }

        if cur.score < best.score - 1e-9 {
            best.clone_from(cur);
            self.since_improve = 0;
            if best.score < self.best_score {
                self.best_score = best.score;
                self.best_sym.clone_from(&best.sym);
            }
        } else {
            self.since_improve += 1;
        }
        evals
    }
}

impl Optimizer for Tabu {
    fn init(&mut self, _ctx: &Ctx, _rng: &mut Rng) {}

    /// A batch of tabu iterations (restarting when stagnant).
    fn step(&mut self, ctx: &Ctx, rng: &mut Rng) -> StepInfo {
        let m = ctx.movable.len();
        if m < 2 {
            self.top.offer(ctx.start.score, &ctx.start.sym);
            return StepInfo { evals: 1000 };
        }
        let stagnation = (self.p.stagnation * m as f64) as u64;
        let mut evals = 0;
        for _ in 0..BATCH {
            if self.cur.is_none() || (stagnation > 0 && self.since_improve >= stagnation) {
                evals += self.finish_restart(ctx);
                self.restart(ctx, rng);
            }
            evals += self.iterate(ctx, rng);
        }
        StepInfo { evals }
    }

    fn finish(&mut self, ctx: &Ctx, _rng: &mut Rng) {
        self.finish_restart(ctx);
    }

    fn top(&self) -> &TopK {
        &self.top
    }

    fn live_best(&self) -> Option<(f64, &[u8])> {
        let t = self.top.best().map(|s| (s.score, s.sym.as_slice()));
        match t {
            Some(t) if t.0 <= self.best_score => Some(t),
            _ if self.best_score.is_finite() => Some((self.best_score, self.best_sym.as_slice())),
            t => t,
        }
    }
}

impl Tabu {
    /// Polish the restart's best into a local optimum and keep it.
    fn finish_restart(&mut self, ctx: &Ctx) -> u64 {
        let Some(mut b) = self.best.take() else { return 0 };
        let evals = steepest_descent(ctx, &mut b);
        self.top.offer(b.score, &b.sym);
        evals
    }
}

/// Fill the Δ matrix (`delta_linear` of every movable pair) from scratch.
fn rebuild(lin: &mut [f64], ctx: &Ctx, st: &State) {
    let m = &ctx.movable;
    for i in 0..m.len() {
        for j in (i + 1)..m.len() {
            lin[i * m.len() + j] = ctx.scorer.delta_linear(st, m[i], m[j]);
        }
    }
}
