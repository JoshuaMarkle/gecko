//! Larger neighbourhoods tried when no single swap improves (a swap-local optimum): exchange two
//! same-finger columns, 3-cycles, and the best pair of swaps (depth 2, as in Oxeylyzer). Every
//! move is a short sequence of swaps between movable symbols, so pins are respected and the
//! score stays delta-tracked. No allocation. `context/algorithms/README.md`.

use serde::{Deserialize, Serialize};

use super::Ctx;
use crate::scorer::{Scorer, State};

/// Which extra neighbourhoods every steepest descent uses after swaps run out.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MoveParams {
    /// Exchange two whole columns on the same finger (row by row).
    pub columns: bool,
    /// Rotate three symbols (a→b→c→a), both orientations.
    pub cycle3: bool,
    /// Best pair of swaps (the second evaluated after the first).
    pub depth2: bool,
    /// Depth 2: first swaps tried, the least-worsening ones (0 = all, O(m⁴)). At most
    /// [`MAX_WIDTH`].
    pub depth2_width: usize,
}

pub const MAX_WIDTH: usize = 64;

/// A neighbourhood search: applies the best improving move; returns (evaluations, improved).
pub type MoveFn = fn(&Ctx, &mut State) -> (u64, bool);

impl Default for MoveParams {
    fn default() -> Self {
        MoveParams { columns: false, cycle3: false, depth2: false, depth2_width: 32 }
    }
}

impl MoveParams {
    /// Parse a comma list of `swap` (no extras), `columns`, `cycle3`, `depth2`, `all`.
    pub fn parse(s: &str) -> Option<MoveParams> {
        let mut m = MoveParams::default();
        for t in s.split(',').map(str::trim).filter(|t| !t.is_empty()) {
            match t {
                "swap" => {}
                "columns" => m.columns = true,
                "cycle3" => m.cycle3 = true,
                "depth2" => m.depth2 = true,
                "all" => (m.columns, m.cycle3, m.depth2) = (true, true, true),
                _ => return None,
            }
        }
        Some(m)
    }

    /// Inverse of [`MoveParams::parse`] (`swap` when no extras are on).
    pub fn name(&self) -> String {
        let v: Vec<&str> = [(self.columns, "columns"), (self.cycle3, "cycle3"), (self.depth2, "depth2")]
            .into_iter()
            .filter(|x| x.0)
            .map(|x| x.1)
            .collect();
        if v.is_empty() {
            "swap".into()
        } else {
            v.join(",")
        }
    }

    pub fn any(&self) -> bool {
        self.columns || self.cycle3 || self.depth2
    }
}

/// Pairs of same-size, fully movable columns on the same finger and layer, as slot lists
/// sorted by row (exchanged slot by slot).
pub fn column_pairs(sc: &Scorer, movable: &[usize], start: &State) -> Vec<(Vec<usize>, Vec<usize>)> {
    let mut movable_slot = vec![false; sc.n];
    for &s in movable {
        movable_slot[start.pos[s] as usize] = true;
    }
    let g = &sc.geometry;
    // (layer, col, finger, all movable, slots)
    let mut cols: Vec<(u8, i8, usize, bool, Vec<usize>)> = Vec::new();
    for (slot, k) in g.key.iter().enumerate() {
        let f = sc.finger_of_slot(slot);
        match cols.iter_mut().find(|c| c.0 == k.layer && c.1 == k.col && c.2 == f) {
            Some(c) => {
                c.3 &= movable_slot[slot];
                c.4.push(slot);
            }
            None => cols.push((k.layer, k.col, f, movable_slot[slot], vec![slot])),
        }
    }
    cols.retain(|c| c.3 && c.4.len() > 1);
    for c in &mut cols {
        c.4.sort_by_key(|&s| g.key[s].row);
    }
    let mut out = Vec::new();
    for i in 0..cols.len() {
        for j in (i + 1)..cols.len() {
            let (a, b) = (&cols[i], &cols[j]);
            if a.0 == b.0 && a.2 == b.2 && a.4.len() == b.4.len() {
                out.push((a.4.clone(), b.4.clone()));
            }
        }
    }
    out
}

const EPS: f64 = -1e-9;

/// Score and loads before a trial move, restored exactly after undoing it.
struct Snap {
    score: f64,
    load: [f64; crate::geometry::FINGERS],
}

impl Snap {
    fn of(st: &State) -> Self {
        Snap { score: st.score, load: st.load }
    }
    fn restore(&self, st: &mut State) {
        st.score = self.score;
        st.load = self.load;
    }
}

/// Apply the swap of `a` and `b`; returns its delta.
#[inline]
fn swap(sc: &Scorer, st: &mut State, a: usize, b: usize) -> f64 {
    let d = sc.delta(st, a, b);
    sc.apply(st, a, b, d);
    d
}

/// Exchange columns `a` and `b` (slot lists) in place; returns the delta and evaluations.
fn exchange(sc: &Scorer, st: &mut State, a: &[usize], b: &[usize]) -> (f64, u64) {
    let mut d = 0.0;
    for (&sa, &sb) in a.iter().zip(b) {
        let (x, y) = (st.sym[sa] as usize, st.sym[sb] as usize);
        d += swap(sc, st, x, y);
    }
    (d, a.len() as u64)
}

/// Best improving same-finger column exchange, applied. Returns (evaluations, improved).
pub fn column_move(ctx: &Ctx, st: &mut State) -> (u64, bool) {
    let sc = ctx.scorer;
    let mut evals = 0;
    let mut best = (usize::MAX, EPS);
    for (i, (a, b)) in ctx.col_pairs.iter().enumerate() {
        let snap = Snap::of(st);
        let (d, e) = exchange(sc, st, a, b);
        evals += e;
        // Undo: the exchanged slots are disjoint, so swapping the same slots back restores pos/sym.
        for (&sa, &sb) in a.iter().zip(b) {
            let (x, y) = (st.sym[sa] as usize, st.sym[sb] as usize);
            sc.apply(st, x, y, 0.0);
        }
        snap.restore(st);
        if d < best.1 {
            best = (i, d);
        }
    }
    if best.0 == usize::MAX {
        return (evals, false);
    }
    let (a, b) = &ctx.col_pairs[best.0];
    evals += exchange(sc, st, a, b).1;
    (evals, true)
}

/// Best improving 3-cycle, applied. A 3-cycle of {a, b, c} is swap(a, b) then swap(a, c) or
/// swap(b, c) (the two orientations). Returns (evaluations, improved).
pub fn cycle3_move(ctx: &Ctx, st: &mut State) -> (u64, bool) {
    let sc = ctx.scorer;
    let m = &ctx.movable;
    let mut evals = 0;
    // (a, b, second swap partner of `c`: 0 = a, 1 = b, c, delta)
    let mut best = (0, 0, 0u8, 0, EPS);
    for i in 0..m.len() {
        for j in (i + 1)..m.len() {
            let (a, b) = (m[i], m[j]);
            let snap = Snap::of(st);
            let d1 = swap(sc, st, a, b);
            for &c in &m[(j + 1)..] {
                for (w, x) in [(0u8, a), (1, b)] {
                    let d = d1 + sc.delta(st, x, c);
                    if d < best.4 {
                        best = (a, b, w, c, d);
                    }
                }
            }
            evals += 1 + 2 * (m.len() - j - 1) as u64;
            sc.apply(st, a, b, 0.0);
            snap.restore(st);
        }
    }
    if best.4 >= EPS {
        return (evals, false);
    }
    let (a, b, w, c, _) = best;
    swap(sc, st, a, b);
    swap(sc, st, if w == 0 { a } else { b }, c);
    (evals + 2, true)
}

/// Best improving pair of swaps, applied: for each first swap (the `depth2_width`
/// least-worsening ones, or all), every second swap is evaluated after it. Returns
/// (evaluations, improved).
pub fn depth2_move(ctx: &Ctx, st: &mut State) -> (u64, bool) {
    let sc = ctx.scorer;
    let m = &ctx.movable;
    let width = ctx.moves.depth2_width.min(MAX_WIDTH);
    let mut evals = 0;
    // Candidate first swaps, ascending by delta (fixed buffer, no allocation).
    let mut cand = [(f64::INFINITY, 0usize, 0usize); MAX_WIDTH];
    let mut nc = 0;
    if width > 0 {
        for i in 0..m.len() {
            for j in (i + 1)..m.len() {
                let d = sc.delta(st, m[i], m[j]);
                evals += 1;
                if nc < width || d < cand[nc - 1].0 {
                    let mut k = nc.min(width - 1);
                    while k > 0 && cand[k - 1].0 > d {
                        cand[k] = cand[k - 1];
                        k -= 1;
                    }
                    cand[k] = (d, m[i], m[j]);
                    nc = (nc + 1).min(width);
                }
            }
        }
    }
    let mut best = (0, 0, 0, 0, EPS);
    let mut second = |st: &mut State, a: usize, b: usize, evals: &mut u64| {
        let snap = Snap::of(st);
        let d1 = swap(sc, st, a, b);
        *evals += 1;
        for i in 0..m.len() {
            for j in (i + 1)..m.len() {
                if (m[i], m[j]) == (a, b) {
                    continue;
                }
                let d = d1 + sc.delta(st, m[i], m[j]);
                *evals += 1;
                if d < best.4 {
                    best = (a, b, m[i], m[j], d);
                }
            }
        }
        sc.apply(st, a, b, 0.0);
        snap.restore(st);
    };
    if width > 0 {
        for &(_, a, b) in &cand[..nc] {
            second(st, a, b, &mut evals);
        }
    } else {
        for i in 0..m.len() {
            for j in (i + 1)..m.len() {
                second(st, m[i], m[j], &mut evals);
            }
        }
    }
    if best.4 >= EPS {
        return (evals, false);
    }
    let (a, b, c, d, _) = best;
    swap(sc, st, a, b);
    swap(sc, st, c, d);
    (evals + 2, true)
}
