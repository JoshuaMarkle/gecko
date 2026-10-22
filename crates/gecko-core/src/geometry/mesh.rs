//! Triangle meshes for STL export (J4): closed convex shells from point sets (incremental 3D
//! hull), point-inside tests, and binary STL. A part is a list of convex shells that may
//! overlap; slicers union overlapping closed shells. See `context/geometry.md` (STL).

use crate::geometry::flat2d::{cross, dot, sub, V3};

/// A closed convex shell: outward-wound triangles over `pts` (millimetres, right-handed, z up).
#[derive(Clone, Debug)]
pub struct Shell {
    pub pts: Vec<V3>,
    pub tris: Vec<[u32; 3]>,
}

/// Points closer than this (mm) are one point; points within it of a face are on the face.
const EPS: f64 = 1e-6;

impl Shell {
    /// Convex hull of `pts`; `None` when they span no volume (fewer than 4 points off a plane).
    pub fn hull(pts: &[V3]) -> Option<Shell> {
        let mut p: Vec<V3> = Vec::with_capacity(pts.len());
        for &q in pts {
            if q.iter().all(|c| c.is_finite()) && !p.iter().any(|&r| len(sub(q, r)) < EPS) {
                p.push(q);
            }
        }
        let t = initial(&p)?;
        let mut tris: Vec<[usize; 3]> = vec![[t[0], t[1], t[2]], [t[0], t[3], t[1]], [t[1], t[3], t[2]], [t[2], t[3], t[0]]];
        // Orient the tetrahedron outward: its fourth vertex must be behind the first face.
        if side(&p, tris[0], t[3]) > 0.0 {
            for f in &mut tris {
                f.swap(1, 2);
            }
        }
        for i in 0..p.len() {
            if t.contains(&i) {
                continue;
            }
            let visible: Vec<bool> = tris.iter().map(|&f| side(&p, f, i) > EPS).collect();
            if !visible.iter().any(|&v| v) {
                continue;
            }
            // Horizon: edges of visible faces whose reverse belongs to a hidden face.
            let mut horizon = Vec::new();
            for (f, _) in tris.iter().zip(&visible).filter(|(_, &v)| v) {
                for e in 0..3 {
                    let (a, b) = (f[e], f[(e + 1) % 3]);
                    let twin_hidden = tris.iter().zip(&visible).any(|(g, &v)| !v && (0..3).any(|k| g[k] == b && g[(k + 1) % 3] == a));
                    if twin_hidden {
                        horizon.push([a, b, i]);
                    }
                }
            }
            let mut keep: Vec<[usize; 3]> = tris.iter().zip(&visible).filter(|(_, &v)| !v).map(|(f, _)| *f).collect();
            keep.extend(horizon);
            tris = keep;
        }
        // Keep only the vertices the hull uses.
        let mut map = vec![u32::MAX; p.len()];
        let mut out = Vec::new();
        for f in &tris {
            for &v in f {
                if map[v] == u32::MAX {
                    map[v] = out.len() as u32;
                    out.push(p[v]);
                }
            }
        }
        Some(Shell { pts: out, tris: tris.iter().map(|f| [map[f[0]], map[f[1]], map[f[2]]]).collect() })
    }

    /// Axis-aligned box from `lo` to `hi`.
    pub fn cuboid(lo: V3, hi: V3) -> Option<Shell> {
        let pts: Vec<V3> = (0..8).map(|i| [[lo, hi][i & 1][0], [lo, hi][(i >> 1) & 1][1], [lo, hi][(i >> 2) & 1][2]]).collect();
        Shell::hull(&pts)
    }

    /// Strictly inside, at least `margin` mm from every face.
    pub fn contains(&self, q: V3, margin: f64) -> bool {
        self.tris.iter().all(|t| {
            let [a, b, c] = t.map(|i| self.pts[i as usize]);
            let n = cross(sub(b, a), sub(c, a));
            let l = len(n);
            l < EPS || dot(n, sub(q, a)) / l < -margin
        })
    }

    /// Enclosed volume, mm³ (positive for outward winding).
    pub fn volume(&self) -> f64 {
        self.tris
            .iter()
            .map(|t| {
                let [a, b, c] = t.map(|i| self.pts[i as usize]);
                dot(a, cross(b, c)) / 6.0
            })
            .sum()
    }

    /// `[min, max]` corners.
    pub fn bounds(&self) -> [V3; 2] {
        bounds(self.pts.iter().copied())
    }
}

/// A part: shells written into one STL.
#[derive(Clone, Debug, Default)]
pub struct Mesh {
    pub shells: Vec<Shell>,
}

impl Mesh {
    pub fn triangles(&self) -> usize {
        self.shells.iter().map(|s| s.tris.len()).sum()
    }

    pub fn bounds(&self) -> [V3; 2] {
        bounds(self.shells.iter().flat_map(|s| s.pts.iter().copied()))
    }

    /// Binary STL: 80-byte header, triangle count, then per triangle a normal, three vertices
    /// (little-endian f32) and a zero attribute word.
    pub fn stl(&self, header: &str) -> Vec<u8> {
        let mut out = Vec::with_capacity(84 + 50 * self.triangles());
        let mut h = [b' '; 80];
        for (d, s) in h.iter_mut().zip(header.bytes()) {
            *d = s;
        }
        out.extend_from_slice(&h);
        out.extend_from_slice(&(self.triangles() as u32).to_le_bytes());
        for s in &self.shells {
            for t in &s.tris {
                let [a, b, c] = t.map(|i| s.pts[i as usize]);
                let n = cross(sub(b, a), sub(c, a));
                let l = len(n).max(1e-30);
                for v in [[n[0] / l, n[1] / l, n[2] / l], a, b, c] {
                    for x in v {
                        out.extend_from_slice(&(x as f32).to_le_bytes());
                    }
                }
                out.extend_from_slice(&[0, 0]);
            }
        }
        out
    }
}

fn len(a: V3) -> f64 {
    dot(a, a).sqrt()
}

fn bounds(pts: impl Iterator<Item = V3>) -> [V3; 2] {
    let mut b = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    for p in pts {
        for a in 0..3 {
            b[0][a] = b[0][a].min(p[a]);
            b[1][a] = b[1][a].max(p[a]);
        }
    }
    b
}

/// Signed distance-like value of point `i` above face `f` (positive = in front, outside).
fn side(p: &[V3], f: [usize; 3], i: usize) -> f64 {
    let n = cross(sub(p[f[1]], p[f[0]]), sub(p[f[2]], p[f[0]]));
    let l = len(n);
    if l < 1e-12 {
        return 0.0;
    }
    dot(n, sub(p[i], p[f[0]])) / l
}

/// Four points spanning a tetrahedron of real volume, or `None`.
fn initial(p: &[V3]) -> Option<[usize; 4]> {
    if p.len() < 4 {
        return None;
    }
    let a = (0..p.len()).min_by(|&i, &j| p[i][0].total_cmp(&p[j][0]))?;
    let far = |score: &dyn Fn(V3) -> f64| (0..p.len()).max_by(|&i, &j| score(p[i]).total_cmp(&score(p[j])));
    let b = far(&|q| len(sub(q, p[a])))?;
    let ab = sub(p[b], p[a]);
    if len(ab) < EPS {
        return None;
    }
    let c = far(&|q| len(cross(ab, sub(q, p[a]))))?;
    let n = cross(ab, sub(p[c], p[a]));
    if len(n) < EPS * len(ab) {
        return None;
    }
    let d = far(&|q| dot(n, sub(q, p[a])).abs())?;
    if dot(n, sub(p[d], p[a])).abs() / len(n) < EPS * 10.0 {
        return None;
    }
    Some([a, b, c, d])
}

/// Every edge appears once in each direction: closed and consistently wound.
pub fn closed(s: &Shell) -> bool {
    let mut e: std::collections::BTreeMap<(u32, u32), i32> = std::collections::BTreeMap::new();
    for t in &s.tris {
        for k in 0..3 {
            *e.entry((t[k], t[(k + 1) % 3])).or_default() += 1;
        }
    }
    e.iter().all(|(&(a, b), &n)| n == 1 && e.get(&(b, a)) == Some(&1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cube_hull_is_closed_outward_and_drops_interior_points() {
        let mut pts: Vec<V3> = (0..8).map(|i| [(i & 1) as f64, ((i >> 1) & 1) as f64, ((i >> 2) & 1) as f64]).collect();
        pts.push([0.5, 0.5, 0.5]);
        pts.push([0.5, 0.5, 1.0]); // on a face
        let s = Shell::hull(&pts).unwrap();
        assert!(closed(&s));
        assert!((s.volume() - 1.0).abs() < 1e-9);
        assert_eq!(s.pts.len(), 8);
        assert!(s.contains([0.5, 0.5, 0.5], 0.1));
        assert!(!s.contains([0.5, 0.5, 0.95], 0.1));
        assert!(!s.contains([1.5, 0.5, 0.5], 0.0));
    }

    #[test]
    fn random_clouds_hull_closed_and_contain_their_points() {
        let mut r = crate::rng::Rng::new(7);
        for _ in 0..50 {
            let pts: Vec<V3> = (0..30).map(|_| [r.f64() * 10.0, r.f64() * 3.0, r.f64() * 7.0]).collect();
            let s = Shell::hull(&pts).unwrap();
            assert!(closed(&s));
            assert!(s.volume() > 0.0);
            assert!(pts.iter().all(|&q| s.contains(q, -1e-6)), "every input point is inside or on the hull");
        }
    }

    #[test]
    fn flat_points_have_no_hull() {
        let pts = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 0.0]];
        assert!(Shell::hull(&pts).is_none());
    }

    #[test]
    fn stl_size_and_count() {
        let m = Mesh { shells: vec![Shell::cuboid([0.0; 3], [1.0, 2.0, 3.0]).unwrap()] };
        let b = m.stl("gecko");
        assert_eq!(m.triangles(), 12);
        assert_eq!(b.len(), 84 + 50 * 12);
        assert_eq!(u32::from_le_bytes(b[80..84].try_into().unwrap()), 12);
    }
}
