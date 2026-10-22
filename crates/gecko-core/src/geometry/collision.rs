//! G04 keycap collision in 2D (`Geometry::collisions`): each keycap is an oriented box
//! (its footprint grown by a margin), pairs come from a uniform grid broad phase, and the
//! separating axis theorem (`flat2d::mtv`) gives the overlap depth. Flat boards only.
//!
//! `Geometry::collisions_3d` is the approximate 3D test the geometry search uses on curved and
//! tented boards until H8 (#66): per key a tapered keycap and a switch body below it, both
//! convex frusta, tested pairwise with the full separating axis theorem. See
//! `context/geometry.md`.

use serde::Serialize;

use crate::geometry::flat2d::{cross, dot, mtv, sub, Rect, V3};
use crate::geometry::pose::{self, CAP_HEIGHT};
use crate::Geometry;

/// Keycap top inset per side, u: an SA/DSA-like cap narrows from its footprint (18 mm) to a
/// ≈ 12.5 mm top; 0.15 u keeps that taper on GECKO's 19.05 mm footprint.
pub const CAP_TAPER: f64 = 0.15;
/// MX switch body below the keycap: half width (15 mm housing) and depth from the cap's bottom
/// (≈ 3 mm to the plate plus the 5.5 mm bottom housing).
pub const SWITCH_HALF: f64 = 0.39;
pub const SWITCH_DEPTH: f64 = 0.45;

/// Two physical keys whose keycaps overlap.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Collision {
    pub a: usize,
    pub b: usize,
    /// Penetration depth in key units: how far one must move to clear the other (with margin).
    pub depth: f32,
}

impl Geometry {
    /// Overlapping keycap pairs (`a < b`, physical keys only), each footprint grown by `margin`
    /// key units on every side, so `margin` m keeps keys 2m apart. Overlaps of at most 0.001 u
    /// count as touching. Empty on non-flat boards (3D collision is H8).
    pub fn collisions(&self, margin: f32) -> Vec<Collision> {
        if !self.is_flat() {
            return Vec::new();
        }
        let m = margin as f64;
        let keys: Vec<usize> = (0..self.len()).filter(|&i| self.key[i].phys.is_none()).collect();
        let rects: Vec<Rect> = keys
            .iter()
            .map(|&i| {
                let k = &self.key[i];
                let h = [k.size[0] as f64 / 2.0, k.size[1] as f64 / 2.0];
                Rect { c: [k.pos[0] as f64 + h[0], k.pos[1] as f64 + h[1]], h: [h[0] + m, h[1] + m], rot: k.rot[2] as f64 }
            })
            .collect();
        // Broad phase: cells as wide as the largest box diagonal, so overlapping boxes have
        // centres in the same or neighbouring cells.
        let cell = rects.iter().map(|r| 2.0 * r.h[0].hypot(r.h[1])).fold(1e-6, f64::max);
        let at = |r: &Rect| ((r.c[0] / cell).floor() as i64, (r.c[1] / cell).floor() as i64);
        let mut grid: Vec<((i64, i64), usize)> = rects.iter().enumerate().map(|(i, r)| (at(r), i)).collect();
        grid.sort_unstable();
        let mut out = Vec::new();
        for (i, r) in rects.iter().enumerate() {
            let (cx, cy) = at(r);
            for nx in cx - 1..=cx + 1 {
                for ny in cy - 1..=cy + 1 {
                    let start = grid.partition_point(|e| e.0 < (nx, ny));
                    for &(_, j) in grid[start..].iter().take_while(|e| e.0 == (nx, ny)) {
                        if j <= i {
                            continue;
                        }
                        if let Some(t) = mtv(r, &rects[j]) {
                            out.push(Collision { a: keys[i], b: keys[j], depth: t[0].hypot(t[1]) as f32 });
                        }
                    }
                }
            }
        }
        out.sort_by_key(|c| (c.a, c.b));
        out
    }
}

/// A frustum in a key's frame: rectangle `lo` (half sizes) at local z `z[0]`, rectangle `hi`
/// at `z[1]`, both centred on the local z axis.
struct Body {
    v: [V3; 8],
}

impl Body {
    fn new(c: V3, m: &pose::M3, lo: [f64; 2], hi: [f64; 2], z: [f64; 2]) -> Body {
        let corner = |h: [f64; 2], z: f64, i: usize| {
            let (sx, sy) = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)][i];
            pose::add(c, pose::apply(m, [sx * h[0], sy * h[1], z]))
        };
        Body { v: std::array::from_fn(|i| if i < 4 { corner(lo, z[0], i) } else { corner(hi, z[1], i - 4) }) }
    }
    /// Face normals: bottom, then the four sides (top is parallel to bottom).
    fn normals(&self) -> [V3; 5] {
        let v = &self.v;
        let mut out = [cross(sub(v[1], v[0]), sub(v[3], v[0])); 5];
        for i in 0..4 {
            out[i + 1] = cross(sub(v[(i + 1) % 4], v[i]), sub(v[i + 4], v[i]));
        }
        out
    }
    /// Edge directions: two of the base (the top's are parallel), then the four slanted edges.
    fn edges(&self) -> [V3; 6] {
        let v = &self.v;
        [sub(v[1], v[0]), sub(v[3], v[0]), sub(v[4], v[0]), sub(v[5], v[1]), sub(v[6], v[2]), sub(v[7], v[3])]
    }
    fn project(&self, a: V3) -> (f64, f64) {
        self.v.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &p| {
            let d = dot(p, a);
            (lo.min(d), hi.max(d))
        })
    }
}

/// Penetration depth of two convex frusta (separating axis theorem over both bodies' face
/// normals and every pair of edge directions), or `None` when they are apart or touching.
fn overlap(a: &Body, b: &Body) -> Option<f64> {
    let mut depth = f64::INFINITY;
    let mut test = |axis: V3| -> bool {
        let len = dot(axis, axis).sqrt();
        if len < 1e-9 {
            return true;
        }
        let axis = [axis[0] / len, axis[1] / len, axis[2] / len];
        let ((a0, a1), (b0, b1)) = (a.project(axis), b.project(axis));
        let d = (a1 - b0).min(b1 - a0);
        depth = depth.min(d);
        d > 1e-3
    };
    for n in a.normals().into_iter().chain(b.normals()) {
        if !test(n) {
            return None;
        }
    }
    for ea in a.edges() {
        for eb in b.edges() {
            if !test(cross(ea, eb)) {
                return None;
            }
        }
    }
    Some(depth)
}

impl Geometry {
    /// Overlapping keys in 3D (`a < b`, physical keys only): each key is its keycap (footprint
    /// grown by `margin`, top inset by [`CAP_TAPER`], [`CAP_HEIGHT`] tall) plus a switch body
    /// ([`SWITCH_HALF`] + `margin`, [`SWITCH_DEPTH`] below the cap). Depth is the deepest
    /// overlap among the bodies. Approximate shapes (H8 refines them); flat boards should use
    /// [`Geometry::collisions`], which this matches for flat keycaps.
    pub fn collisions_3d(&self, margin: f32) -> Vec<Collision> {
        let m = margin as f64;
        let h = CAP_HEIGHT / 2.0;
        let keys: Vec<usize> = (0..self.len()).filter(|&i| self.key[i].phys.is_none()).collect();
        let bodies: Vec<(V3, f64, [Body; 2])> = keys
            .iter()
            .map(|&i| {
                let k = &self.key[i];
                let r = pose::rotation(k.rot);
                // Centre of the keycap box, as in `pose::contact`.
                let c = [k.pos[0] as f64 + k.size[0] as f64 / 2.0, k.pos[1] as f64 + k.size[1] as f64 / 2.0, k.pos[2] as f64 + h];
                let half = [k.size[0] as f64 / 2.0 + m, k.size[1] as f64 / 2.0 + m];
                let top = [(half[0] - CAP_TAPER).max(0.05), (half[1] - CAP_TAPER).max(0.05)];
                let sw = [SWITCH_HALF + m; 2];
                let cap = Body::new(c, &r, half, top, [-h, h]);
                let switch = Body::new(c, &r, sw, sw, [-h - SWITCH_DEPTH, -h]);
                let radius = (half[0].hypot(half[1])).max(sw[0] * 2f64.sqrt()).hypot(h + SWITCH_DEPTH);
                (c, radius, [cap, switch])
            })
            .collect();
        let mut out = Vec::new();
        for i in 0..bodies.len() {
            for j in i + 1..bodies.len() {
                let (ci, ri, bi) = &bodies[i];
                let (cj, rj, bj) = &bodies[j];
                let d = sub(*ci, *cj);
                if dot(d, d) >= (ri + rj) * (ri + rj) {
                    continue;
                }
                let depth = bi.iter().flat_map(|x| bj.iter().filter_map(move |y| overlap(x, y))).fold(0.0, f64::max);
                if depth > 0.0 {
                    out.push(Collision { a: keys[i], b: keys[j], depth: depth as f32 });
                }
            }
        }
        out
    }
}
