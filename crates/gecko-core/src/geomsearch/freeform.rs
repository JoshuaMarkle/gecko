//! I1 free-form boards: every key's pose (x, y, z, rx, ry, rz) is a free variable, starting
//! from a parametric board (usually a geometry run's best). On a mirror-symmetric split only
//! the left half's keys are variables and the right half mirrors them. A smoothness penalty
//! holds each key's move close to its neighbours' moves, so the board bends rather than
//! scatters; colliding candidates are pushed apart by [`Geometry::repair`] before scoring.
//! See `context/algorithms/geometry-search.md#free-form-keys-i1`.

use super::Var;
use crate::geometry::{Finger, Geometry, Key};
use crate::{Error, Result};

/// Pose components of a key variable `key.<i>.<c>`.
pub const COMPONENTS: [&str; 6] = ["x", "y", "z", "rx", "ry", "rz"];

/// Default search range around a key's start value: ±0.5 u for positions, ±15° for angles.
fn half_range(c: usize) -> f64 {
    if c < 3 {
        0.5
    } else {
        15.0
    }
}

/// `name` is a key variable (`key.<i>.<c>`).
pub fn is_key_var(name: &str) -> bool {
    name.starts_with("key.")
}

/// A `--free` term this module expands: `keys` (all six components), `keys2d` (x, y, rz) or
/// `keys.<c>`.
pub fn is_keys_term(term: &str) -> bool {
    term == "keys" || term == "keys2d" || term.starts_with("keys.")
}

fn components(term: &str) -> Result<Vec<usize>> {
    match term {
        "keys" => Ok((0..6).collect()),
        "keys2d" => Ok(vec![0, 1, 5]),
        t => {
            let c = t.strip_prefix("keys.").and_then(|c| COMPONENTS.iter().position(|n| *n == c));
            c.map(|c| vec![c]).ok_or_else(|| {
                Error::invalid("free parameter", format!("'{t}': use keys, keys2d or keys.<x|y|z|rx|ry|rz>"))
            })
        }
    }
}

fn get(k: &Key, c: usize) -> f64 {
    if c < 3 {
        k.pos[c] as f64
    } else {
        k.rot[c - 3] as f64
    }
}

fn set(k: &mut Key, c: usize, v: f64) {
    if c < 3 {
        k.pos[c] = v as f32;
    } else {
        k.rot[c - 3] = v as f32;
    }
}

fn mirrored(f: Finger) -> Finger {
    Finger::ALL[9 - f.index()]
}

/// Mirror pairs of a split board: `partner[i]` for every key and the mirror sum `S` (a right
/// key's x is `S − x − width` of its left key), when every key has a partner with the mirrored
/// finger, the same row, size, y, z and mirrored rotation. `None` for other boards.
fn mirror_pairs(g: &Geometry) -> Option<(Vec<Option<usize>>, f64)> {
    let close = |a: f64, b: f64| (a - b).abs() < 1e-3;
    let (left, right): (Vec<usize>, Vec<usize>) =
        (0..g.len()).filter(|&i| g.key[i].phys.is_none()).partition(|&i| g.key[i].hand() == crate::Hand::L);
    if left.is_empty() || left.len() != right.len() {
        return None;
    }
    let matches = |l: usize, r: usize| {
        let (a, b) = (&g.key[l], &g.key[r]);
        b.row == a.row
            && b.finger == mirrored(a.finger)
            && b.size == a.size
            && close(a.pos[1] as f64, b.pos[1] as f64)
            && close(a.pos[2] as f64, b.pos[2] as f64)
            && close(a.rot[0] as f64, b.rot[0] as f64)
            && close(a.rot[1] as f64, -b.rot[1] as f64)
            && close(a.rot[2] as f64, -b.rot[2] as f64)
    };
    let sum = |l: usize, r: usize| (g.key[l].pos[0] + g.key[r].pos[0] + g.key[l].size[0]) as f64;
    // The mirror sum most candidate pairs agree on.
    let sums: Vec<f64> = left.iter().flat_map(|&l| right.iter().filter(move |&&r| matches(l, r)).map(move |&r| sum(l, r))).collect();
    let s = *sums.iter().max_by_key(|&&s| sums.iter().filter(|&&t| close(s, t)).count())?;
    let mut partner = vec![None; g.len()];
    for &l in &left {
        let mut rs = right.iter().copied().filter(|&r| matches(l, r) && close(sum(l, r), s));
        let (Some(r), None) = (rs.next(), rs.next()) else { return None };
        if partner[r].is_some() {
            return None;
        }
        (partner[l], partner[r]) = (Some(r), Some(l));
    }
    Some((partner, s))
}

/// Neighbouring physical keys: same hand, both thumb keys or both finger keys, and (row, col)
/// labels one apart in one direction.
fn neighbours(g: &Geometry) -> Vec<(usize, usize)> {
    let keys: Vec<usize> = (0..g.len()).filter(|&i| g.key[i].phys.is_none()).collect();
    let mut out = Vec::new();
    for (n, &i) in keys.iter().enumerate() {
        for &j in &keys[n + 1..] {
            let (a, b) = (&g.key[i], &g.key[j]);
            let step = (a.row as i32 - b.row as i32).abs() + (a.col as i32 - b.col as i32).abs();
            if a.hand() == b.hand() && a.finger.is_thumb() == b.finger.is_thumb() && step == 1 {
                out.push((i, j));
            }
        }
    }
    out
}

/// Key variables of `board` (generated from the parametric start board) for the `keys` terms
/// of a `--free` list. `=lo:hi` on a term sets the range relative to each key's start value.
pub fn parse(board: &Geometry, terms: &[(&str, Option<(f64, f64)>)]) -> Result<Vec<Var>> {
    let partner = mirror_pairs(board).map(|p| p.0);
    // Variables for the left key of each mirror pair, every key otherwise.
    let keys: Vec<usize> = (0..board.len())
        .filter(|&i| board.key[i].phys.is_none())
        .filter(|&i| partner.as_ref().is_none_or(|_| board.key[i].hand() == crate::Hand::L))
        .collect();
    let mut comps: Vec<(usize, Option<(f64, f64)>)> = Vec::new();
    for &(term, range) in terms {
        for c in components(term)? {
            match comps.iter_mut().find(|e| e.0 == c) {
                Some(e) => e.1 = range.or(e.1),
                None => comps.push((c, range)),
            }
        }
    }
    comps.sort_by_key(|e| e.0);
    let mut vars = Vec::new();
    for &k in &keys {
        for &(c, range) in &comps {
            let start = get(&board.key[k], c);
            let (lo, hi) = range.unwrap_or((-half_range(c), half_range(c)));
            vars.push(Var { name: format!("key.{k}.{}", COMPONENTS[c]), lo: start + lo.min(0.0), hi: start + hi.max(0.0), start });
        }
    }
    Ok(vars)
}

/// The board a set of key-variable values describes.
pub(super) struct KeyModel {
    /// The start board (efforts as the search uses them).
    base: Geometry,
    /// (key, component) of each variable.
    vars: Vec<(usize, usize)>,
    /// Mirror partner of each key and the mirror sum, on a symmetric split.
    partner: Vec<Option<usize>>,
    mirror_sum: f64,
    neighbours: Vec<(usize, usize)>,
    keep_effort: bool,
}

impl KeyModel {
    pub fn new(base: &Geometry, vars: &[Var], keep_effort: bool) -> Result<KeyModel> {
        let bad = |n: &str| Error::invalid("free parameter", format!("'{n}': not a key of the start board"));
        let vars: Vec<(usize, usize)> = vars
            .iter()
            .map(|v| {
                let parts: Vec<&str> = v.name.split('.').collect();
                let [_, k, c] = parts[..] else { return Err(bad(&v.name)) };
                let k = k.parse::<usize>().ok().filter(|&k| k < base.len() && base.key[k].phys.is_none()).ok_or_else(|| bad(&v.name))?;
                let c = COMPONENTS.iter().position(|n| *n == c).ok_or_else(|| bad(&v.name))?;
                Ok((k, c))
            })
            .collect::<Result<_>>()?;
        let (partner, mirror_sum) = mirror_pairs(base).unwrap_or_else(|| (vec![None; base.len()], 0.0));
        if vars.iter().any(|&(k, _)| partner[k].is_some_and(|_| base.key[k].hand() == crate::Hand::R)) {
            return Err(Error::invalid("free parameter", "on a mirrored split only left-hand keys are variables"));
        }
        Ok(KeyModel { base: base.clone(), vars, partner, mirror_sum, neighbours: neighbours(base), keep_effort })
    }

    /// The board at `x`: each variable set on its key, mirrored onto the partner key, efforts
    /// re-derived unless kept.
    pub fn board(&self, x: &[f64]) -> Geometry {
        let mut g = self.base.clone();
        for (&(k, c), &v) in self.vars.iter().zip(x) {
            set(&mut g.key[k], c, v);
            if let Some(p) = self.partner[k] {
                let w = g.key[k].size[0] as f64;
                let m = match c {
                    0 => self.mirror_sum - v - w,
                    4 | 5 => -v,
                    _ => v,
                };
                set(&mut g.key[p], c, m);
            }
        }
        if !self.keep_effort {
            g.key.iter_mut().for_each(|k| k.effort = f32::NAN);
            g.derive_missing(false);
        }
        g
    }

    /// The variable values of board `g` (the inverse of [`KeyModel::board`] for its own keys).
    pub fn values(&self, g: &Geometry) -> Vec<f64> {
        self.vars.iter().map(|&(k, c)| get(&g.key[k], c)).collect()
    }

    /// Smoothness: over neighbouring keys, the squared difference of their moves from the start
    /// board (position in u, rotation in radians, all six components).
    pub fn roughness(&self, g: &Geometry) -> f64 {
        let moved = |i: usize, c: usize| {
            let d = get(&g.key[i], c) - get(&self.base.key[i], c);
            if c < 3 {
                d
            } else {
                d.to_radians()
            }
        };
        self.neighbours.iter().map(|&(i, j)| (0..6).map(|c| (moved(i, c) - moved(j, c)).powi(2)).sum::<f64>()).sum()
    }
}
