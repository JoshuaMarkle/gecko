//! Steepest-descent hill climbing with random restarts. `context/algorithms/hill-climbing.md`.

use super::{steepest_descent, Ctx, Optimizer, StepInfo, TopK};
use crate::rng::Rng;

pub struct HillClimb {
    top: TopK,
    first: bool,
}

impl HillClimb {
    pub fn new(top: usize) -> Self {
        HillClimb { top: TopK::new(top), first: true }
    }
}

impl Optimizer for HillClimb {
    fn init(&mut self, _ctx: &Ctx, _rng: &mut Rng) {
        self.first = true;
    }

    /// One restart: a random layout (the start layout on the first restart) descended to a local optimum.
    fn step(&mut self, ctx: &Ctx, rng: &mut Rng) -> StepInfo {
        let mut st = if std::mem::take(&mut self.first) { ctx.start.clone() } else { ctx.random_state(rng) };
        let evals = steepest_descent(ctx, &mut st);
        self.top.offer(st.score, &st.sym);
        StepInfo { evals: evals + 1 }
    }

    fn finish(&mut self, _ctx: &Ctx, _rng: &mut Rng) {}

    fn top(&self) -> &TopK {
        &self.top
    }
}
