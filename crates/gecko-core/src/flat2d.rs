//! A clean 2D drawing of any board (`Geometry::flat2d`): keys projected onto the best-fit
//! plane (per half for tented split boards), then pushed apart until no two keycaps overlap.
//! Flat boards come back unchanged. Used by the web's 2D keyboards and image export.

use serde::{Deserialize, Serialize};

use crate::geometry::{Geometry, Hand};

/// A key in the 2D drawing: `pos` is the unrotated top-left corner (same convention as
/// `Key::pos`), `rot` the rotation in degrees about the key center (like `Key::rot[2]`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Flat {
    pub pos: [f32; 2],
    pub rot: f32,
}

/// Halves whose best-fit planes differ by more than this are projected separately.
const SPLIT_DEG: f64 = 5.0;
/// Overlap depth (key units) below which two keys count as touching, not overlapping.
const EPS: f64 = 1e-3;
const MAX_PASSES: usize = 500;

pub(crate) type V3 = [f64; 3];

pub(crate) fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub(crate) fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub(crate) fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
pub(crate) fn norm(a: V3) -> V3 {
    let l = dot(a, a).sqrt();
    if l < 1e-12 {
        a
    } else {
        [a[0] / l, a[1] / l, a[2] / l]
    }
}

/// The key's local x axis in board space. Rotation as the 3D viewer applies it (three.js
/// Euler XYZ over `(rot.x, -rot.z, rot.y)` in three's y-up frame, board y = three z).
fn key_x_axis(rot: [f32; 3]) -> V3 {
    let d = std::f64::consts::PI / 180.0;
    let (a, b, c) = (rot[0] as f64 * d, -rot[2] as f64 * d, rot[1] as f64 * d);
    let v = [c.cos(), c.sin(), 0.0]; // Rz
    let v = [b.cos() * v[0] + b.sin() * v[2], v[1], -b.sin() * v[0] + b.cos() * v[2]]; // Ry
    let v = [v[0], a.cos() * v[1] - a.sin() * v[2], a.sin() * v[1] + a.cos() * v[2]]; // Rx
    [v[0], v[2], v[1]]
}

/// Eigenvalues (ascending) and the eigenvector of the smallest one of a symmetric 3×3 matrix
/// (cyclic Jacobi).
fn eig_min(mut m: [[f64; 3]; 3]) -> ([f64; 3], V3) {
    let mut v = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for _ in 0..50 {
        let off = m[0][1].abs() + m[0][2].abs() + m[1][2].abs();
        if off < 1e-14 {
            break;
        }
        for (p, q) in [(0, 1), (0, 2), (1, 2)] {
            if m[p][q].abs() < 1e-18 {
                continue;
            }
            let theta = (m[q][q] - m[p][p]) / (2.0 * m[p][q]);
            let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
            let t = if theta == 0.0 { 1.0 } else { t };
            let (c, s) = (1.0 / (t * t + 1.0).sqrt(), t / (t * t + 1.0).sqrt());
            for k in 0..3 {
                let (mkp, mkq) = (m[k][p], m[k][q]);
                m[k][p] = c * mkp - s * mkq;
                m[k][q] = s * mkp + c * mkq;
            }
            for k in 0..3 {
                let (mpk, mqk) = (m[p][k], m[q][k]);
                m[p][k] = c * mpk - s * mqk;
                m[q][k] = s * mpk + c * mqk;
            }
            for k in 0..3 {
                let (vkp, vkq) = (v[k][p], v[k][q]);
                v[k][p] = c * vkp - s * vkq;
                v[k][q] = s * vkp + c * vkq;
            }
        }
    }
    let mut ev = [m[0][0], m[1][1], m[2][2]];
    let i = (0..3).min_by(|&a, &b| ev[a].total_cmp(&ev[b])).unwrap();
    let vec = [v[0][i], v[1][i], v[2][i]];
    ev.sort_by(f64::total_cmp);
    (ev, vec)
}

/// Best-fit plane normal of `pts` (pointing up, +z). Fewer than 3 points or collinear points
/// give the board plane.
pub(crate) fn plane_normal(pts: &[V3]) -> V3 {
    let n = pts.len() as f64;
    if pts.len() < 3 {
        return [0.0, 0.0, 1.0];
    }
    let c = pts.iter().fold([0.0; 3], |a, p| [a[0] + p[0] / n, a[1] + p[1] / n, a[2] + p[2] / n]);
    let mut m = [[0.0; 3]; 3];
    for p in pts {
        let d = sub(*p, c);
        for i in 0..3 {
            for j in 0..3 {
                m[i][j] += d[i] * d[j];
            }
        }
    }
    let (ev, normal) = eig_min(m);
    // Second-smallest eigenvalue ~0: the points are collinear and the plane is undefined.
    if ev[1] <= 1e-9 * ev[2] {
        return [0.0, 0.0, 1.0];
    }
    let normal = norm(normal);
    if normal[2] < 0.0 {
        [-normal[0], -normal[1], -normal[2]]
    } else {
        normal
    }
}

/// Oriented key rectangle in 2D: centre, half extents, rotation (degrees).
#[derive(Clone, Copy)]
pub(crate) struct Rect {
    pub c: [f64; 2],
    pub h: [f64; 2],
    pub rot: f64,
}

impl Rect {
    fn axes(&self) -> [[f64; 2]; 2] {
        let (s, c) = self.rot.to_radians().sin_cos();
        [[c, s], [-s, c]]
    }
    /// Half extent of the rectangle projected on unit axis `a`.
    fn radius(&self, a: [f64; 2]) -> f64 {
        let [u, v] = self.axes();
        self.h[0] * (u[0] * a[0] + u[1] * a[1]).abs() + self.h[1] * (v[0] * a[0] + v[1] * a[1]).abs()
    }
}

/// Minimum translation vector that moves `b` off `a` (separating axis theorem), or `None`
/// when they overlap by at most `EPS`. Points from `a` toward `b`.
pub(crate) fn mtv(a: &Rect, b: &Rect) -> Option<[f64; 2]> {
    let d = [b.c[0] - a.c[0], b.c[1] - a.c[1]];
    let mut best: Option<(f64, [f64; 2])> = None;
    for ax in a.axes().into_iter().chain(b.axes()) {
        let dist = d[0] * ax[0] + d[1] * ax[1];
        let depth = a.radius(ax) + b.radius(ax) - dist.abs();
        if depth <= EPS {
            return None;
        }
        if best.is_none_or(|(bd, _)| depth < bd) {
            let s = if dist < 0.0 { -1.0 } else { 1.0 };
            best = Some((depth, [ax[0] * s, ax[1] * s]));
        }
    }
    best.map(|(depth, ax)| [ax[0] * depth, ax[1] * depth])
}

/// Push overlapping rectangles apart, half the MTV each, until none overlap. Each push moves
/// the pair away from each other along the separating axis, so keys never pass one another
/// and row/column order is kept.
fn separate(rects: &mut [Rect]) {
    for _ in 0..MAX_PASSES {
        let mut moved = false;
        for i in 0..rects.len() {
            for j in i + 1..rects.len() {
                if let Some(t) = mtv(&rects[i], &rects[j]) {
                    // Overshoot slightly so the pair ends clear of EPS.
                    let k = 0.5 + EPS;
                    rects[i].c = [rects[i].c[0] - t[0] * k, rects[i].c[1] - t[1] * k];
                    rects[j].c = [rects[j].c[0] + t[0] * k, rects[j].c[1] + t[1] * k];
                    moved = true;
                }
            }
        }
        if !moved {
            return;
        }
    }
}

impl Geometry {
    /// True when every key lies in the z = 0 plane with only z-rotation.
    pub fn is_flat(&self) -> bool {
        self.key.iter().all(|k| k.pos[2] == 0.0 && k.rot[0] == 0.0 && k.rot[1] == 0.0)
    }

    /// A 2D drawing of the board, one entry per key. Flat boards return their own x, y and
    /// z-rotation. Otherwise key centers are projected onto the best-fit (PCA) plane, per half
    /// when the halves' planes differ (tented splits), keeping each key's in-plane rotation and
    /// each half's centroid; overlaps are then removed. Virtual layer keys copy their
    /// physical key.
    pub fn flat2d(&self) -> Vec<Flat> {
        if self.is_flat() {
            return self.key.iter().map(|k| Flat { pos: [k.pos[0], k.pos[1]], rot: k.rot[2] }).collect();
        }
        let phys: Vec<usize> = (0..self.len()).filter(|&i| self.key[i].phys.is_none()).collect();
        let center = |i: usize| {
            let k = &self.key[i];
            [k.pos[0] as f64 + k.size[0] as f64 / 2.0, k.pos[1] as f64 + k.size[1] as f64 / 2.0, k.pos[2] as f64]
        };
        let half = |h: Hand| phys.iter().copied().filter(|&i| self.key[i].hand() == h).collect::<Vec<_>>();
        let (l, r) = (half(Hand::L), half(Hand::R));
        let pts = |ks: &[usize]| ks.iter().map(|&i| center(i)).collect::<Vec<_>>();
        let (nl, nr) = (plane_normal(&pts(&l)), plane_normal(&pts(&r)));
        let split = !l.is_empty() && !r.is_empty() && dot(nl, nr).clamp(-1.0, 1.0).acos().to_degrees() > SPLIT_DEG;
        let groups = if split { vec![l, r] } else { vec![phys.clone()] };

        let mut rects: Vec<Option<Rect>> = vec![None; self.len()];
        for g in groups.iter().filter(|g| !g.is_empty()) {
            let p = pts(g);
            let n = plane_normal(&p);
            let len = p.len() as f64;
            let c = p.iter().fold([0.0; 3], |a, q| [a[0] + q[0] / len, a[1] + q[1] / len, a[2] + q[2] / len]);
            // In-plane axes: board x and y projected onto the plane (x exact, y completes it).
            let u = norm(sub([1.0, 0.0, 0.0], [n[0] * n[0], n[0] * n[1], n[0] * n[2]]));
            let u = if dot(u, u) < 0.5 { norm(cross([0.0, 1.0, 0.0], n)) } else { u };
            let v = cross(n, u);
            for (&i, q) in g.iter().zip(&p) {
                let d = sub(*q, c);
                let k = &self.key[i];
                let a = key_x_axis(k.rot);
                rects[i] = Some(Rect {
                    c: [c[0] + dot(d, u), c[1] + dot(d, v)],
                    h: [k.size[0] as f64 / 2.0, k.size[1] as f64 / 2.0],
                    rot: dot(a, v).atan2(dot(a, u)).to_degrees(),
                });
            }
        }
        let mut solid: Vec<Rect> = phys.iter().map(|&i| rects[i].unwrap()).collect();
        separate(&mut solid);
        for (&i, r) in phys.iter().zip(solid) {
            rects[i] = Some(r);
        }
        (0..self.len())
            .map(|i| {
                let r = rects[self.phys(i)].unwrap();
                Flat { pos: [(r.c[0] - r.h[0]) as f32, (r.c[1] - r.h[1]) as f32], rot: r.rot as f32 }
            })
            .collect()
    }
}

/// Whether two drawn keys (with their sizes) overlap by more than a hair.
pub fn overlaps(a: (&Flat, [f32; 2]), b: (&Flat, [f32; 2])) -> bool {
    let rect = |f: &Flat, s: [f32; 2]| Rect {
        c: [f.pos[0] as f64 + s[0] as f64 / 2.0, f.pos[1] as f64 + s[1] as f64 / 2.0],
        h: [s[0] as f64 / 2.0, s[1] as f64 / 2.0],
        rot: f.rot as f64,
    };
    mtv(&rect(a.0, a.1), &rect(b.0, b.1)).is_some()
}
