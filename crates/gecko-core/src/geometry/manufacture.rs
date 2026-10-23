//! G07 manufacturability: can this board be built? Limits on switch spacing, key angle, board
//! height, plate (print bed) size and column counts, from a profile's `[build]` section. Each
//! violation has an excess in u-equivalents (angles: 45° = 1 u; counts: 1 key = 1 u), so the
//! geometry search adds their total to its collision depth: the same penalty and hard fail.
//! STL generation (#75) reuses the same limits; `case` also builds the case and counts keycaps
//! that hit it. See `context/geometry.md`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::geometry::case::{CaseParams, Switch};
use crate::geometry::flat2d::V3;
use crate::geometry::pose::{add, apply, rotation, CAP_HEIGHT};
use crate::geometry::{Hand, Key};
use crate::{Error, Geometry, Result};

/// Degrees of angle excess that count as 1 u of excess.
pub const DEG_PER_U: f64 = 45.0;

/// Excess per key whose cap hits the case or whose switch cutout is blocked, u. The case check
/// has no depth, so each key counts a quarter key: a few hits already cost like a collision.
pub const CASE_EXCESS: f64 = 0.25;

/// Build limits (key units, degrees). A limit of 0 is off.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BuildLimits {
    /// Least distance between two switch centres (keycap bottom centres), u. 0.82 u ≈ 15.6 mm:
    /// a 14 mm MX cutout plus 1.6 mm of plate between holes.
    pub min_pitch: f64,
    /// Largest angle between a keycap's normal and vertical, degrees. Steeper keys are hard to
    /// press, wire and print without supports.
    pub max_key_angle: f64,
    /// Largest height of the board, u: highest keycap top above the lowest keycap bottom.
    pub max_height: f64,
    /// Largest plate footprint per piece, `[w, d]` u, in any orientation (a print bed);
    /// [12, 12] ≈ 229 mm. A split board's halves are separate pieces.
    pub max_plate: [f64; 2],
    /// Halves closer than this (nearest switch centres, u) are one piece. 1.25 u: unibody
    /// boards have neighbouring keys 1 u apart across the hands; bundled splits ≥ 1.5 u.
    pub piece_gap: f64,
    /// Most keys in one column (same hand and `col`).
    pub max_column_keys: usize,
    /// Most columns per hand (distinct `col` of finger keys). Off by default: board files label
    /// columns loosely (ergodox's right hand has 13 labels); parametric boards label them exactly.
    pub max_columns: usize,
    /// Build the default case for this switch (`mx` or `choc`) and count keys whose cap hits it
    /// within its travel (`cap_hit`) or whose cutout it blocks (`blocked`). Off by default: it
    /// costs a case build (a few ms) per check.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub case: Option<Switch>,
}

impl Default for BuildLimits {
    fn default() -> Self {
        BuildLimits {
            min_pitch: 0.82,
            max_key_angle: 60.0,
            max_height: 4.0,
            max_plate: [12.0, 12.0],
            piece_gap: 1.25,
            max_column_keys: 6,
            max_columns: 0,
            case: None,
        }
    }
}

impl BuildLimits {
    pub fn validate(&self) -> Result<()> {
        let v = [self.min_pitch, self.max_key_angle, self.max_height, self.max_plate[0], self.max_plate[1], self.piece_gap];
        if !v.iter().all(|x| x.is_finite() && *x >= 0.0) || self.max_key_angle > 180.0 {
            return Err(Error::invalid("profile", "build: limits must be finite and ≥ 0, max_key_angle ≤ 180"));
        }
        Ok(())
    }
}

/// One broken limit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Violation {
    /// `pitch`, `key_angle`, `height`, `plate`, `column_keys`, `columns`, `cap_hit`, `blocked`, `case`.
    pub kind: String,
    /// Keys involved (physical key indices); for `plate` and `columns`, every key of the piece
    /// or hand.
    pub keys: Vec<usize>,
    pub value: f64,
    pub limit: f64,
    /// How far past the limit, u-equivalents (≥ 0).
    pub excess: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BuildReport {
    pub violations: Vec<Violation>,
    /// Sum of the violations' excess.
    pub excess: f64,
}

impl BuildReport {
    pub fn feasible(&self) -> bool {
        self.violations.is_empty()
    }
}

/// Centre of the keycap's bottom face, about where the switch meets the plate (board space).
pub(crate) fn switch_centre(k: &Key) -> V3 {
    let h = CAP_HEIGHT / 2.0;
    let mid = [k.pos[0] as f64 + k.size[0] as f64 / 2.0, k.pos[1] as f64 + k.size[1] as f64 / 2.0, k.pos[2] as f64 + h];
    add(mid, apply(&rotation(k.rot), [0.0, 0.0, -h]))
}

/// The 4 bottom and 4 top corners of a keycap (board space).
fn corners(k: &Key) -> [V3; 8] {
    let h = CAP_HEIGHT / 2.0;
    let m = rotation(k.rot);
    let mid = [k.pos[0] as f64 + k.size[0] as f64 / 2.0, k.pos[1] as f64 + k.size[1] as f64 / 2.0, k.pos[2] as f64 + h];
    let (w, d) = (k.size[0] as f64 / 2.0, k.size[1] as f64 / 2.0);
    let mut out = [[0.0; 3]; 8];
    for (i, o) in out.iter_mut().enumerate() {
        let sx = if i & 1 == 0 { -w } else { w };
        let sy = if i & 2 == 0 { -d } else { d };
        let sz = if i & 4 == 0 { -h } else { h };
        *o = add(mid, apply(&m, [sx, sy, sz]));
    }
    out
}

fn dist(a: V3, b: V3) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// Smallest excess of footprint points `pts` over a `lim` rectangle, turning the piece in 5°
/// steps (a quarter turn covers every orientation, with both rectangle orientations); also the
/// `[w, d]` of the axis-aligned footprint.
fn plate_excess(pts: &[[f64; 2]], lim: [f64; 2]) -> (f64, [f64; 2]) {
    let size = |deg: f64| {
        let (s, c) = deg.to_radians().sin_cos();
        let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
        for p in pts {
            let q = [c * p[0] - s * p[1], s * p[0] + c * p[1]];
            for a in 0..2 {
                lo[a] = lo[a].min(q[a]);
                hi[a] = hi[a].max(q[a]);
            }
        }
        [hi[0] - lo[0], hi[1] - lo[1]]
    };
    let over = |s: [f64; 2], l: [f64; 2]| (s[0] - l[0]).max(0.0) + (s[1] - l[1]).max(0.0);
    let best = (0..18).map(|i| size(i as f64 * 5.0)).map(|s| over(s, lim).min(over(s, [lim[1], lim[0]]))).fold(f64::INFINITY, f64::min);
    (best, size(0.0))
}

impl Geometry {
    /// G07: every build limit this board breaks (physical keys only; layer copies are skipped).
    pub fn manufacture(&self, lim: &BuildLimits) -> BuildReport {
        let keys: Vec<usize> = (0..self.len()).filter(|&i| self.key[i].phys.is_none()).collect();
        let sw: Vec<V3> = keys.iter().map(|&i| switch_centre(&self.key[i])).collect();
        let mut out = Vec::new();
        let mut push = |kind: &str, keys: Vec<usize>, value: f64, limit: f64, excess: f64| {
            out.push(Violation { kind: kind.into(), keys, value, limit, excess });
        };

        if lim.min_pitch > 0.0 {
            for a in 0..keys.len() {
                for b in a + 1..keys.len() {
                    let d = dist(sw[a], sw[b]);
                    if d < lim.min_pitch - 1e-6 {
                        push("pitch", vec![keys[a], keys[b]], d, lim.min_pitch, lim.min_pitch - d);
                    }
                }
            }
        }

        if lim.max_key_angle > 0.0 {
            for &i in &keys {
                let n = apply(&rotation(self.key[i].rot), [0.0, 0.0, 1.0]);
                let deg = n[2].clamp(-1.0, 1.0).acos().to_degrees();
                if deg > lim.max_key_angle + 1e-6 {
                    push("key_angle", vec![i], deg, lim.max_key_angle, (deg - lim.max_key_angle) / DEG_PER_U);
                }
            }
        }

        let all: Vec<[V3; 8]> = keys.iter().map(|&i| corners(&self.key[i])).collect();
        if lim.max_height > 0.0 && !keys.is_empty() {
            let zs = all.iter().flatten().map(|p| p[2]);
            let (lo, hi) = zs.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), z| (lo.min(z), hi.max(z)));
            let h = hi - lo;
            if h > lim.max_height + 1e-6 {
                push("height", keys.clone(), h, lim.max_height, h - lim.max_height);
            }
        }

        if lim.max_plate[0] > 0.0 && lim.max_plate[1] > 0.0 {
            for piece in self.pieces(&keys, &sw, lim.piece_gap) {
                let pts: Vec<[f64; 2]> = piece.iter().flat_map(|&p| all[p].iter().map(|c| [c[0], c[1]])).collect();
                let (excess, size) = plate_excess(&pts, lim.max_plate);
                if excess > 1e-6 {
                    let area = size[0] * size[1];
                    push("plate", piece.iter().map(|&p| keys[p]).collect(), area, lim.max_plate[0] * lim.max_plate[1], excess);
                }
            }
        }

        let mut cols: BTreeMap<(u8, i8), Vec<usize>> = BTreeMap::new();
        for &i in &keys {
            let k = &self.key[i];
            if !k.finger.is_thumb() {
                cols.entry((k.hand() as u8, k.col)).or_default().push(i);
            }
        }
        if lim.max_column_keys > 0 {
            for ks in cols.values().filter(|ks| ks.len() > lim.max_column_keys) {
                let (n, l) = (ks.len() as f64, lim.max_column_keys as f64);
                push("column_keys", ks.clone(), n, l, n - l);
            }
        }
        if lim.max_columns > 0 {
            for hand in [Hand::L, Hand::R] {
                let mine: Vec<&(u8, i8)> = cols.keys().filter(|(h, _)| *h == hand as u8).collect();
                if mine.len() > lim.max_columns {
                    let ks = mine.iter().flat_map(|c| cols[c].iter().copied()).collect();
                    let (n, l) = (mine.len() as f64, lim.max_columns as f64);
                    push("columns", ks, n, l, n - l);
                }
            }
        }

        if let Some(switch) = lim.case {
            match self.case(&CaseParams::for_switch(switch), lim.piece_gap) {
                Ok(c) => {
                    for &k in &c.report.cap_hits {
                        push("cap_hit", vec![k], 1.0, 0.0, CASE_EXCESS);
                    }
                    for &k in &c.report.blocked {
                        push("blocked", vec![k], 1.0, 0.0, CASE_EXCESS);
                    }
                }
                Err(_) => push("case", keys.clone(), 1.0, 0.0, 1.0),
            }
        }

        let excess = out.iter().map(|v| v.excess).sum();
        BuildReport { violations: out, excess }
    }

    /// Plate pieces as indices into `keys`: one piece, or the two hands when every left switch
    /// is at least `gap` from every right one (a split board).
    pub(crate) fn pieces(&self, keys: &[usize], sw: &[V3], gap: f64) -> Vec<Vec<usize>> {
        let (l, r): (Vec<usize>, Vec<usize>) = (0..keys.len()).partition(|&p| self.key[keys[p]].hand() == Hand::L);
        let near = l.iter().any(|&a| r.iter().any(|&b| dist(sw[a], sw[b]) < gap));
        if l.is_empty() || r.is_empty() || near {
            vec![(0..keys.len()).collect()]
        } else {
            vec![l, r]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Finger;

    fn key(x: f32, y: f32, finger: Finger, col: i8) -> Key {
        Key {
            pos: [x, y, 0.0],
            rot: [0.0; 3],
            size: [1.0, 1.0],
            finger,
            row: y as i8,
            col,
            home: false,
            stretch: false,
            hrm: None,
            effort: 1.0,
            fill: None,
            layer: 0,
            phys: None,
            alt: Vec::new(),
        }
    }

    fn board(keys: Vec<Key>) -> Geometry {
        Geometry {
            name: "t".into(),
            description: String::new(),
            key: keys,
            comfort: BTreeMap::new(),
            comfort_tables: BTreeMap::new(),
            layer_keys: vec![],
            layer_modes: vec![],
        }
    }

    fn kinds(r: &BuildReport) -> Vec<&str> {
        r.violations.iter().map(|v| v.kind.as_str()).collect()
    }

    #[test]
    fn grid_is_feasible() {
        let keys = (0..3).flat_map(|y| (0..5).map(move |x| key(x as f32, y as f32, Finger::LI, x))).collect();
        let r = board(keys).manufacture(&BuildLimits::default());
        assert!(r.feasible(), "{r:?}");
        assert_eq!(r.excess, 0.0);
    }

    #[test]
    fn close_switches_break_pitch() {
        let r = board(vec![key(0.0, 0.0, Finger::LI, 0), key(0.7, 0.0, Finger::LI, 1)]).manufacture(&BuildLimits::default());
        assert_eq!(kinds(&r), ["pitch"]);
        assert!((r.excess - 0.12).abs() < 1e-5, "{}", r.excess);
        assert_eq!(r.violations[0].keys, [0, 1]);
    }

    #[test]
    fn steep_key_breaks_angle_and_scales_by_45() {
        let mut k = key(0.0, 0.0, Finger::LI, 0);
        k.rot = [75.0, 0.0, 0.0];
        let r = board(vec![k]).manufacture(&BuildLimits::default());
        assert_eq!(kinds(&r), ["key_angle"]);
        assert!((r.violations[0].value - 75.0).abs() < 1e-4);
        assert!((r.excess - 15.0 / DEG_PER_U).abs() < 1e-5);
    }

    #[test]
    fn tall_board_breaks_height() {
        let mut k = key(3.0, 0.0, Finger::LI, 1);
        k.pos[2] = 5.0;
        let r = board(vec![key(0.0, 0.0, Finger::LI, 0), k]).manufacture(&BuildLimits::default());
        assert_eq!(kinds(&r), ["height"]);
        assert!((r.violations[0].value - (5.0 + CAP_HEIGHT)).abs() < 1e-5);
    }

    #[test]
    fn plate_turns_to_fit_and_splits_pieces() {
        // A 14 × 1 row does not fit 12 × 12 axis-aligned, but fits diagonally (≈ 16.97 u).
        let row: Vec<Key> = (0..14).map(|x| key(x as f32, 0.0, Finger::LI, x)).collect();
        let lim = BuildLimits::default();
        assert!(board(row.clone()).manufacture(&lim).feasible());
        let small = BuildLimits { max_plate: [10.0, 10.0], ..lim.clone() };
        assert_eq!(kinds(&board(row).manufacture(&small)), ["plate"]);
        // Two 8-wide halves 6 u apart are two pieces that each fit; joined they would not.
        let half = |x0: f32, f: Finger| (0..8).map(move |x| key(x0 + x as f32, 0.0, f, x));
        let split: Vec<Key> = half(0.0, Finger::LI).chain(half(14.0, Finger::RI)).collect();
        let tight = BuildLimits { max_plate: [9.0, 2.0], ..lim };
        assert!(board(split.clone()).manufacture(&tight).feasible());
        let joined = BuildLimits { piece_gap: 10.0, ..tight };
        assert_eq!(kinds(&board(split).manufacture(&joined)), ["plate"]);
    }

    #[test]
    fn column_counts() {
        let tall: Vec<Key> = (0..7).map(|y| key(0.0, y as f32, Finger::LI, 0)).collect();
        let r = board(tall).manufacture(&BuildLimits::default());
        assert_eq!(kinds(&r), ["column_keys"]);
        assert_eq!(r.excess, 1.0);
        let wide: Vec<Key> = (0..10).map(|x| key(x as f32, 0.0, Finger::RI, x)).collect();
        let eight = BuildLimits { max_columns: 8, ..Default::default() };
        let r = board(wide).manufacture(&eight);
        assert_eq!(kinds(&r), ["columns"]);
        assert_eq!(r.excess, 2.0);
        // Thumbs are not columns.
        let thumbs: Vec<Key> = (0..10).map(|x| key(x as f32, 0.0, Finger::RT, x)).collect();
        assert!(board(thumbs).manufacture(&eight).feasible());
    }

    #[test]
    fn zero_limits_are_off() {
        let off = BuildLimits {
            min_pitch: 0.0,
            max_key_angle: 0.0,
            max_height: 0.0,
            max_plate: [0.0, 0.0],
            piece_gap: 0.0,
            max_column_keys: 0,
            max_columns: 0,
            case: None,
        };
        let mut k = key(0.1, 0.0, Finger::LI, 0);
        k.rot = [89.0, 0.0, 0.0];
        k.pos[2] = 50.0;
        assert!(board(vec![key(0.0, 0.0, Finger::LI, 0), k]).manufacture(&off).feasible());
    }
}
