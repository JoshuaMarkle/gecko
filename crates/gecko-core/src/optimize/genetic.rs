//! Genetic algorithm over permutations: tournament selection, order crossover (OX) on the
//! movable slots, swap mutation, elitism, duplicate rejection. `context/algorithms/genetic.md`.

use serde::{Deserialize, Serialize};

use super::{steepest_descent, Ctx, Optimizer, StepInfo, TopK};
use crate::rng::Rng;
use crate::scorer::State;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct GaParams {
    pub pop_size: usize,
    pub tournament: usize,
    pub elitism: usize,
    pub crossover_rate: f64,
    /// Expected number of swap mutations per child.
    pub mutation_rate: f64,
}

impl Default for GaParams {
    fn default() -> Self {
        GaParams { pop_size: 128, tournament: 3, elitism: 4, crossover_rate: 0.9, mutation_rate: 1.0 }
    }
}

pub struct Genetic {
    p: GaParams,
    top: TopK,
    pop: Vec<State>,
    slots: Vec<usize>,
}

impl Genetic {
    pub fn new(p: GaParams, top: usize) -> Self {
        Genetic { p, top: TopK::new(top), pop: Vec::new(), slots: Vec::new() }
    }
}

pub(super) fn pick<'a>(pop: &'a [State], p: &GaParams, rng: &mut Rng) -> &'a State {
    let mut best = &pop[rng.below(pop.len())];
    for _ in 1..p.tournament.max(1) {
        let c = &pop[rng.below(pop.len())];
        if c.score < best.score {
            best = c;
        }
    }
    best
}

/// Order crossover restricted to the movable slots; pinned slots copy from parent 1.
pub(super) fn crossover(slots: &[usize], a: &State, b: &State, rng: &mut Rng) -> Vec<u8> {
    let m = slots.len();
    let mut child = a.sym.clone();
    let (mut i, mut j) = (rng.below(m), rng.below(m));
    if i > j {
        std::mem::swap(&mut i, &mut j);
    }
    let mut used = [false; crate::scorer::MAX_KEYS];
    for &s in &slots[i..=j] {
        used[a.sym[s] as usize] = true;
    }
    let mut fill = slots.iter().map(|&s| b.sym[s]).filter(|&x| !used[x as usize]);
    for (k, &s) in slots.iter().enumerate() {
        if k < i || k > j {
            child[s] = fill.next().expect("OX fill");
        }
    }
    child
}

impl Optimizer for Genetic {
    fn init(&mut self, ctx: &Ctx, rng: &mut Rng) {
        let mut slots: Vec<usize> = ctx.movable.iter().map(|&s| ctx.start.pos[s] as usize).collect();
        slots.sort_unstable();
        self.slots = slots;
        self.pop.clear();
        self.pop.push(ctx.start.clone());
        while self.pop.len() < self.p.pop_size.max(4) {
            self.pop.push(ctx.random_state(rng));
        }
        for s in &self.pop {
            self.top.offer(s.score, &s.sym);
        }
    }

    /// One generation.
    fn step(&mut self, ctx: &Ctx, rng: &mut Rng) -> StepInfo {
        if self.slots.len() < 2 {
            return StepInfo { evals: self.pop.len() as u64 };
        }
        let sc = ctx.scorer;
        self.pop.sort_by(|a, b| a.score.total_cmp(&b.score));
        let n = self.pop.len();
        let mut next: Vec<State> = self.pop[..self.p.elitism.min(n)].to_vec();
        let mut evals = 0u64;
        let mut tries = 0;
        while next.len() < n && tries < n * 20 {
            tries += 1;
            let sym = offspring(&self.pop, &self.slots, &self.p, rng);
            if next.iter().any(|s| s.sym == sym) {
                continue;
            }
            let st = sc.state_from_sym(sym);
            evals += 1;
            self.top.offer(st.score, &st.sym);
            next.push(st);
        }
        self.pop = next;
        StepInfo { evals }
    }

    fn finish(&mut self, ctx: &Ctx, _rng: &mut Rng) {
        // Polish the best few so GA results are local optima, comparable with other algorithms.
        let best: Vec<_> = self.top.items.iter().take(3).map(|s| s.sym.clone()).collect();
        for sym in best {
            let mut st = ctx.scorer.state_from_sym(sym);
            steepest_descent(ctx, &mut st);
            self.top.offer(st.score, &st.sym);
        }
    }

    fn top(&self) -> &TopK {
        &self.top
    }
}

/// Shared permutation operators for GA and memetic search. Called between local searches.
pub(super) fn offspring(pop: &[State], slots: &[usize], p: &GaParams, rng: &mut Rng) -> Vec<u8> {
    let a = pick(pop, p, rng);
    let mut sym = if rng.f64() < p.crossover_rate { crossover(slots, a, pick(pop, p, rng), rng) } else { a.sym.clone() };
    mutate(slots, &mut sym, &a.sym, p.mutation_rate, rng);
    sym
}

/// Shared swap mutation restricted to movable slots.
pub(super) fn mutate(slots: &[usize], sym: &mut [u8], parent: &[u8], mutation_rate: f64, rng: &mut Rng) {
    if slots.len() < 2 { return; }
    let mut swaps = 0;
    while rng.f64() < mutation_rate / (1.0 + mutation_rate) {
        swaps += 1;
    }
    if swaps == 0 && sym == parent {
        swaps = 1;
    }
    for _ in 0..swaps {
        let (i, j) = (slots[rng.below(slots.len())], slots[rng.below(slots.len())]);
        sym.swap(i, j);
    }
}
