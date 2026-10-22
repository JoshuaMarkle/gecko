//! G04 keycap collision in 2D (`Geometry::collisions`): each keycap is an oriented box
//! (its footprint grown by a margin), pairs come from a uniform grid broad phase, and the
//! separating axis theorem (`flat2d::mtv`) gives the overlap depth. Flat boards only; 3D
//! boards need keycap and switch volumes (H8). See `context/geometry.md`.

use serde::Serialize;

use crate::geometry::flat2d::{mtv, Rect};
use crate::Geometry;

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
