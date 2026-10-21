//! 3D key poses: the rotation convention shared with the web viewer, keycap contact points,
//! and per-hand measuring frames so metrics work on curved and tented boards.
//!
//! Board space: x right, y toward the user, z up, in key units. `Key::rot = [rx, ry, rz]`
//! (degrees) is the matrix `Rx(rx) · Rz(rz) · Ry(ry)` (tent first, then splay, then tilt),
//! which is what the viewer's three.js Euler XYZ over `(rx, −rz, ry)` draws. Positive `rx`
//! turns the keycap top toward the user, positive `ry` raises the key's +x side, positive
//! `rz` turns it clockwise seen from above. See `context/geometry.md`.

use crate::flat2d::{cross, dot, norm, plane_normal, sub, V3};
use crate::geometry::{Geometry, Hand, Key};

pub type M3 = [[f64; 3]; 3];

/// Keycap height in key units, as the web viewer draws it (≈ 8 mm). A key's `pos` z is the
/// bottom of the cap; the finger touches the centre of the top, `CAP_HEIGHT` above it.
pub const CAP_HEIGHT: f64 = 0.42;

pub const IDENTITY: M3 = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

pub fn rot_x(deg: f64) -> M3 {
    let (s, c) = deg.to_radians().sin_cos();
    [[1.0, 0.0, 0.0], [0.0, c, s], [0.0, -s, c]]
}
pub fn rot_y(deg: f64) -> M3 {
    let (s, c) = deg.to_radians().sin_cos();
    [[c, 0.0, -s], [0.0, 1.0, 0.0], [s, 0.0, c]]
}
pub fn rot_z(deg: f64) -> M3 {
    let (s, c) = deg.to_radians().sin_cos();
    [[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]]
}

pub fn mul(a: &M3, b: &M3) -> M3 {
    let mut m = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            m[i][j] = (0..3).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    m
}

pub fn apply(m: &M3, v: V3) -> V3 {
    [dot(m[0], v), dot(m[1], v), dot(m[2], v)]
}

pub fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

/// Rotation matrix of `Key::rot`.
pub fn rotation(rot: [f32; 3]) -> M3 {
    mul(&mul(&rot_x(rot[0] as f64), &rot_z(rot[2] as f64)), &rot_y(rot[1] as f64))
}

/// `Key::rot` angles (degrees) of a rotation matrix; inverse of [`rotation`].
pub fn euler(m: &M3) -> [f64; 3] {
    let sz = (-m[0][1]).clamp(-1.0, 1.0);
    let rz = sz.asin();
    if rz.cos() > 1e-9 {
        [(-m[2][1]).atan2(m[1][1]).to_degrees(), (-m[0][2]).atan2(m[0][0]).to_degrees(), rz.to_degrees()]
    } else {
        // Gimbal lock (rz = ±90°): put the whole remaining rotation into ry.
        [0.0, m[2][0].atan2(m[2][2]).to_degrees(), rz.to_degrees()]
    }
}

/// Mirror a rotation across the x = const plane (left hand ↔ right hand).
pub fn mirror_rot(rot: [f64; 3]) -> [f64; 3] {
    [rot[0], -rot[1], -rot[2]]
}

/// Centre of the keycap's top surface, where the fingertip presses (board space).
pub fn contact(k: &Key) -> V3 {
    let m = rotation(k.rot);
    let h = CAP_HEIGHT / 2.0;
    let mid = [k.pos[0] as f64 + k.size[0] as f64 / 2.0, k.pos[1] as f64 + k.size[1] as f64 / 2.0, k.pos[2] as f64 + h];
    add(mid, apply(&m, [0.0, 0.0, h]))
}

/// Unit normal of the keycap top (the press direction, reversed).
pub fn normal(k: &Key) -> V3 {
    apply(&rotation(k.rot), [0.0, 0.0, 1.0])
}

/// `Key::pos` for a key whose top centre is `top`, with rotation `m` and footprint `size`
/// (inverse of [`contact`]).
pub fn pos_from_contact(top: V3, m: &M3, size: [f64; 2]) -> V3 {
    let h = CAP_HEIGHT / 2.0;
    let t = apply(m, [0.0, 0.0, h]);
    [top[0] - t[0] - size[0] / 2.0, top[1] - t[1] - size[1] / 2.0, top[2] - t[2] - h]
}

impl Geometry {
    /// Points the geometric metrics measure between, one per key (key units).
    ///
    /// Flat boards: exactly `pos`, so their scores are untouched. Other boards: each key's
    /// top-centre contact point, offset like `pos` (minus half its size), expressed in its
    /// hand's frame: the best-fit plane of that hand's finger keys, x along board x and y
    /// toward the user within the plane, z along the plane normal. Differences between two
    /// keys of one hand are then true 3D distances between fingertip targets, with "sideways"
    /// and "forward" measured as the hand sees them on a tented or curved half.
    pub fn metric_points(&self) -> Vec<[f32; 3]> {
        if self.is_flat() {
            return self.key.iter().map(|k| k.pos).collect();
        }
        let tops: Vec<V3> =
            self.key.iter().map(|k| sub(contact(k), [k.size[0] as f64 / 2.0, k.size[1] as f64 / 2.0, CAP_HEIGHT])).collect();
        let mut out: Vec<[f32; 3]> = tops.iter().map(|p| [p[0] as f32, p[1] as f32, p[2] as f32]).collect();
        for hand in [Hand::L, Hand::R] {
            let fit: Vec<V3> = (0..self.len())
                .filter(|&i| self.key[i].phys.is_none() && self.key[i].hand() == hand && !self.key[i].finger.is_thumb())
                .map(|i| tops[i])
                .collect();
            if fit.len() < 3 {
                continue;
            }
            let n = plane_normal(&fit);
            let len = fit.len() as f64;
            let c = fit.iter().fold([0.0; 3], |a, q| add(a, [q[0] / len, q[1] / len, q[2] / len]));
            let u = norm(sub([1.0, 0.0, 0.0], [n[0] * n[0], n[0] * n[1], n[0] * n[2]]));
            let u = if dot(u, u) < 0.5 { norm(cross([0.0, 1.0, 0.0], n)) } else { u };
            let v = cross(n, u);
            for i in (0..self.len()).filter(|&i| self.key[i].hand() == hand) {
                let d = sub(tops[i], c);
                let p = add(c, [dot(d, u), dot(d, v), dot(d, n)]);
                out[i] = [p[0] as f32, p[1] as f32, p[2] as f32];
            }
        }
        out
    }
}
