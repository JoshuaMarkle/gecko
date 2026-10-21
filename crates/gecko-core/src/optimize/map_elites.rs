//! MAP-Elites worker: mutate a random elite (random swaps + short steepest descent) and offer
//! the result to the grid, which lives in `crate::explore::qd` and is reached through the
//! `Observer` in `Ctx`. `context/algorithms/map-elites.md`.

use serde::{Deserialize, Serialize};

use super::{steepest_descent_capped, Ctx, Optimizer, StepInfo, TopK};
use crate::rng::Rng;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct QdParams {
    /// Behaviour axes, each `name` or `name:lo:hi` (`explore::qd::DESCRIPTORS` or a metric id).
    /// Axes without a range get one from the seed descents.
    pub axes: Vec<String>,
    /// Bins per axis.
    pub bins: usize,
    /// Random descents that seed the grid (and set automatic axis ranges).
    pub seeds: usize,
    /// Mutation: a uniform number of random swaps in `kick_min..=kick_max`.
    pub kick_min: usize,
    pub kick_max: usize,
    /// Improving moves of the steepest descent after a kick; 0 = descend to a local optimum.
    pub descent: usize,
    /// How often `qd.json` / `elites.bin` are rewritten during the run.
    pub snapshot_ms: u64,
}

impl Default for QdParams {
    fn default() -> Self {
        QdParams {
            axes: vec!["roll_ratio".into(), "index_load".into()],
            bins: 16,
            seeds: 64,
            kick_min: 2,
            kick_max: 6,
            descent: 0,
            snapshot_ms: 2000,
        }
    }
}

/// MAP-Elites worker: mutate a random elite (random swaps + short descent) and offer the result
/// to the grid (`ctx.observer`). Without a grid (or before it has elites) it does random
/// restarts with a full descent.
pub struct MapElites {
    p: QdParams,
    top: TopK,
}

impl MapElites {
    pub fn new(p: QdParams, top: usize) -> Self {
        MapElites { p, top: TopK::new(top) }
    }
}

impl Optimizer for MapElites {
    fn init(&mut self, _ctx: &Ctx, _rng: &mut Rng) {}

    fn step(&mut self, ctx: &Ctx, rng: &mut Rng) -> StepInfo {
        let sc = ctx.scorer;
        if ctx.movable.len() < 2 {
            self.top.offer(ctx.start.score, &ctx.start.sym);
            return StepInfo { evals: 1000 };
        }
        let mut evals = 1;
        let st = match ctx.observer.and_then(|o| o.parent(rng)) {
            Some(sym) => {
                let mut st = sc.state_from_sym(sym);
                let (lo, hi) = (self.p.kick_min.max(1), self.p.kick_max.max(self.p.kick_min.max(1)));
                for _ in 0..lo + rng.below(hi - lo + 1) {
                    let (a, b) = ctx.random_pair(rng);
                    let d = sc.delta(&st, a, b);
                    sc.apply(&mut st, a, b, d);
                    evals += 1;
                }
                let cap = if self.p.descent == 0 { usize::MAX } else { self.p.descent };
                evals += steepest_descent_capped(ctx, &mut st, cap);
                st
            }
            None => {
                let mut st = ctx.random_state(rng);
                evals += steepest_descent_capped(ctx, &mut st, usize::MAX);
                st
            }
        };
        if let Some(o) = ctx.observer {
            o.optimum(&st, false);
        }
        self.top.offer(st.score, &st.sym);
        StepInfo { evals }
    }

    fn finish(&mut self, _ctx: &Ctx, _rng: &mut Rng) {}

    fn top(&self) -> &TopK {
        &self.top
    }
}
