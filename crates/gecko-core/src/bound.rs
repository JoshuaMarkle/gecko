//! Gilmore–Lawler lower bound on the objective, and the optimality gap of a layout.
//! Method and caveats: `context/algorithms/bounds.md`.
//!
//! The objective is a quadratic assignment `Σ_x M[x, p(x)] + Σ_{x≠y} Q(x, y, p(x), p(y))`
//! (trigrams relaxed to pairs, load terms ≥ 0 dropped; `Scorer::qap_terms`). For symbol x on
//! slot i, `l[x, i] = M'[x, i] + min_assignment Σ_y ½(Q(x, y, i, j) + Q(y, x, j, i))` over the
//! other movable symbols and free slots; the bound is the minimum assignment over `l`. Pinned
//! symbols are constants: their terms with movable symbols are folded into `M'`.

use serde::{Deserialize, Serialize};

use crate::scorer::QapTerms;
use crate::{Scorer, State};

/// Above this many movable symbols the bound is skipped: the dual ascent stores m⁴ pair costs
/// (42 MB at 48) and each iteration solves m² assignments of size m − 1.
pub const MAX_SYMBOLS: usize = 48;

/// Dual-ascent iterations after the plain Gilmore–Lawler bound (iteration 0).
/// On ansi-30 / default the bound rises 4.49 → 5.70 (10) → 5.77 (20) → 5.81 (50, converged).
pub const ASCENT_ITERS: usize = 20;

/// A lower bound on (part of) the objective and how far a layout's value is above it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Gap {
    /// No layout can score below this.
    pub lower: f64,
    /// The plain Gilmore–Lawler bound (before dual ascent).
    pub gilmore_lawler: f64,
    /// The layout's value of the same part.
    pub value: f64,
    /// `(value − lower) / |value|` in %: the layout is within this of optimal.
    pub gap_pct: f64,
}

impl Gap {
    fn new(b: Bounds, value: f64) -> Self {
        let gap_pct = if value.abs() > 1e-12 { 100.0 * (value - b.lower) / value.abs() } else { 0.0 };
        Gap { lower: b.lower, gilmore_lawler: b.gl, value, gap_pct }
    }
}

/// Bounds reported with a run's best layout (`best.json` → `bound`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BoundReport {
    pub method: String,
    /// Movable symbols in the assignment.
    pub symbols: usize,
    /// Dual-ascent iterations.
    pub iters: usize,
    /// The whole objective: trigrams relaxed to pairs, load terms bounded by 0.
    pub total: Gap,
    /// Only the mono + bigram/skipgram part (trigram and load terms excluded): the layout's
    /// value of that part vs. the best any layout could reach on it.
    pub pair: Gap,
    pub elapsed_ms: u64,
}

/// Lower bounds for `st` with `movable` symbols free (the rest stay on their slots in `st`).
/// `None` with too many symbols or a negatively weighted load term.
pub fn report(scorer: &Scorer, movable: &[usize], st: &State, iters: usize) -> Option<BoundReport> {
    if movable.len() > MAX_SYMBOLS {
        return None;
    }
    let t0 = std::time::Instant::now();
    let total = lower_bound(&scorer.qap_terms(true)?, movable, &st.pos, iters);
    let pair = lower_bound(&scorer.qap_terms(false)?, movable, &st.pos, iters);
    let (pair_value, _, _) = scorer.parts(st);
    Some(BoundReport {
        method: "gilmore-lawler+dual-ascent".into(),
        symbols: movable.len(),
        iters,
        total: Gap::new(total, st.score),
        pair: Gap::new(pair, pair_value),
        elapsed_ms: t0.elapsed().as_millis() as u64,
    })
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Bounds {
    /// Plain Gilmore–Lawler.
    pub gl: f64,
    /// After the dual ascent (≥ `gl`).
    pub lower: f64,
}

/// Gilmore–Lawler bound of `t` over permutations of `movable` among their slots in `pos`,
/// then `iters` rounds of dual ascent (Hahn–Grant style): every round re-expresses the
/// objective exactly as `K + Σ lin[x, p(x)] + Σ R[x, p(x), y, p(y)]` with `lin, R ≥ 0`, so `K`
/// is a lower bound; it then spreads `lin` back into `R` and re-solves, which raises `K`.
pub(crate) fn lower_bound(t: &QapTerms, movable: &[usize], pos: &[u8], iters: usize) -> Bounds {
    let a = ascent(t, movable, pos, iters);
    Bounds { gl: a.gl, lower: a.lower }
}

/// The objective over permutations of `m` movable symbols among their slots, rewritten
/// exactly as `k + Σ_a lin[a, s_a] + Σ_{a≠b} r[a, s_a, b, s_b]` (`r` indexed
/// `((a·m + s)·m + b)·m + u`, both orders counted) with `lin, r ≥ 0` up to rounding, so any
/// layout's value is `k` plus non-negative terms. `crate::certify` branches on this form.
pub(crate) struct Ascent {
    pub gl: f64,
    pub lower: f64,
    pub k: f64,
    pub lin: Vec<f64>,
    pub r: Vec<f64>,
}

/// Gilmore–Lawler plus dual ascent; see [`lower_bound`]. Also returns the final exact rewrite.
pub(crate) fn ascent(t: &QapTerms, movable: &[usize], pos: &[u8], iters: usize) -> Ascent {
    let n = t.n;
    let q =
        |x: usize, y: usize, i: usize, j: usize| -> f64 { t.chans.iter().map(|(f, c)| f[x * n + y] as f64 * c[i * n + j] as f64).sum() };
    let mut is_free = vec![true; n];
    for &x in movable {
        is_free[x] = false;
    }
    let pinned: Vec<usize> = (0..n).filter(|&x| is_free[x]).collect();
    let slots: Vec<usize> = movable.iter().map(|&x| pos[x] as usize).collect();
    // Constant: pinned symbols among themselves.
    let mut k = 0.0;
    for &p in &pinned {
        k += t.mono[p * n + pos[p] as usize];
        for &r in &pinned {
            if r != p {
                k += q(p, r, pos[p] as usize, pos[r] as usize);
            }
        }
    }
    let m = movable.len();
    // Linear terms: mono plus every term with a pinned symbol.
    let mut lin = vec![0.0f64; m * m];
    for (a, &x) in movable.iter().enumerate() {
        for (s, &i) in slots.iter().enumerate() {
            let mut l = t.mono[x * n + i];
            for &p in &pinned {
                let pp = pos[p] as usize;
                l += q(x, p, i, pp) + q(p, x, pp, i);
            }
            lin[a * m + s] = l;
        }
    }
    let mut lap = Lap::new(m.max(1));
    if m <= 1 {
        let v = k + lin.first().copied().unwrap_or(0.0);
        return Ascent { gl: v, lower: v, k, lin, r: vec![0.0; m] };
    }
    // Pair costs R[(a, s), (b, u)] = ½(Q(x, y, i, j) + Q(y, x, j, i)) for a ≠ b, s ≠ u.
    let idx = |a: usize, s: usize, b: usize, u: usize| ((a * m + s) * m + b) * m + u;
    let mut r = vec![0.0f64; m * m * m * m];
    for (a, &x) in movable.iter().enumerate() {
        for (b, &y) in movable.iter().enumerate() {
            if a == b {
                continue;
            }
            for (s, &i) in slots.iter().enumerate() {
                for (u, &j) in slots.iter().enumerate() {
                    if s != u {
                        r[idx(a, s, b, u)] = 0.5 * (q(x, y, i, j) + q(y, x, j, i));
                    }
                }
            }
        }
    }
    let k1 = m - 1;
    let mut inner = vec![0.0f64; k1 * k1];
    let (mut gl, mut best) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for it in 0..=iters {
        // Each (a, s) row: its best completion becomes linear, R keeps the reduced costs.
        for a in 0..m {
            for s in 0..m {
                let mut c = 0;
                for b in (0..m).filter(|&b| b != a) {
                    for u in (0..m).filter(|&u| u != s) {
                        inner[c] = r[idx(a, s, b, u)];
                        c += 1;
                    }
                }
                lin[a * m + s] += lap.solve(k1, &inner);
                for (bi, b) in (0..m).filter(|&b| b != a).enumerate() {
                    for (ui, u) in (0..m).filter(|&u| u != s).enumerate() {
                        r[idx(a, s, b, u)] = inner[bi * k1 + ui] - lap.u[bi + 1] - lap.v[ui + 1];
                    }
                }
            }
        }
        k += lap.solve(m, &lin);
        for a in 0..m {
            for s in 0..m {
                lin[a * m + s] -= lap.u[a + 1] + lap.v[s + 1];
            }
        }
        if it == 0 {
            gl = k;
        }
        let gain = k - best;
        best = best.max(k);
        if it == iters || gain <= 1e-9 * k.abs().max(1.0) {
            break;
        }
        // Make R symmetric ((a, s, b, u) and (b, u, a, s) are the same event), then spread
        // lin[a, s] evenly over its m − 1 pair terms (any layout uses exactly m − 1 of them).
        for a in 0..m {
            for b in a + 1..m {
                for s in 0..m {
                    for u in (0..m).filter(|&u| u != s) {
                        let (p, o) = (idx(a, s, b, u), idx(b, u, a, s));
                        let v = 0.5 * (r[p] + r[o]);
                        r[p] = v;
                        r[o] = v;
                    }
                }
            }
        }
        for a in 0..m {
            for s in 0..m {
                let add = lin[a * m + s] / k1 as f64;
                lin[a * m + s] = 0.0;
                for b in (0..m).filter(|&b| b != a) {
                    for u in (0..m).filter(|&u| u != s) {
                        r[idx(a, s, b, u)] += add;
                    }
                }
            }
        }
    }
    Ascent { gl, lower: best, k, lin, r }
}

/// Hungarian algorithm (shortest augmenting paths with potentials), O(n³). Buffers are
/// reused across calls up to the size given to `new`.
pub(crate) struct Lap {
    u: Vec<f64>,
    v: Vec<f64>,
    p: Vec<usize>,
    way: Vec<usize>,
    minv: Vec<f64>,
    used: Vec<bool>,
}

impl Lap {
    pub(crate) fn new(max: usize) -> Self {
        Lap {
            u: vec![0.0; max + 1],
            v: vec![0.0; max + 1],
            p: vec![0; max + 1],
            way: vec![0; max + 1],
            minv: vec![0.0; max + 1],
            used: vec![false; max + 1],
        }
    }

    /// Minimum-cost perfect assignment of the `k × k` row-major matrix `a`.
    pub(crate) fn solve(&mut self, k: usize, a: &[f64]) -> f64 {
        let (u, v, p, way, minv, used) = (&mut self.u, &mut self.v, &mut self.p, &mut self.way, &mut self.minv, &mut self.used);
        u[..=k].fill(0.0);
        v[..=k].fill(0.0);
        p[..=k].fill(0);
        way[..=k].fill(0);
        for i in 1..=k {
            p[0] = i;
            let mut j0 = 0;
            minv[..=k].fill(f64::INFINITY);
            used[..=k].fill(false);
            loop {
                used[j0] = true;
                let i0 = p[j0];
                let mut delta = f64::INFINITY;
                let mut j1 = 0;
                for j in 1..=k {
                    if !used[j] {
                        let cur = a[(i0 - 1) * k + j - 1] - u[i0] - v[j];
                        if cur < minv[j] {
                            minv[j] = cur;
                            way[j] = j0;
                        }
                        if minv[j] < delta {
                            delta = minv[j];
                            j1 = j;
                        }
                    }
                }
                for j in 0..=k {
                    if used[j] {
                        u[p[j]] += delta;
                        v[j] -= delta;
                    } else {
                        minv[j] -= delta;
                    }
                }
                j0 = j1;
                if p[j0] == 0 {
                    break;
                }
            }
            loop {
                let j1 = way[j0];
                p[j0] = p[j1];
                j0 = j1;
                if j0 == 0 {
                    break;
                }
            }
        }
        (1..=k).map(|j| a[(p[j] - 1) * k + j - 1]).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::Lap;

    #[test]
    fn hungarian_matches_brute_force() {
        let mut rng = crate::rng::Rng::new(5);
        let mut lap = Lap::new(6);
        for k in 1..=6usize {
            let a: Vec<f64> = (0..k * k).map(|_| rng.below(1000) as f64 - 300.0).collect();
            let mut perm: Vec<usize> = (0..k).collect();
            let mut best = f64::INFINITY;
            heap(&mut perm, k, &mut |p| best = best.min((0..k).map(|i| a[i * k + p[i]]).sum()));
            assert!((lap.solve(k, &a) - best).abs() < 1e-9, "k = {k}");
        }
    }

    fn heap(v: &mut Vec<usize>, k: usize, f: &mut impl FnMut(&[usize])) {
        if k <= 1 {
            f(v);
            return;
        }
        for i in 0..k {
            heap(v, k - 1, f);
            let j = if k.is_multiple_of(2) { i } else { 0 };
            v.swap(j, k - 1);
        }
    }
}
