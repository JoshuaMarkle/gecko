//! H8 3D key collision (`Geometry::collisions_3d`, `Geometry::contacts_3d`): each physical key
//! is two convex hulls, its tapered keycap and the MX switch body below it. Candidate pairs
//! come from a uniform 3D grid over bounding spheres; GJK decides whether two hulls intersect
//! and EPA gives the penetration depth and the direction that separates them (the minimum
//! translation vector). `Geometry::repair` (I3) pushes overlapping keys apart along those
//! vectors. See `context/geometry.md`.

use crate::geometry::flat2d::{cross, dot, mtv, sub, Rect, V3};
use crate::geometry::pose::{self, CAP_HEIGHT};
use crate::Geometry;

use super::collision::Collision;

/// Keycap top inset per side, u: an SA/DSA-like cap narrows from its footprint (18 mm) to a
/// ≈ 12.5 mm top; 0.15 u keeps that taper on GECKO's 19.05 mm footprint.
pub const CAP_TAPER: f64 = 0.15;
/// MX switch body below the keycap: half width (15 mm housing) and depth from the cap's bottom
/// (≈ 3 mm to the plate plus the 5.5 mm bottom housing).
pub const SWITCH_HALF: f64 = 0.39;
pub const SWITCH_DEPTH: f64 = 0.45;
/// Finger access above each keycap: its top face extruded this far (u) along the key's normal
/// must hold no other key's cap or switch. ≈ 19 mm, a fingertip's thickness plus approach, so a
/// key buried under a neighbour (which no finger can press) counts as a collision.
pub const ACCESS_HEIGHT: f64 = 1.0;
/// A finger key's access also covers the finger lying back from the fingertip: the top face
/// swept this far (u, ≈ 29 mm: distal phalanx and part of the middle one) toward the user, rising
/// at 45° off the keycap. Keys may sit in front of a finger key (a Corne's thumbs, 1.3 u ahead),
/// but not raised into the finger's path.
pub const FINGER_LENGTH: f64 = 1.5;
/// Overlaps of at most this depth (u) count as touching, as in 2D.
const TOUCH: f64 = 1e-3;

/// A convex hull given by its vertices (board space, key units).
#[derive(Clone, Debug)]
pub struct Hull {
    pub v: Vec<V3>,
}

impl Hull {
    /// A frustum in a key's frame (centre `c`, rotation `m`): rectangle `lo` (half sizes) at
    /// local z `z[0]`, rectangle `hi` at `z[1]`.
    pub fn frustum(c: V3, m: &pose::M3, lo: [f64; 2], hi: [f64; 2], z: [f64; 2]) -> Hull {
        let mut v = Vec::with_capacity(8);
        for (h, z) in [(lo, z[0]), (hi, z[1])] {
            for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                v.push(pose::add(c, pose::apply(m, [sx * h[0], sy * h[1], z])));
            }
        }
        Hull { v }
    }

    fn support(&self, d: V3) -> V3 {
        let mut best = self.v[0];
        let mut bd = dot(best, d);
        for &p in &self.v[1..] {
            let pd = dot(p, d);
            if pd > bd {
                (best, bd) = (p, pd);
            }
        }
        best
    }

    fn centroid(&self) -> V3 {
        let n = self.v.len() as f64;
        let s = self.v.iter().fold([0.0; 3], |a, &p| pose::add(a, p));
        [s[0] / n, s[1] / n, s[2] / n]
    }
}

/// Support point of the Minkowski difference `a − b` in direction `d`.
fn support(a: &Hull, b: &Hull, d: V3) -> V3 {
    sub(a.support(d), b.support(neg(d)))
}

fn neg(a: V3) -> V3 {
    [-a[0], -a[1], -a[2]]
}

fn scale(a: V3, s: f64) -> V3 {
    [a[0] * s, a[1] * s, a[2] * s]
}

/// `(a × b) × c`: with `a = c` an edge, the direction from it toward `b`, perpendicular to it.
fn triple(a: V3, b: V3, c: V3) -> V3 {
    cross(cross(a, b), c)
}

/// GJK: a tetrahedron of `a − b` that contains the origin, or `None` when the hulls are apart
/// or only touch.
fn gjk(a: &Hull, b: &Hull) -> Option<[V3; 4]> {
    let mut d = sub(a.centroid(), b.centroid());
    if dot(d, d) < 1e-18 {
        d = [1.0, 0.0, 0.0];
    }
    let mut s: Vec<V3> = vec![support(a, b, d)];
    d = neg(s[0]);
    for _ in 0..64 {
        if dot(d, d) < 1e-18 {
            return None; // the origin lies on the simplex: touching
        }
        let p = support(a, b, d);
        if dot(p, d) <= 0.0 {
            return None;
        }
        s.push(p);
        if next_simplex(&mut s, &mut d) {
            return Some([s[0], s[1], s[2], s[3]]);
        }
    }
    None
}

/// Reduce simplex `s` (newest point last) to the feature nearest the origin and set the next
/// search direction `d`; true when the tetrahedron contains the origin.
fn next_simplex(s: &mut Vec<V3>, d: &mut V3) -> bool {
    match s.len() {
        2 => {
            let (b, a) = (s[0], s[1]);
            let (ab, ao) = (sub(b, a), neg(a));
            if dot(ab, ao) > 0.0 {
                *d = triple(ab, ao, ab);
                if dot(*d, *d) < 1e-18 {
                    // The origin is on the segment's line (symmetric pairs): any perpendicular.
                    let axis = if ab[0].abs() < 0.5 * dot(ab, ab).sqrt() { [1.0, 0.0, 0.0] } else { [0.0, 1.0, 0.0] };
                    *d = cross(ab, axis);
                }
            } else {
                *s = vec![a];
                *d = ao;
            }
            false
        }
        3 => {
            let (c, b, a) = (s[0], s[1], s[2]);
            let (ab, ac, ao) = (sub(b, a), sub(c, a), neg(a));
            let n = cross(ab, ac);
            if dot(cross(n, ac), ao) > 0.0 {
                if dot(ac, ao) > 0.0 {
                    *s = vec![c, a];
                    *d = triple(ac, ao, ac);
                } else {
                    *s = vec![b, a];
                    return next_simplex(s, d);
                }
            } else if dot(cross(ab, n), ao) > 0.0 {
                *s = vec![b, a];
                return next_simplex(s, d);
            } else if dot(n, ao) > 0.0 {
                *d = n;
            } else {
                *s = vec![b, c, a];
                *d = neg(n);
            }
            false
        }
        _ => {
            let (d0, c, b, a) = (s[0], s[1], s[2], s[3]);
            let (ab, ac, ad, ao) = (sub(b, a), sub(c, a), sub(d0, a), neg(a));
            let (abc, acd, adb) = (cross(ab, ac), cross(ac, ad), cross(ad, ab));
            // Each face's normal points away from the opposite vertex.
            let out = |n: V3, opp: V3| if dot(n, opp) > 0.0 { neg(n) } else { n };
            let (abc, acd, adb) = (out(abc, ad), out(acd, ab), out(adb, ac));
            if dot(abc, ao) > 0.0 {
                *s = vec![c, b, a];
                return next_simplex(s, d);
            }
            if dot(acd, ao) > 0.0 {
                *s = vec![d0, c, a];
                return next_simplex(s, d);
            }
            if dot(adb, ao) > 0.0 {
                *s = vec![b, d0, a];
                return next_simplex(s, d);
            }
            true
        }
    }
}

/// EPA from GJK's tetrahedron: penetration depth and the unit direction `n` such that moving
/// `b` by `depth · n` (or `a` by `−depth · n`) separates them.
fn epa(a: &Hull, b: &Hull, t: [V3; 4]) -> (f64, V3) {
    let mut v: Vec<V3> = t.to_vec();
    let mut faces: Vec<[usize; 3]> = vec![[0, 1, 2], [0, 3, 1], [0, 2, 3], [1, 3, 2]];
    // Outward unit normal and distance from the origin of face f (the origin is inside).
    let plane = |v: &[V3], f: [usize; 3]| -> (V3, f64) {
        let n = cross(sub(v[f[1]], v[f[0]]), sub(v[f[2]], v[f[0]]));
        let l = dot(n, n).sqrt();
        if l < 1e-15 {
            return ([0.0, 0.0, 1.0], f64::INFINITY);
        }
        let n = scale(n, 1.0 / l);
        let dist = dot(n, v[f[0]]);
        if dist < 0.0 {
            (neg(n), -dist)
        } else {
            (n, dist)
        }
    };
    // Orient the start faces outward (away from the tetrahedron's centroid).
    let c = scale(v.iter().fold([0.0; 3], |s, &p| pose::add(s, p)), 0.25);
    for f in &mut faces {
        let n = cross(sub(v[f[1]], v[f[0]]), sub(v[f[2]], v[f[0]]));
        if dot(n, sub(v[f[0]], c)) < 0.0 {
            f.swap(1, 2);
        }
    }
    let mut best = (f64::INFINITY, [0.0, 0.0, 1.0]);
    for _ in 0..64 {
        let (n, dist) = faces.iter().map(|&f| plane(&v, f)).min_by(|x, y| x.1.total_cmp(&y.1)).expect("faces");
        best = (dist, n);
        let p = support(a, b, n);
        if dot(p, n) - dist < 1e-9 {
            break;
        }
        // Remove every face p can see and stitch the horizon to p.
        let mut edges: Vec<[usize; 2]> = Vec::new();
        faces.retain(|&f| {
            let fn_ = cross(sub(v[f[1]], v[f[0]]), sub(v[f[2]], v[f[0]]));
            if dot(fn_, sub(p, v[f[0]])) > 0.0 {
                for e in [[f[0], f[1]], [f[1], f[2]], [f[2], f[0]]] {
                    match edges.iter().position(|&x| x == [e[1], e[0]]) {
                        Some(i) => {
                            edges.swap_remove(i);
                        }
                        None => edges.push(e),
                    }
                }
                false
            } else {
                true
            }
        });
        if edges.is_empty() {
            break;
        }
        v.push(p);
        let pi = v.len() - 1;
        faces.extend(edges.into_iter().map(|e| [e[0], e[1], pi]));
    }
    best
}

/// Penetration depth of two convex hulls and the unit direction `n` that separates them
/// (move `b` by `depth · n`), or `None` when they are apart or only touch (≤ 0.001 u).
pub fn penetration(a: &Hull, b: &Hull) -> Option<(f64, V3)> {
    let t = gjk(a, b)?;
    let (depth, n) = epa(a, b, t);
    (depth > TOUCH && depth.is_finite()).then_some((depth, n))
}

/// Two physical keys whose 3D volumes overlap, with the minimum translation that clears them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Contact {
    pub a: usize,
    pub b: usize,
    /// Penetration depth, u (the deepest of the keycap and switch body pairs).
    pub depth: f64,
    /// Unit direction: moving key `b` by `depth · normal` (or `a` by the opposite) clears that
    /// deepest pair.
    pub normal: V3,
}

/// A key's two solid hulls (keycap, switch body), grown by `margin` u per side.
pub fn key_hulls(k: &crate::geometry::Key, margin: f64) -> [Hull; 2] {
    let [cap, switch, _] = key_volumes(k, margin);
    [cap, switch]
}

/// [`key_hulls`] plus the key's finger access volume: its cap top (with margin) extruded
/// [`ACCESS_HEIGHT`] along the normal and, on finger keys, [`FINGER_LENGTH`] up and toward the
/// user (key frame +y), one convex wedge.
pub fn key_volumes(k: &crate::geometry::Key, margin: f64) -> [Hull; 3] {
    let h = CAP_HEIGHT / 2.0;
    let r = pose::rotation(k.rot);
    // Centre of the keycap box, as in `pose::contact`.
    let c = [k.pos[0] as f64 + k.size[0] as f64 / 2.0, k.pos[1] as f64 + k.size[1] as f64 / 2.0, k.pos[2] as f64 + h];
    let half = [k.size[0] as f64 / 2.0 + margin, k.size[1] as f64 / 2.0 + margin];
    let top = [(half[0] - CAP_TAPER).max(0.05), (half[1] - CAP_TAPER).max(0.05)];
    let sw = [SWITCH_HALF + margin; 2];
    let mut access = Hull::frustum(c, &r, top, top, [h, h + ACCESS_HEIGHT]);
    if !k.finger.is_thumb() {
        let back = pose::apply(&r, [0.0, FINGER_LENGTH / 2f64.sqrt(), FINGER_LENGTH / 2f64.sqrt()]);
        let swept: Vec<V3> = access.v[..4].iter().map(|&p| pose::add(p, back)).collect();
        access.v.extend(swept);
    }
    [Hull::frustum(c, &r, half, top, [-h, h]), Hull::frustum(c, &r, sw, sw, [-h - SWITCH_DEPTH, -h]), access]
}

/// Deepest overlap between keys `a` and `b` (volumes from [`key_volumes`]): solid against solid,
/// or one key's access volume against the other's solids. Two access volumes may overlap.
fn deepest(a: &[Hull; 3], b: &[Hull; 3]) -> Option<(f64, V3)> {
    const PAIRS: [(usize, usize); 6] = [(0, 0), (0, 1), (1, 0), (1, 1), (2, 0), (2, 1)];
    let ab = PAIRS.iter().filter_map(|&(x, y)| penetration(&a[x], &b[y]));
    // b's access volume against a's solids: same pairs swapped, normal kept as "move b".
    let ba = PAIRS[4..].iter().filter_map(|&(x, y)| penetration(&b[x], &a[y]).map(|(d, n)| (d, neg(n))));
    ab.chain(ba).max_by(|x, y| x.0.total_cmp(&y.0))
}

impl Geometry {
    /// Overlapping keys in 3D (`a < b`, physical keys only) with their minimum translation
    /// vectors. Each key is its keycap (footprint grown by `margin`, top inset by
    /// [`CAP_TAPER`], [`CAP_HEIGHT`] tall), a switch body ([`SWITCH_HALF`] + `margin`,
    /// [`SWITCH_DEPTH`] below the cap) and the finger access above its top ([`ACCESS_HEIGHT`]),
    /// which must be free of other keys. Pairs come from a uniform grid over bounding spheres.
    pub fn contacts_3d(&self, margin: f32) -> Vec<Contact> {
        let m = margin as f64;
        let keys: Vec<usize> = (0..self.len()).filter(|&i| self.key[i].phys.is_none()).collect();
        let hulls: Vec<[Hull; 3]> = keys.iter().map(|&i| key_volumes(&self.key[i], m)).collect();
        // Bounding sphere per key: centre of all its vertices, radius to the farthest one.
        let spheres: Vec<(V3, f64)> = hulls
            .iter()
            .map(|hs| {
                let pts = || hs.iter().flat_map(|h| h.v.iter().copied());
                let n = pts().count() as f64;
                let c = scale(pts().fold([0.0; 3], pose::add), 1.0 / n);
                let r = pts().map(|p| dot(sub(p, c), sub(p, c)).sqrt()).fold(0.0, f64::max);
                (c, r)
            })
            .collect();
        // Broad phase: cells as wide as the largest sphere diameter, so overlapping spheres have
        // centres in the same or neighbouring cells.
        let cell = spheres.iter().map(|s| 2.0 * s.1).fold(1e-6, f64::max);
        let at = |c: V3| [(c[0] / cell).floor() as i64, (c[1] / cell).floor() as i64, (c[2] / cell).floor() as i64];
        let mut grid: Vec<([i64; 3], usize)> = spheres.iter().enumerate().map(|(i, s)| (at(s.0), i)).collect();
        grid.sort_unstable();
        let mut out = Vec::new();
        for (i, &(ci, ri)) in spheres.iter().enumerate() {
            let g = at(ci);
            for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        let cellid = [g[0] + dx, g[1] + dy, g[2] + dz];
                        let start = grid.partition_point(|e| e.0 < cellid);
                        for &(_, j) in grid[start..].iter().take_while(|e| e.0 == cellid) {
                            let (cj, rj) = spheres[j];
                            let d = sub(ci, cj);
                            if j <= i || dot(d, d) >= (ri + rj) * (ri + rj) {
                                continue;
                            }
                            if let Some((depth, normal)) = deepest(&hulls[i], &hulls[j]) {
                                out.push(Contact { a: keys[i], b: keys[j], depth, normal });
                            }
                        }
                    }
                }
            }
        }
        out.sort_by_key(|c| (c.a, c.b));
        out
    }

    /// [`Geometry::contacts_3d`] without the directions: what the geometry search penalizes on
    /// non-flat boards. On flat boards it finds the pairs [`Geometry::collisions`] finds.
    pub fn collisions_3d(&self, margin: f32) -> Vec<Collision> {
        self.contacts_3d(margin).into_iter().map(|c| Collision { a: c.a, b: c.b, depth: c.depth as f32 }).collect()
    }
}

/// What [`Geometry::repair`] did.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Repair {
    /// Passes that moved keys.
    pub passes: usize,
    /// Largest distance any key moved, u.
    pub max_shift: f64,
    /// Overlapping pairs left after the last pass (0 unless `max_passes` ran out).
    pub remaining: usize,
}

impl Geometry {
    /// Push overlapping keys apart until none overlap (at `margin`) or `max_passes` run out
    /// (I3). Each pass takes the overlapping pairs (grid + GJK/EPA in 3D, the 2D test on flat
    /// boards) and moves each pair half its minimum translation vector apart (growing 5% per
    /// pass, which breaks jams that cycle), plus the touching tolerance, recomputing a pair's vector from the keys' current positions (Gauss–Seidel).
    /// Keys only translate: rotations stay, and a flat board stays flat (its keys move in x, y).
    /// Layer copies follow their physical key.
    pub fn repair(&mut self, margin: f32, max_passes: usize) -> Repair {
        let flat = self.is_flat();
        let start: Vec<[f32; 3]> = self.key.iter().map(|k| k.pos).collect();
        let mut pos: Vec<V3> = start.iter().map(|p| [p[0] as f64, p[1] as f64, p[2] as f64]).collect();
        let mut passes = 0;
        let mut remaining = 0;
        for pass in 0..=max_passes {
            let pairs: Vec<(usize, usize)> = if flat {
                self.collisions(margin).iter().map(|c| (c.a, c.b)).collect()
            } else {
                self.contacts_3d(margin).iter().map(|c| (c.a, c.b)).collect()
            };
            remaining = pairs.len();
            if pairs.is_empty() || pass == max_passes {
                break;
            }
            passes += 1;
            for (a, b) in pairs {
                let t = if flat { self.mtv_2d(a, b, margin) } else { self.mtv_3d(a, b, margin) };
                let Some((n, depth)) = t else { continue };
                // Jammed clusters can cycle (three keys pushing each other back); a step that
                // grows by 5% per pass breaks the cycle.
                let k = depth / 2.0 * (1.0 + 0.05 * pass as f64) + TOUCH;
                for (i, s) in [(a, -k), (b, k)] {
                    pos[i] = pose::add(pos[i], scale(n, s));
                    self.key[i].pos = [pos[i][0] as f32, pos[i][1] as f32, pos[i][2] as f32];
                }
            }
        }
        for i in 0..self.len() {
            if let Some(p) = self.key[i].phys {
                self.key[i].pos = self.key[p as usize].pos;
            }
        }
        let max_shift = self
            .key
            .iter()
            .zip(&start)
            .map(|(k, s)| {
                let d = [(k.pos[0] - s[0]) as f64, (k.pos[1] - s[1]) as f64, (k.pos[2] - s[2]) as f64];
                dot(d, d).sqrt()
            })
            .fold(0.0, f64::max);
        Repair { passes, max_shift, remaining }
    }

    /// Unit direction and depth that move key `b` clear of key `a` in 3D (deepest hull pair).
    fn mtv_3d(&self, a: usize, b: usize, margin: f32) -> Option<(V3, f64)> {
        let (ha, hb) = (key_volumes(&self.key[a], margin as f64), key_volumes(&self.key[b], margin as f64));
        deepest(&ha, &hb).map(|(d, n)| (n, d))
    }

    /// As [`Geometry::mtv_3d`] for flat keycaps in the board plane.
    fn mtv_2d(&self, a: usize, b: usize, margin: f32) -> Option<(V3, f64)> {
        let m = margin as f64;
        let rect = |k: &crate::geometry::Key| {
            let h = [k.size[0] as f64 / 2.0, k.size[1] as f64 / 2.0];
            Rect { c: [k.pos[0] as f64 + h[0], k.pos[1] as f64 + h[1]], h: [h[0] + m, h[1] + m], rot: k.rot[2] as f64 }
        };
        let t = mtv(&rect(&self.key[a]), &rect(&self.key[b]))?;
        let d = t[0].hypot(t[1]);
        Some(([t[0] / d, t[1] / d, 0.0], d))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Rng;

    /// Separating axis depth of two hulls given as frusta (face normals and edge × edge axes):
    /// the reference GJK + EPA must match.
    fn sat(a: &Hull, b: &Hull) -> Option<f64> {
        let normals = |h: &Hull| {
            let v = &h.v;
            let mut out = vec![cross(sub(v[1], v[0]), sub(v[3], v[0]))];
            out.extend((0..4).map(|i| cross(sub(v[(i + 1) % 4], v[i]), sub(v[i + 4], v[i]))));
            out
        };
        let edges = |h: &Hull| {
            let v = &h.v;
            vec![sub(v[1], v[0]), sub(v[3], v[0]), sub(v[4], v[0]), sub(v[5], v[1]), sub(v[6], v[2]), sub(v[7], v[3])]
        };
        let proj = |h: &Hull, ax: V3| {
            h.v.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &p| (lo.min(dot(p, ax)), hi.max(dot(p, ax))))
        };
        let mut axes = normals(a);
        axes.extend(normals(b));
        for ea in edges(a) {
            for eb in edges(b) {
                axes.push(cross(ea, eb));
            }
        }
        let mut depth = f64::INFINITY;
        for ax in axes {
            let l = dot(ax, ax).sqrt();
            if l < 1e-9 {
                continue;
            }
            let ax = scale(ax, 1.0 / l);
            let ((a0, a1), (b0, b1)) = (proj(a, ax), proj(b, ax));
            depth = depth.min((a1 - b0).min(b1 - a0));
        }
        (depth > TOUCH).then_some(depth)
    }

    fn random_frustum(rng: &mut Rng) -> Hull {
        let mut u = || rng.f64();
        let c = [u() * 1.2, u() * 1.2, u() * 0.8];
        let m = pose::rotation([(u() * 90.0 - 45.0) as f32, (u() * 90.0 - 45.0) as f32, (u() * 360.0) as f32]);
        let lo = [0.2 + u() * 0.4, 0.2 + u() * 0.4];
        let hi = [lo[0] * (0.4 + 0.6 * u()), lo[1] * (0.4 + 0.6 * u())];
        Hull::frustum(c, &m, lo, hi, [-0.2, 0.1 + u() * 0.3])
    }

    #[test]
    fn gjk_epa_matches_separating_axes() {
        let mut rng = Rng::new(7);
        let (mut hits, mut misses) = (0, 0);
        for _ in 0..4000 {
            let (a, b) = (random_frustum(&mut rng), random_frustum(&mut rng));
            let (got, want) = (penetration(&a, &b), sat(&a, &b));
            match (got, want) {
                (Some((d, n)), Some(w)) => {
                    hits += 1;
                    assert!((d - w).abs() < 1e-6, "depth {d} vs SAT {w}");
                    // Moving b by the MTV (plus a hair) clears the pair.
                    let moved = Hull { v: b.v.iter().map(|&p| pose::add(p, scale(n, d + 2.0 * TOUCH))).collect() };
                    assert!(sat(&a, &moved).is_none() && penetration(&a, &moved).is_none());
                }
                (None, None) => misses += 1,
                // Within the touching tolerance either may call it touching.
                (Some((d, _)), None) => assert!(d < 2.0 * TOUCH, "GJK {d}, SAT apart"),
                (None, Some(w)) => assert!(w < 2.0 * TOUCH, "GJK apart, SAT {w}"),
            }
        }
        assert!(hits > 500 && misses > 500, "{hits} hits, {misses} misses");
    }

    #[test]
    fn symmetric_and_coincident_pairs() {
        let box_at = |c: V3, rot: [f32; 3]| Hull::frustum(c, &pose::rotation(rot), [0.5, 0.5], [0.35, 0.35], [-0.2, 0.2]);
        // Mirror-symmetric: the origin lies on GJK's first segment.
        let (a, b) = (box_at([0.0, 0.0, 0.0], [0.0, 30.0, 0.0]), box_at([0.8, 0.0, 0.0], [0.0, -30.0, 0.0]));
        assert!((penetration(&a, &b).unwrap().0 - sat(&a, &b).unwrap()).abs() < 1e-9);
        // Coincident and stacked hulls, axis-aligned.
        let c = box_at([0.0, 0.0, 0.0], [0.0; 3]);
        assert!((penetration(&c, &c).unwrap().0 - 0.4).abs() < 1e-9);
        let (d, n) = penetration(&c, &box_at([0.0, 0.0, 0.3], [0.0; 3])).unwrap();
        assert!((d - 0.1).abs() < 1e-9 && (n[2] - 1.0).abs() < 1e-9, "{d} {n:?}");
        // Face to face exactly touching, and apart.
        assert!(penetration(&c, &box_at([1.0, 0.0, 0.0], [0.0; 3])).is_none());
        assert!(penetration(&c, &box_at([1.5, 0.2, 0.0], [0.0; 3])).is_none());
    }
}
