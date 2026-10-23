//! Path relinking between two layouts: the swap sequence that turns one into the other,
//! taking at each step the swap that costs least. Shows the barrier between two designs and
//! the layouts halfway. `research/studies/search-space-visualization.md` §3.3 (31).

use serde::Serialize;

use crate::scorer::Scorer;
use crate::{Error, Result};

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct PathStep {
    /// Slots whose symbols were exchanged.
    pub a: usize,
    pub b: usize,
    /// Score after the swap.
    pub score: f64,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Path {
    pub start: f64,
    pub steps: Vec<PathStep>,
    /// Highest score on the path above the worse of its two ends (0 = no barrier).
    pub barrier: f64,
}

/// Path from `from` to `to` (`sym[slot]` over the scorer's symbols). Every step moves one more
/// symbol to the slot it has in `to` (so the path has at most Cayley-distance + cycles steps),
/// choosing the swap with the smallest score change.
pub fn relink(sc: &Scorer, from: &[u8], to: &[u8]) -> Result<Path> {
    let n = sc.n;
    let mut seen = vec![false; n];
    let valid = |s: &[u8]| s.len() == n && s.iter().all(|&x| (x as usize) < n);
    if !valid(from) || !valid(to) || !to.iter().all(|&x| !std::mem::replace(&mut seen[x as usize], true)) {
        return Err(Error::invalid("path", "both layouts must hold the same symbols"));
    }
    let mut st = sc.state_from_sym(from.to_vec());
    let start = st.score;
    let mut steps = Vec::new();
    loop {
        let mut best: Option<(usize, usize, f64)> = None;
        for p in 0..n {
            if st.sym[p] == to[p] {
                continue;
            }
            let (x, y) = (st.sym[p] as usize, to[p] as usize);
            let d = sc.delta(&st, x, y);
            if best.is_none_or(|b| d < b.2) {
                best = Some((x, y, d));
            }
        }
        let Some((x, y, d)) = best else { break };
        let (a, b) = (st.pos[x] as usize, st.pos[y] as usize);
        sc.apply(&mut st, x, y, d);
        steps.push(PathStep { a: a.min(b), b: a.max(b), score: st.score });
    }
    // The last score from scratch, so accumulated deltas never show as a fake gap.
    if let Some(last) = steps.last_mut() {
        last.score = sc.full(&st);
    }
    let end = steps.last().map_or(start, |s| s.score);
    let top = steps.iter().map(|s| s.score).fold(start, f64::max);
    Ok(Path { start, steps, barrier: top - start.max(end) })
}

/// `chars` (slot-ordered, as the scorer's start layout lists them) as symbol ids.
pub fn target_sym(sc: &Scorer, chars: &str) -> Result<Vec<u8>> {
    let sym: Option<Vec<u8>> = chars.chars().map(|c| sc.chars.iter().position(|&x| x == c).map(|i| i as u8)).collect();
    sym.filter(|s| s.len() == sc.n).ok_or_else(|| Error::invalid("path", "the target layout has other symbols than the start"))
}
