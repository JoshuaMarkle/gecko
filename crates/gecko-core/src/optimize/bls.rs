//! Breakout local search (Benlic & Hao): steepest descent, then kick the local optimum with an
//! adaptive number of random swaps (or a column exchange) and descend again. Every kick is an
//! edge between two local optima, so with an [`Observer`](super::Observer) this samples the local optima network.
//! In explore mode, some restarts first descend under a distance constraint that keeps them away
//! from the best distinct layouts found so far. `context/algorithms/explore.md`.

use serde::{Deserialize, Serialize};

use super::{steepest_descent, Ctx, Kick, Optimizer, StepInfo, TopK};
use crate::rng::Rng;
use crate::scorer::State;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct BlsParams {
    /// Kick strength (random swaps) after an improvement or a move to a new optimum.
    pub kick_min: usize,
    /// Strength past which the worker restarts. 0 = auto (a third of the movable symbols).
    pub kick_max: usize,
    /// Probability of moving to a worse optimum found by a kick (diversification).
    pub accept_worse: f64,
    /// Restart after this many kicks without improving the worker's best since its restart.
    pub stagnation: u32,
    /// Share of kicks that exchange two whole columns instead of random swaps.
    pub column_kick: f64,
    /// Explore mode: share of restarts that use an exclusion descent.
    pub exclusion: f64,
    /// Explore mode: minimum Hamming distance (slots that differ) from every excluded layout.
    pub exclusion_dist: usize,
}

impl Default for BlsParams {
    fn default() -> Self {
        BlsParams { kick_min: 2, kick_max: 0, accept_worse: 0.1, stagnation: 150, column_kick: 0.2, exclusion: 0.5, exclusion_dist: 8 }
    }
}

pub struct Bls {
    p: BlsParams,
    top: TopK,
    explore: bool,
    cur: Option<State>,
    cur_id: u32,
    next: Option<State>,
    best_since_restart: f64,
    k: usize,
    k_max: usize,
    stale: u32,
    restarts: u64,
    /// Movable slots grouped by geometry column (only groups with a same-size partner).
    columns: Vec<Vec<usize>>,
}

impl Bls {
    pub fn new(p: BlsParams, top: usize, explore: bool) -> Self {
        Bls {
            p,
            top: TopK::new(top),
            explore,
            cur: None,
            cur_id: 0,
            next: None,
            best_since_restart: f64::INFINITY,
            k: 2,
            k_max: 10,
            stale: 0,
            restarts: 0,
            columns: Vec::new(),
        }
    }

    fn record(&self, ctx: &Ctx, st: &State, constrained: bool) -> u32 {
        ctx.observer.map_or(0, |o| o.optimum(st, constrained))
    }

    fn restart(&mut self, ctx: &Ctx, rng: &mut Rng) -> u64 {
        let mut evals = 0;
        let first = self.restarts == 0;
        self.restarts += 1;
        let mut st = if first { ctx.start.clone() } else { ctx.random_state(rng) };
        let excluding = match ctx.observer {
            Some(o) if !first && self.explore && rng.f64() < self.p.exclusion => Some(o.exclusions()),
            _ => None,
        };
        if let Some(ex) = excluding {
            if !ex.is_empty() {
                // Start next to a good layout: kick one of them until it is far enough from all.
                let dmin = self.p.exclusion_dist;
                st = ctx.scorer.state_from_sym(ex[rng.below(ex.len())].clone());
                let far = |st: &State| ex.iter().all(|e| crate::explore::distance::hamming(e, &st.sym) >= dmin);
                for _ in 0..4 * dmin {
                    if far(&st) {
                        break;
                    }
                    let (a, b) = ctx.random_pair(rng);
                    let d = ctx.scorer.delta(&st, a, b);
                    ctx.scorer.apply(&mut st, a, b, d);
                    evals += 1;
                }
                evals += constrained_descent(ctx, &mut st, &ex, dmin);
                self.record(ctx, &st, true);
                self.top.offer(st.score, &st.sym);
            }
        }
        evals += steepest_descent(ctx, &mut st);
        self.cur_id = self.record(ctx, &st, false);
        self.top.offer(st.score, &st.sym);
        self.best_since_restart = st.score;
        self.k = self.p.kick_min.max(1);
        self.stale = 0;
        self.cur = Some(st);
        evals
    }

    /// Apply a kick to `st`; returns its kind and strength and the evaluations used.
    fn kick(&self, ctx: &Ctx, st: &mut State, rng: &mut Rng) -> (Kick, u8, u64) {
        let sc = ctx.scorer;
        if !self.columns.is_empty() && rng.f64() < self.p.column_kick {
            let a = &self.columns[rng.below(self.columns.len())];
            let partners: Vec<&Vec<usize>> = self.columns.iter().filter(|c| c.len() == a.len() && c[0] != a[0]).collect();
            if !partners.is_empty() {
                let b = partners[rng.below(partners.len())];
                for (&sa, &sb) in a.iter().zip(b.iter()) {
                    let (x, y) = (st.sym[sa] as usize, st.sym[sb] as usize);
                    let d = sc.delta(st, x, y);
                    sc.apply(st, x, y, d);
                }
                return (Kick::Columns, a.len() as u8, a.len() as u64);
            }
        }
        for _ in 0..self.k {
            let (a, b) = ctx.random_pair(rng);
            let d = sc.delta(st, a, b);
            sc.apply(st, a, b, d);
        }
        (Kick::Swaps, self.k.min(255) as u8, self.k as u64)
    }
}

impl Optimizer for Bls {
    fn init(&mut self, ctx: &Ctx, _rng: &mut Rng) {
        let m = ctx.movable.len();
        self.k_max = if self.p.kick_max > 0 { self.p.kick_max } else { (m / 3).max(self.p.kick_min + 1) };
        let movable_slot: Vec<bool> = {
            let mut v = vec![false; ctx.scorer.n];
            for &s in &ctx.movable {
                v[ctx.start.pos[s] as usize] = true;
            }
            v
        };
        let g = &ctx.scorer.geometry;
        let mut cols: Vec<(i8, bool, Vec<usize>)> = Vec::new();
        for (slot, k) in g.key.iter().enumerate() {
            match cols.iter_mut().find(|c| c.0 == k.col) {
                Some(c) => {
                    c.1 &= movable_slot[slot];
                    c.2.push(slot);
                }
                None => cols.push((k.col, movable_slot[slot], vec![slot])),
            }
        }
        let groups: Vec<Vec<usize>> = cols.into_iter().filter(|c| c.1 && c.2.len() > 1).map(|c| c.2).collect();
        self.columns = groups.iter().filter(|a| groups.iter().filter(|b| b.len() == a.len()).count() > 1).cloned().collect();
        for c in &mut self.columns {
            c.sort_by_key(|&s| g.key[s].row);
        }
    }

    /// One restart, or one kick + descent from the current optimum.
    fn step(&mut self, ctx: &Ctx, rng: &mut Rng) -> StepInfo {
        if ctx.movable.len() < 2 {
            self.top.offer(ctx.start.score, &ctx.start.sym);
            return StepInfo { evals: 1000 };
        }
        let Some(cur) = self.cur.as_ref() else {
            return StepInfo { evals: self.restart(ctx, rng) + 1 };
        };
        let mut nx = self.next.take().unwrap_or_else(|| cur.clone());
        nx.clone_from(cur);
        let (kick, strength, mut evals) = self.kick(ctx, &mut nx, rng);
        evals += steepest_descent(ctx, &mut nx);
        let id = self.record(ctx, &nx, false);
        if let Some(o) = ctx.observer {
            o.edge(self.cur_id, id, kick, strength);
        }
        self.top.offer(nx.score, &nx.sym);
        let cur = self.cur.as_mut().unwrap();
        if nx.sym == cur.sym {
            // Fell back into the same basin: kick harder.
            self.k += 1;
            if self.k > self.k_max {
                self.cur = None;
            }
        } else if nx.score < cur.score - 1e-9 || rng.f64() < self.p.accept_worse {
            std::mem::swap(cur, &mut nx);
            self.cur_id = id;
            self.k = self.p.kick_min.max(1);
        }
        if let Some(c) = &self.cur {
            if c.score < self.best_since_restart - 1e-9 {
                self.best_since_restart = c.score;
                self.stale = 0;
            } else {
                self.stale += 1;
                if self.stale >= self.p.stagnation {
                    self.cur = None;
                }
            }
        }
        self.next = Some(nx);
        StepInfo { evals }
    }

    fn finish(&mut self, _ctx: &Ctx, _rng: &mut Rng) {}

    fn top(&self) -> &TopK {
        &self.top
    }
}

/// Steepest descent that only takes swaps keeping the layout at least `dmin` slots away
/// (Hamming) from every layout in `ex`. Returns evaluations used. If the start is already too
/// close to one of them, the constraint for that one is dropped.
pub fn constrained_descent(ctx: &Ctx, st: &mut State, ex: &[Vec<u8>], dmin: usize) -> u64 {
    let sc = ctx.scorer;
    let m = &ctx.movable;
    let ham = |b: &[u8], s: &[u8]| b.iter().zip(s).filter(|(x, y)| x != y).count() as i32;
    let mut dist: Vec<i32> = ex.iter().map(|b| ham(b, &st.sym)).collect();
    let active: Vec<bool> = dist.iter().map(|&d| d >= dmin as i32).collect();
    let feasible = |st: &State, dist: &[i32], a: usize, b: usize| {
        let (pa, pb) = (st.pos[a] as usize, st.pos[b] as usize);
        ex.iter().zip(dist).zip(&active).all(|((e, &d), &on)| {
            if !on {
                return true;
            }
            let before = (e[pa] as usize != a) as i32 + (e[pb] as usize != b) as i32;
            let after = (e[pa] as usize != b) as i32 + (e[pb] as usize != a) as i32;
            d - before + after >= dmin as i32
        })
    };
    let mut evals = 0;
    loop {
        let mut best = (0, 0, -1e-9);
        for i in 0..m.len() {
            for j in (i + 1)..m.len() {
                let d = sc.delta(st, m[i], m[j]);
                evals += 1;
                if d < best.2 && feasible(st, &dist, m[i], m[j]) {
                    best = (m[i], m[j], d);
                }
            }
        }
        if best.2 >= -1e-9 {
            return evals;
        }
        let (a, b) = (best.0, best.1);
        let (pa, pb) = (st.pos[a] as usize, st.pos[b] as usize);
        for (e, d) in ex.iter().zip(dist.iter_mut()) {
            *d += (e[pa] as usize != b) as i32 + (e[pb] as usize != a) as i32 - (e[pa] as usize != a) as i32 - (e[pb] as usize != b) as i32;
        }
        sc.apply(st, a, b, best.2);
    }
}
