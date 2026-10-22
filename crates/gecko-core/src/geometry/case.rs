//! J4 plate and case generation (#75): a printable switch plate and shell from the key poses.
//!
//! Every part is a list of closed convex shells (`mesh.rs`), Dactyl style:
//! - a **tile** per physical key: a plate frame with a switch cutout at the key's pose, and
//!   under the clip layer a pocket widened along the key's y for the MX clips;
//! - a **web** between facing edges of neighbouring keys (gap ≤ `bridge`), and a **junction**
//!   fill where three or more keys' corners meet;
//! - a **wall** from every edge without a web straight down to the floor, its foot pushed
//!   outward, plus a **corner** wall joining neighbouring wall ends;
//! - a **bottom** plate per piece: the convex hull of the wall feet.
//!
//! The board is lifted so its lowest switch bottom sits `clearance` above the floor (z = 0).
//! Webs and junctions that would block a switch hole are dropped; anything else in a hole or
//! a keycap's travel is reported. Output is millimetres, right-handed (STL y = −board y).
//! See `context/geometry.md` (STL).

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::geometry::flat2d::{dot, norm, sub, V3};
use crate::geometry::manufacture::switch_centre;
use crate::geometry::mesh::{Mesh, Shell};
use crate::geometry::pose::{add, apply, rotation};
use crate::{Error, Geometry, Result};

/// Millimetres per key unit.
pub const U_MM: f64 = 19.05;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Switch {
    #[default]
    Mx,
    Choc,
}

/// Case dimensions, millimetres. [`CaseParams::for_switch`] gives the defaults.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseParams {
    pub switch: Switch,
    /// Square switch cutout (MX 14.0, Choc 13.8).
    pub hole: f64,
    /// Plate thickness where the switch clips catch (MX 1.5, Choc 1.3).
    pub clip: f64,
    /// Extra cutout length along the key's y below the clip layer, room for the clips.
    pub clip_pocket: f64,
    /// Plate thickness.
    pub plate: f64,
    /// Plate around the cutout; a 1u tile is `hole + 2·rim` wide (Dactyl: 17.4).
    pub rim: f64,
    /// Keycap bottom to plate top along the key normal, at rest.
    pub cap_gap: f64,
    /// Keycap travel, for the clearance check.
    pub travel: f64,
    /// Plate top to the bottom of the switch housing.
    pub switch_depth: f64,
    /// Floor to the lowest switch bottom (wiring room).
    pub clearance: f64,
    /// Wall thickness at the floor.
    pub wall: f64,
    /// How far a wall's foot moves outward from its plate edge.
    pub wall_offset: f64,
    /// Bottom plate thickness.
    pub bottom: f64,
    /// Largest gap between facing key edges that a web spans.
    pub bridge: f64,
}

impl Default for CaseParams {
    fn default() -> Self {
        CaseParams::for_switch(Switch::Mx)
    }
}

impl CaseParams {
    pub fn for_switch(switch: Switch) -> CaseParams {
        let mx = switch == Switch::Mx;
        CaseParams {
            switch,
            hole: if mx { 14.0 } else { 13.8 },
            clip: if mx { 1.5 } else { 1.3 },
            clip_pocket: if mx { 1.0 } else { 0.8 },
            plate: 3.0,
            rim: 1.7,
            cap_gap: if mx { 6.0 } else { 3.0 },
            travel: if mx { 4.0 } else { 3.0 },
            switch_depth: if mx { 5.0 } else { 3.0 },
            clearance: 4.0,
            wall: 2.5,
            wall_offset: 2.0,
            bottom: 2.0,
            bridge: 15.0,
        }
    }

    pub fn validate(&self) -> Result<()> {
        let v = [
            self.hole,
            self.clip,
            self.clip_pocket,
            self.plate,
            self.rim,
            self.cap_gap,
            self.travel,
            self.switch_depth,
            self.clearance,
            self.wall,
            self.wall_offset,
            self.bottom,
            self.bridge,
        ];
        if !v.iter().all(|x| x.is_finite() && *x >= 0.0) || self.hole <= 0.0 || self.plate <= 0.0 || self.rim <= 0.0 {
            return Err(Error::invalid("case", "dimensions must be finite and ≥ 0; hole, plate and rim > 0"));
        }
        Ok(())
    }
}

/// The generated parts.
#[derive(Clone, Debug)]
pub struct Case {
    /// Tiles, webs and junctions: the switch plate alone (a sandwich or hand-wired build).
    pub plate: Mesh,
    /// Walls and corner walls; the case is `plate` + `walls`.
    pub walls: Mesh,
    pub bottom: Mesh,
    pub report: CaseReport,
}

impl Case {
    /// Plate and walls together.
    pub fn case(&self) -> Mesh {
        Mesh { shells: self.plate.shells.iter().chain(&self.walls.shells).cloned().collect() }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CaseReport {
    /// Physical keys (switch cutouts).
    pub keys: usize,
    pub pieces: Vec<PieceReport>,
    pub webs: usize,
    pub junctions: usize,
    pub walls: usize,
    pub corners: usize,
    /// Webs, junctions and corner walls left out because they would block a switch hole.
    pub dropped: usize,
    /// Keys whose switch hole another shell still enters (a wall, or a neighbour too close).
    pub blocked: Vec<usize>,
    /// Keys whose keycap would hit a shell within its travel.
    pub cap_hits: Vec<usize>,
    /// How far the board was raised (mm, board z → STL z) to clear the floor.
    pub lift: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PieceReport {
    /// Physical key indices.
    pub keys: Vec<usize>,
    /// Bounding box `[x, y, z]`, mm, case and bottom together.
    pub size: [f64; 3],
}

/// A key's plate frame: plate-top centre `c` and axes (mm, board space after the lift).
#[derive(Clone, Copy)]
struct Tile {
    c: V3,
    x: V3,
    y: V3,
    n: V3,
    /// Half the outer tile size along x and y.
    half: [f64; 2],
}

/// Corner signs, counter-clockwise from (−x, −y); edge `e` runs from corner `e` to `e + 1`.
const CORNERS: [[f64; 2]; 4] = [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]];
/// Local outward normal of edge `e`.
const EDGE_NORMAL: [[f64; 2]; 4] = [[0.0, -1.0], [1.0, 0.0], [0.0, 1.0], [-1.0, 0.0]];

impl Tile {
    fn at(&self, u: f64, v: f64, w: f64) -> V3 {
        let s = |a: V3, k: f64| [a[0] * k, a[1] * k, a[2] * k];
        add(add(add(self.c, s(self.x, u)), s(self.y, v)), s(self.n, w))
    }

    /// Top and bottom of the plate at corner `i`.
    fn post(&self, i: usize, plate: f64) -> [V3; 2] {
        self.post_in(i, plate, 0.0)
    }

    /// [`Tile::post`] moved `inset` mm into the tile, so a connector overlaps the frame
    /// instead of sharing its vertices.
    fn post_in(&self, i: usize, plate: f64, inset: f64) -> [V3; 2] {
        let [su, sv] = CORNERS[i % 4];
        let (u, v) = (su * (self.half[0] - inset), sv * (self.half[1] - inset));
        [self.at(u, v, 0.0), self.at(u, v, -plate)]
    }

    fn edge_mid(&self, e: usize) -> V3 {
        let [nu, nv] = EDGE_NORMAL[e];
        self.at(nu * self.half[0], nv * self.half[1], 0.0)
    }

    /// Board-space outward direction of edge `e` (in the tile's plane).
    fn edge_dir(&self, e: usize) -> V3 {
        let [nu, nv] = EDGE_NORMAL[e];
        norm(sub(self.at(nu, nv, 0.0), self.c))
    }

    /// Top-surface end points of edge `e`.
    fn edge(&self, e: usize) -> [V3; 2] {
        [self.post(e, 0.0)[0], self.post(e + 1, 0.0)[0]]
    }
}

/// An edge end with a wall: key, edge, corner and the wall's outward floor direction.
struct WallEnd {
    key: usize,
    edge: usize,
    corner: usize,
    out: V3,
    /// Foot offset from the plate edge, mm (≥ `wall_offset`, more where a tilted switch hangs out).
    off: f64,
}

impl WallEnd {
    /// Plate posts (moved `inset` into the tile) and the two floor points below them, the
    /// wall's inner and outer foot, each pulled `inset` toward the other.
    fn points(&self, tiles: &[Tile], p: &CaseParams, inset: f64) -> [V3; 4] {
        let posts = tiles[self.key].post_in(self.corner, p.plate, inset);
        let base = tiles[self.key].post(self.corner, p.plate)[1];
        let foot = |o: f64| [base[0] + self.out[0] * o, base[1] + self.out[1] * o, 0.0];
        let pull = (inset / 4.0).min(p.wall / 3.0);
        [posts[0], posts[1], foot(self.off + pull), foot(self.off + p.wall - pull)]
    }
}

/// Distinct small insets, so no two connector shells share a vertex: slicers stitch an STL by
/// exact vertex coordinates, and shared vertices between separate shells make non-manifold edges.
struct Insets(usize);

impl Insets {
    fn next(&mut self, base: f64) -> f64 {
        self.0 += 1;
        base + 0.0005 * (self.0 % 250) as f64
    }
}

impl Geometry {
    /// Generate the plate, case walls and bottom plate. See the module docs.
    pub fn case(&self, p: &CaseParams, piece_gap: f64) -> Result<Case> {
        p.validate()?;
        let keys: Vec<usize> = (0..self.len()).filter(|&i| self.key[i].phys.is_none() && self.key[i].layer == 0).collect();
        if keys.is_empty() {
            return Err(Error::invalid("case", "the board has no keys"));
        }
        let mut tiles: Vec<Tile> = keys.iter().map(|&i| tile(&self.key[i], p)).collect();
        // Lift so the lowest switch bottom (or plate bottom) clears the floor.
        let h = p.hole / 2.0;
        let low = tiles
            .iter()
            .flat_map(|t| {
                let sw = CORNERS.map(|[a, b]| t.at(a * h, b * h, -p.switch_depth.max(p.plate))[2]);
                (0..4).map(move |i| t.post(i, p.plate)[1][2]).chain(sw)
            })
            .fold(f64::INFINITY, f64::min);
        let lift = p.clearance - low;
        for t in &mut tiles {
            t.c[2] += lift;
        }
        let sw: Vec<V3> = keys.iter().map(|&i| switch_centre(&self.key[i])).collect();
        let pieces = self.pieces(&keys, &sw, piece_gap);

        let mut plate = Mesh::default();
        let mut walls = Mesh::default();
        let mut bottom = Mesh::default();
        let mut rep = CaseReport { keys: keys.len(), lift, ..CaseReport::default() };
        // Webs (true) and junctions (false) until the hole check keeps or drops them.
        let mut connectors: Vec<(Shell, bool, usize)> = Vec::new();
        let mut boxes = vec![[[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]]; pieces.len()];
        let mut inset = Insets(0);
        let holes: Vec<Vec<V3>> = tiles.iter().map(|t| hole_samples(t, p)).collect();
        // Convex pieces of the plate for the hole checks, with the tile each belongs to.
        let mut solid: Vec<(Shell, Option<usize>)> = Vec::new();
        for (k, t) in tiles.iter().enumerate() {
            plate.shells.push(tile_frame(t, p));
            solid.extend(tile_boxes(t, p).into_iter().map(|s| (s, Some(k))));
        }
        for (pi, piece) in pieces.iter().enumerate() {
            let mut covered = vec![[false; 4]; tiles.len()];
            // Webs between facing edges.
            for (ai, &a) in piece.iter().enumerate() {
                for &b in &piece[ai + 1..] {
                    let close = dist(tiles[a].c, tiles[b].c) < 1.3 * U_MM;
                    let pair = facing(&tiles[a], &tiles[b], p.bridge).or_else(|| {
                        if close {
                            nearest_edges(&tiles[a], &tiles[b], p.bridge)
                        } else {
                            None
                        }
                    });
                    if let Some((ea, eb)) = pair {
                        let mut pts: Vec<V3> = Vec::new();
                        let d = inset.next(0.2);
                        for (t, e) in [(&tiles[a], ea), (&tiles[b], eb)] {
                            pts.extend(t.post_in(e, p.plate, d));
                            pts.extend(t.post_in(e + 1, p.plate, d));
                        }
                        if let Some(s) = hull(&pts) {
                            connectors.push((s, true, pi));
                        }
                        covered[a][ea] = true;
                        covered[b][eb] = true;
                    }
                }
            }
            // Junctions: corners of three or more keys within `bridge` of each other.
            let mut seen = BTreeSet::new();
            for &a in piece {
                for ca in 0..4 {
                    let pa = tiles[a].post(ca, 0.0)[0];
                    let mut group = vec![(a, ca)];
                    for &b in piece.iter().filter(|&&b| b != a) {
                        let (cb, d) = (0..4).map(|c| (c, dist(pa, tiles[b].post(c, 0.0)[0]))).min_by(|x, y| x.1.total_cmp(&y.1)).unwrap();
                        if d < p.bridge {
                            group.push((b, cb));
                        }
                    }
                    if group.len() < 3 {
                        continue;
                    }
                    group.sort();
                    if seen.insert(group.clone()) {
                        let d = inset.next(0.4);
                        let pts: Vec<V3> = group.iter().flat_map(|&(k, c)| tiles[k].post_in(c, p.plate, d)).collect();
                        if let Some(s) = hull(&pts) {
                            connectors.push((s, false, pi));
                        }
                    }
                }
            }
            // Walls from uncovered edges.
            let centroid = piece.iter().fold([0.0; 3], |s, &k| add(s, tiles[k].c));
            let centroid = centroid.map(|v| v / piece.len() as f64);
            let mut ends: Vec<WallEnd> = Vec::new();
            for &k in piece {
                let t = &tiles[k];
                for e in (0..4).filter(|&e| !covered[k][e]) {
                    let d = t.edge_dir(e);
                    let mut out = [d[0], d[1], 0.0];
                    if dot(out, out) < 0.04 {
                        let m = sub(t.edge_mid(e), centroid);
                        out = [m[0], m[1], 0.0];
                    }
                    let out = norm(out);
                    let d = inset.next(0.6);
                    // Push the foot out until the wall clears the piece's switches (a tilted key
                    // hangs its own switch past its edge; a curved column has keys below it).
                    let mut off = p.wall_offset;
                    let shell = loop {
                        let pts: Vec<V3> = [e, e + 1]
                            .iter()
                            .flat_map(|&c| WallEnd { key: k, edge: e, corner: c % 4, out, off }.points(&tiles, p, d))
                            .collect();
                        let s = hull(&pts);
                        let clear = s.as_ref().is_none_or(|s| !piece.iter().flat_map(|&j| &holes[j]).any(|&q| s.contains(q, 0.05)));
                        if clear || off > p.wall_offset + 30.0 {
                            break s;
                        }
                        off += 1.0;
                    };
                    ends.extend([e, e + 1].map(|c| WallEnd { key: k, edge: e, corner: c % 4, out, off }));
                    if let Some(s) = shell {
                        grow(&mut boxes[pi], &s);
                        walls.shells.push(s);
                        rep.walls += 1;
                    }
                }
            }
            // Corner walls: each wall end joins its nearest end on another edge.
            let mut pairs = BTreeSet::new();
            for (i, a) in ends.iter().enumerate() {
                let near = ends
                    .iter()
                    .enumerate()
                    .filter(|(_, b)| (b.key, b.edge) != (a.key, a.edge))
                    .map(|(j, b)| (j, dist(tiles[a.key].post(a.corner, 0.0)[0], tiles[b.key].post(b.corner, 0.0)[0])))
                    .min_by(|x, y| x.1.total_cmp(&y.1));
                if let Some((j, d)) = near {
                    if d < p.bridge && pairs.insert((i.min(j), i.max(j))) {
                        let d = inset.next(0.8);
                        let pts: Vec<V3> = a.points(&tiles, p, d).into_iter().chain(ends[j].points(&tiles, p, d)).collect();
                        match hull(&pts) {
                            Some(s) if piece.iter().flat_map(|&k| &holes[k]).any(|&q| s.contains(q, 0.05)) => rep.dropped += 1,
                            Some(s) => {
                                grow(&mut boxes[pi], &s);
                                walls.shells.push(s);
                                rep.corners += 1;
                            }
                            None => {}
                        }
                    }
                }
            }
            // Bottom plate: hull of the wall feet (or the tiles' footprint).
            let mut foot: Vec<[f64; 2]> =
                ends.iter().flat_map(|e| e.points(&tiles, p, 0.0)[2..].iter().map(|f| [f[0], f[1]]).collect::<Vec<_>>()).collect();
            if foot.is_empty() {
                foot = piece
                    .iter()
                    .flat_map(|&k| (0..4).map(move |c| (k, c)))
                    .map(|(k, c)| tiles[k].post(c, 0.0)[0])
                    .map(|q| [q[0], q[1]])
                    .collect();
            }
            let pts: Vec<V3> = foot.iter().flat_map(|q| [[q[0], q[1], 0.0], [q[0], q[1], p.bottom]]).collect();
            if let Some(s) = hull(&pts) {
                grow(&mut boxes[pi], &s);
                bottom.shells.push(s);
            }
            for &k in piece {
                grow(&mut boxes[pi], &plate.shells[k]);
            }
        }

        // Drop connectors that enter a switch hole; report what still does.
        let in_hole = |s: &Shell| holes.iter().flatten().any(|&q| s.contains(q, 0.05));
        for (s, web, pi) in connectors {
            if in_hole(&s) {
                rep.dropped += 1;
                continue;
            }
            if web {
                rep.webs += 1;
            } else {
                rep.junctions += 1;
            }
            grow(&mut boxes[pi], &s);
            solid.push((s.clone(), None));
            plate.shells.push(s);
        }
        for (k, pts) in holes.iter().enumerate() {
            let blocked = solid.iter().any(|(s, o)| *o != Some(k) && pts.iter().any(|&q| s.contains(q, 0.05)))
                || walls.shells.iter().any(|s| pts.iter().any(|&q| s.contains(q, 0.05)));
            if blocked {
                rep.blocked.push(keys[k]);
            }
            let caps = cap_samples(&tiles[k], &self.key[keys[k]], p);
            if solid.iter().map(|(s, _)| s).chain(&walls.shells).any(|s| caps.iter().any(|&q| s.contains(q, 0.05))) {
                rep.cap_hits.push(keys[k]);
            }
        }

        // Board space → STL space (mirror y), which also flips the winding back outward.
        for m in [&mut plate, &mut walls, &mut bottom] {
            for s in &mut m.shells {
                for q in &mut s.pts {
                    q[1] = -q[1];
                }
                for t in &mut s.tris {
                    t.swap(1, 2);
                }
            }
        }
        for (piece, b) in pieces.iter().zip(&boxes) {
            rep.pieces.push(PieceReport { keys: piece.iter().map(|&k| keys[k]).collect(), size: [0, 1, 2].map(|a| b[1][a] - b[0][a]) });
        }
        Ok(Case { plate, walls, bottom, report: rep })
    }
}

fn tile(k: &crate::geometry::Key, p: &CaseParams) -> Tile {
    let m = rotation(k.rot);
    let x = apply(&m, [1.0, 0.0, 0.0]);
    let y = apply(&m, [0.0, 1.0, 0.0]);
    let n = apply(&m, [0.0, 0.0, 1.0]);
    let s = switch_centre(k);
    let c = [s[0] * U_MM - n[0] * p.cap_gap, s[1] * U_MM - n[1] * p.cap_gap, s[2] * U_MM - n[2] * p.cap_gap];
    let half = |size: f32| (p.hole / 2.0 + p.rim).max(size as f64 * U_MM / 2.0 - 0.825);
    Tile { c, x, y, n, half: [half(k.size[0]), half(k.size[1])] }
}

/// Half-sizes `[u, v]` of the cutout in the clip layer and in the pocket below it (the pocket
/// is `clip_pocket` longer along the key's y and 0.5 mm wider), and the clip layer depth.
fn cutouts(t: &Tile, p: &CaseParams) -> ([f64; 2], Option<[f64; 2]>, f64) {
    let h = p.hole / 2.0;
    let clip = p.clip.min(p.plate);
    let pocket = (p.plate > clip + 1e-9).then(|| [(h + 0.25).min(t.half[0] - 0.4), (h + p.clip_pocket / 2.0).min(t.half[1] - 0.4)]);
    ([h, h], pocket, clip)
}

/// The plate frame of one key: one closed (genus-1) mesh, outer box minus the stepped cutout.
fn tile_frame(t: &Tile, p: &CaseParams) -> Shell {
    let (hole, pocket, clip) = cutouts(t, p);
    let [hx, hy] = t.half;
    let mut pts: Vec<V3> = Vec::new();
    let mut ring = |hu: f64, hv: f64, w: f64| -> [u32; 4] {
        let base = pts.len() as u32;
        pts.extend(CORNERS.map(|[a, b]| t.at(a * hu, b * hv, w)));
        [base, base + 1, base + 2, base + 3]
    };
    let ot = ring(hx, hy, 0.0);
    let ob = ring(hx, hy, -p.plate);
    let it = ring(hole[0], hole[1], 0.0);
    let (ib, lower) = match pocket {
        Some([pu, pv]) => (ring(hole[0], hole[1], -clip), Some((ring(pu, pv, -clip), ring(pu, pv, -p.plate)))),
        None => (ring(hole[0], hole[1], -p.plate), None),
    };
    let mut tris: Vec<[u32; 3]> = Vec::new();
    let mut quad = |a: u32, b: u32, c: u32, d: u32| {
        tris.push([a, b, c]);
        tris.push([a, c, d]);
    };
    for k in 0..4 {
        let j = (k + 1) % 4;
        quad(ot[k], ot[j], it[j], it[k]); // top ring
        quad(ob[k], ob[j], ot[j], ot[k]); // outer side
        quad(it[k], it[j], ib[j], ib[k]); // cutout wall
        match lower {
            Some((pt, pb)) => {
                quad(pt[k], ib[k], ib[j], pt[j]); // ledge under the clips
                quad(pt[k], pt[j], pb[j], pb[k]); // pocket wall
                quad(ob[k], pb[k], pb[j], ob[j]); // bottom ring
            }
            None => quad(ob[k], ib[k], ib[j], ob[j]),
        }
    }
    let mut s = Shell { pts, tris };
    if s.volume() < 0.0 {
        for t in &mut s.tris {
            t.swap(1, 2);
        }
    }
    s
}

/// The plate frame as convex boxes (for inside tests): the clip layer around the cutout and
/// the layer around the pocket below it.
fn tile_boxes(t: &Tile, p: &CaseParams) -> Vec<Shell> {
    let (hole, pocket, clip) = cutouts(t, p);
    let [hx, hy] = t.half;
    let mut layers = vec![(0.0, if pocket.is_some() { -clip } else { -p.plate }, hole)];
    if let Some(pk) = pocket {
        layers.push((-clip, -p.plate, pk));
    }
    let mut out = Vec::new();
    for (w0, w1, [hu, hv]) in layers {
        let boxes = [[-hx, -hu, -hy, hy], [hu, hx, -hy, hy], [-hu, hu, -hy, -hv], [-hu, hu, hv, hy]];
        for [u0, u1, v0, v1] in boxes {
            let pts: Vec<V3> = (0..8).map(|i| t.at([u0, u1][i & 1], [v0, v1][(i >> 1) & 1], [w0, w1][i >> 2])).collect();
            out.extend(hull(&pts));
        }
    }
    out
}

/// Points inside a switch's cutout and body that no other shell may enter.
fn hole_samples(t: &Tile, p: &CaseParams) -> Vec<V3> {
    let s = p.hole * 0.4;
    let mut out = Vec::new();
    for w in [-p.clip / 2.0, -p.plate / 2.0 - p.clip / 2.0, -p.switch_depth * 0.75] {
        for u in [-s, 0.0, s] {
            for v in [-s, 0.0, s] {
                out.push(t.at(u, v, w));
            }
        }
    }
    out
}

/// Points of a keycap's bottom through its travel.
fn cap_samples(t: &Tile, k: &crate::geometry::Key, p: &CaseParams) -> Vec<V3> {
    let (su, sv) = (k.size[0] as f64 * U_MM * 0.45, k.size[1] as f64 * U_MM * 0.45);
    let mut out = Vec::new();
    for w in [p.cap_gap - p.travel + 0.5, p.cap_gap] {
        for u in [-su, 0.0, su] {
            for v in [-sv, 0.0, sv] {
                out.push(t.at(u, v, w));
            }
        }
    }
    out
}

/// The facing edge pair of two tiles with a gap ≤ `bridge`, if any: opposite outward
/// directions, each edge in front of the other, closest edge pair first.
fn facing(a: &Tile, b: &Tile, bridge: f64) -> Option<(usize, usize)> {
    let reach = a.half[0].hypot(a.half[1]) + b.half[0].hypot(b.half[1]) + bridge;
    if dist(a.c, b.c) > reach {
        return None;
    }
    let mut best: Option<(usize, usize, f64)> = None;
    for ea in 0..4 {
        for eb in 0..4 {
            let (da, db) = (a.edge_dir(ea), b.edge_dir(eb));
            if dot(da, db) > -0.7
                || dot(da, sub(b.edge_mid(eb), a.edge_mid(ea))) <= 0.0
                || dot(db, sub(a.edge_mid(ea), b.edge_mid(eb))) <= 0.0
            {
                continue;
            }
            let [p0, p1] = a.edge(ea);
            let [q0, q1] = b.edge(eb);
            let d = segment_distance(p0, p1, q0, q1);
            // The edges must overlap side by side, not just meet end to end.
            let along = norm(sub(p1, p0));
            let (s0, s1) = (dot(sub(q0, p0), along), dot(sub(q1, p0), along));
            let overlap = s0.max(s1).min(dist(p0, p1)) - s0.min(s1).max(0.0);
            if d <= bridge && overlap > 1.0 && best.is_none_or(|x| d < x.2) {
                best = Some((ea, eb, d));
            }
        }
    }
    best.map(|(ea, eb, _)| (ea, eb))
}

/// Fallback for keys turned against each other (thumb clusters): the closest edge pair whose
/// outward directions point toward each other at all, if within `bridge`.
fn nearest_edges(a: &Tile, b: &Tile, bridge: f64) -> Option<(usize, usize)> {
    let mut best: Option<(usize, usize, f64)> = None;
    for ea in 0..4 {
        for eb in 0..4 {
            let (ma, mb) = (a.edge_mid(ea), b.edge_mid(eb));
            if dot(a.edge_dir(ea), sub(b.c, ma)) <= 0.0 || dot(b.edge_dir(eb), sub(a.c, mb)) <= 0.0 {
                continue;
            }
            let [p0, p1] = a.edge(ea);
            let [q0, q1] = b.edge(eb);
            // Rank by gap, then by how squarely the edges face (mid-point distance).
            let d = segment_distance(p0, p1, q0, q1) + 0.25 * dist(ma, mb);
            if segment_distance(p0, p1, q0, q1) <= bridge && best.is_none_or(|x| d < x.2) {
                best = Some((ea, eb, d));
            }
        }
    }
    best.map(|(ea, eb, _)| (ea, eb))
}

fn hull(pts: &[V3]) -> Option<Shell> {
    Shell::hull(pts)
}

fn dist(a: V3, b: V3) -> f64 {
    let d = sub(a, b);
    dot(d, d).sqrt()
}

/// Least distance between segments `p0p1` and `q0q1`.
fn segment_distance(p0: V3, p1: V3, q0: V3, q1: V3) -> f64 {
    let (d1, d2, r) = (sub(p1, p0), sub(q1, q0), sub(p0, q0));
    let (a, e, f) = (dot(d1, d1), dot(d2, d2), dot(d2, r));
    let (c, b) = (dot(d1, r), dot(d1, d2));
    let denom = a * e - b * b;
    let mut s = if denom > 1e-12 { ((b * f - c * e) / denom).clamp(0.0, 1.0) } else { 0.0 };
    let mut t = if e > 1e-12 { (b * s + f) / e } else { 0.0 };
    if t < 0.0 {
        t = 0.0;
        s = if a > 1e-12 { (-c / a).clamp(0.0, 1.0) } else { 0.0 };
    } else if t > 1.0 {
        t = 1.0;
        s = if a > 1e-12 { ((b - c) / a).clamp(0.0, 1.0) } else { 0.0 };
    }
    let pa = add(p0, d1.map(|v| v * s));
    let qb = add(q0, d2.map(|v| v * t));
    dist(pa, qb)
}

fn grow(b: &mut [V3; 2], s: &Shell) {
    let sb = s.bounds();
    for a in 0..3 {
        b[0][a] = b[0][a].min(sb[0][a]);
        b[1][a] = b[1][a].max(sb[1][a]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Finger, Key};

    fn key(rot: [f32; 3]) -> Key {
        let toml = "[[key]]\npos = [0.0, 0.0, 0.0]\nfinger = \"LI\"\nrow = 0\ncol = 0\neffort = 1.0\n";
        let mut k = Geometry::from_toml(&format!("name = \"t\"\n{toml}")).unwrap().key.remove(0);
        k.rot = rot;
        k.finger = Finger::LI;
        k
    }

    #[test]
    fn tile_frame_is_one_closed_ring_with_the_cutout() {
        for plate in [3.0, 1.5] {
            let p = CaseParams { plate, ..CaseParams::default() };
            let t = tile(&key([10.0, 20.0, 30.0]), &p);
            let f = tile_frame(&t, &p);
            assert!(crate::geometry::mesh::closed(&f));
            // Volume: outer box minus the cutout (and pocket below the clip layer).
            let (o, h) = (2.0 * (7.0 + 1.7), 14.0);
            let want = if plate > 1.5 { o * o * 3.0 - h * h * 1.5 - 14.5 * 15.0 * 1.5 } else { (o * o - h * h) * 1.5 };
            assert!((f.volume() - want).abs() < 1e-6, "{} vs {want}", f.volume());
            let boxes: f64 = tile_boxes(&t, &p).iter().map(Shell::volume).sum();
            assert!((boxes - want).abs() < 1e-6);
            // The cutout is open: its centre is in no box.
            assert!(!tile_boxes(&t, &p).iter().any(|b| b.contains(t.at(0.0, 0.0, -0.75), 0.0)));
        }
    }

    #[test]
    fn segment_distance_cases() {
        let d = segment_distance([0.0; 3], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0], [0.5, 2.0, 0.0]);
        assert!((d - 1.0).abs() < 1e-12);
        let d = segment_distance([0.0; 3], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0], [3.0, 0.0, 0.0]);
        assert!((d - 1.0).abs() < 1e-12);
    }
}
