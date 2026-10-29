// Procedural geometry for the physical Corne, in millimetres (three: x right, y up, z toward the viewer).
// Higher detail than ../kinetic/keyboard: smooth-cornered caps with a cylindrical dish and a soft rim,
// filleted slabs, union outlines from key rectangles.

import * as THREE from "three";
import { mergeGeometries, toCreasedNormals } from "three/examples/jsm/utils/BufferGeometryUtils.js";

/** Layer heights (mm) of the flat stack: bottom plate, PCB, switch plate; keycap base above the plate top. */
export const STACK = { bot0: 0, bot1: 2, pcb0: 5, pcb1: 6.6, plate0: 11.6, plate1: 13.1 } as const;
/** Keycap height and dish depth (mm). */
export const CAP = { h: 8, dish: 0.9 } as const;
/** Key travel (mm) a press sinks the cap and stem. */
export const TRAVEL = 3.5;

/** Rounded rectangle (centred) in the xz plane, `n` segments per corner; angle runs +x → +z. */
export function roundRect(w: number, d: number, r: number, n: number): [number, number][] {
  const hw = w / 2 - r, hd = d / 2 - r, out: [number, number][] = [];
  [[hw, hd], [-hw, hd], [-hw, -hd], [hw, -hd]].forEach(([cx, cz], q) => {
    for (let i = 0; i <= n; i++) {
      const a = (q + i / n) * Math.PI / 2;
      out.push([cx + r * Math.cos(a), cz + r * Math.sin(a)]);
    }
  });
  return out;
}

const caps = new Map<string, THREE.BufferGeometry>();

/**
 * Keycap of footprint w × d (mm), base at y = 0: a rounded base lofted up tapered walls to a rim that
 * rolls (one chamfer ring) into a cylindrical dish. Group 0 = walls + underside, group 1 = the dished
 * top, whose UVs span the top for the legend. The top is shifted toward the back (`off`, sculpt).
 */
export function capGeometry(w: number, d: number): THREE.BufferGeometry & { userData: { tw: number; td: number; off: number } } {
  const id = `${w.toFixed(2)}x${d.toFixed(2)}`;
  const hit = caps.get(id);
  if (hit) return hit as never;
  const N = 8;                                   // segments per corner
  const tw = w - 5.4, td = d - 3.8, off = -0.6;  // top size and its shift toward the back
  const rim = 0.55;                              // chamfer between wall and dish
  const H = CAP.h, D = CAP.dish;
  const base = roundRect(w, d, 1.1, N);
  const shoulder = roundRect(tw + 2 * rim, td + 2 * rim, 2.2 + rim, N);
  const top = roundRect(tw, td, 2.2, N);
  const dish = (x: number) => D * (1 - Math.min(1, (2 * x / tw) ** 2));
  const M = base.length;

  const pos: number[] = [], uv: number[] = [], idx: number[] = [];
  const v = (x: number, y: number, z: number, u = 0, t = 0) => { pos.push(x, y, z); uv.push(u, t); return pos.length / 3 - 1; };
  const band = (a: number, b: number) => {
    for (let i = 0; i < M; i++) {
      const j = (i + 1) % M;
      idx.push(a + i, b + i, b + j, a + i, b + j, a + j);
    }
  };
  // Walls: base → a mid ring (slight belly) → shoulder → rim.
  const ring = (pts: [number, number][], y: (x: number) => number, dz: number) => {
    const s = pos.length / 3;
    pts.forEach(([x, z]) => v(x, y(x), z + dz));
    return s;
  };
  const mid = base.map(([x, z], i) => [(x * 0.52 + shoulder[i][0] * 0.48) * 1.006, (z * 0.52 + shoulder[i][1] * 0.48) * 1.006] as [number, number]);
  const r0 = ring(base, () => 0, 0);
  const r1 = ring(mid, () => H * 0.5, off * 0.5);
  const r2 = ring(shoulder, (x) => H - D * 0.35 - dish(x) * 0.4, off);
  band(r0, r1); band(r1, r2);
  // Underside (open caps look wrong from low angles; close it).
  const c = v(0, 0.6, 0);
  const rb = ring(base, () => 0, 0);
  for (let i = 0; i < M; i++) idx.push(c, rb + (i + 1) % M, rb + i);
  const wallCount = idx.length;
  // Rim into the dish, then concentric rings to the centre.
  const r3 = pos.length / 3;
  top.forEach(([x, z]) => v(x, H - dish(x), z + off, 0.5 + x / tw, 0.5 - z / td));
  const r2t = pos.length / 3;
  shoulder.forEach(([x, z]) => v(x, H - D * 0.35 - dish(x) * 0.4, z + off, 0.5 + x / tw, 0.5 - z / td));
  band(r2t, r3);
  const K = 7;
  let prev = r3;
  for (let k = 1; k <= K; k++) {
    const f = 1 - k / K;
    const s = pos.length / 3;
    top.forEach(([x, z]) => v(x * f, H - dish(x * f), z * f + off, 0.5 + x * f / tw, 0.5 - z * f / td));
    band(prev, s);
    prev = s;
  }
  let g: THREE.BufferGeometry = new THREE.BufferGeometry();
  g.setAttribute("position", new THREE.Float32BufferAttribute(pos, 3));
  g.setAttribute("uv", new THREE.Float32BufferAttribute(uv, 2));
  g.setIndex(idx);
  g.addGroup(0, wallCount, 0);
  g.addGroup(wallCount, idx.length - wallCount, 1);
  g = toCreasedNormals(g, Math.PI / 5);
  g.userData = { tw, td, off };
  caps.set(id, g);
  return g as never;
}

/** Outline polygons (mm, plan view: x, z; overlapping pieces form their union) with filleted corners, extruded from y0 to y1. */
export function slab(outlines: [number, number][][], y0: number, y1: number, r = 1.2, seg = 6): THREE.BufferGeometry {
  const parts = outlines.map((o) => slabPiece(o, y0, y1, r, seg));
  if (parts.length === 1) return parts[0];
  // mergeGeometries drops material groups; keep the pieces' (caps = 0, sides = 1) in one run per piece.
  const merged = mergeGeometries(parts)!;
  let at = 0;
  for (const g of parts) {
    for (const gr of g.groups) merged.addGroup(at + gr.start, gr.count, gr.materialIndex);
    at += g.attributes.position.count;
  }
  return merged;
}

function slabPiece(points: [number, number][], y0: number, y1: number, r: number, seg: number): THREE.BufferGeometry {
  const pts = points.map(([x, z]) => new THREE.Vector2(x, -z));
  const n = pts.length, shape = new THREE.Shape();
  const cuts = pts.map((p, i) => {
    const a = pts[(i + n - 1) % n], b = pts[(i + 1) % n];
    const k = Math.min(r, p.distanceTo(a) / 2, p.distanceTo(b) / 2);
    return [p.clone().add(a.clone().sub(p).setLength(k)), p.clone().add(b.clone().sub(p).setLength(k))];
  });
  shape.moveTo(cuts[0][1].x, cuts[0][1].y);
  for (let i = 1; i <= n; i++) {
    const [c1, c2] = cuts[i % n], p = pts[i % n];
    shape.lineTo(c1.x, c1.y);
    shape.quadraticCurveTo(p.x, p.y, c2.x, c2.y);
  }
  const g = new THREE.ExtrudeGeometry(shape, { depth: y1 - y0, bevelEnabled: true, bevelThickness: 0.25, bevelSize: 0.25, bevelSegments: 2, curveSegments: seg });
  g.rotateX(-Math.PI / 2);
  g.translate(0, y0, 0);
  return toCreasedNormals(g, Math.PI / 6);
}

export interface Rect { x0: number; z0: number; x1: number; z1: number }

/**
 * Outline (one loop, clockwise in plan view) of the union of axis-aligned rectangles, by coordinate
 * compression: mark covered grid cells, walk the boundary edges, drop collinear points.
 * Assumes the union is one piece without holes (true for a Corne half).
 */
export function unionOutline(rects: Rect[]): [number, number][] {
  const eps = 1e-6;
  const uniq = (a: number[]) => [...new Set(a.map((x) => Math.round(x / eps) * eps))].sort((p, q) => p - q);
  const xs = uniq(rects.flatMap((r) => [r.x0, r.x1])), zs = uniq(rects.flatMap((r) => [r.z0, r.z1]));
  const nx = xs.length - 1, nz = zs.length - 1;
  const cell = (i: number, j: number) => {
    if (i < 0 || j < 0 || i >= nx || j >= nz) return false;
    const cx = (xs[i] + xs[i + 1]) / 2, cz = (zs[j] + zs[j + 1]) / 2;
    return rects.some((r) => cx > r.x0 && cx < r.x1 && cz > r.z0 && cz < r.z1);
  };
  // Directed boundary edges with the covered cell on the right (clockwise in x-right, z-down plan).
  const next = new Map<string, [number, number]>();
  const key = (i: number, j: number) => `${i},${j}`;
  for (let i = 0; i < nx; i++) for (let j = 0; j < nz; j++) {
    if (!cell(i, j)) continue;
    if (!cell(i, j - 1)) next.set(key(i, j), [i + 1, j]);         // top edge, left → right
    if (!cell(i + 1, j)) next.set(key(i + 1, j), [i + 1, j + 1]); // right edge, down
    if (!cell(i, j + 1)) next.set(key(i + 1, j + 1), [i, j + 1]); // bottom edge, right → left
    if (!cell(i - 1, j)) next.set(key(i, j + 1), [i, j]);         // left edge, up
  }
  const start = next.keys().next().value as string;
  const loop: [number, number][] = [];
  let k = start;
  do {
    const [i, j] = k.split(",").map(Number);
    loop.push([xs[i], zs[j]]);
    const n = next.get(k)!;
    k = key(n[0], n[1]);
  } while (k !== start && loop.length < 10000);
  return loop.filter((p, i) => {
    const a = loop[(i + loop.length - 1) % loop.length], b = loop[(i + 1) % loop.length];
    return !((Math.abs(a[0] - p[0]) < eps && Math.abs(b[0] - p[0]) < eps) || (Math.abs(a[1] - p[1]) < eps && Math.abs(b[1] - p[1]) < eps));
  });
}

type P2 = [number, number];

const area2 = (p: P2[]) => p.reduce((s, a, i) => { const b = p[(i + 1) % p.length]; return s + a[0] * b[1] - b[0] * a[1]; }, 0);
const edgesOf = (p: P2[]) => p.map((a, i) => [a, p[(i + 1) % p.length]] as [P2, P2]);

/** Convex hull of plan points (monotone chain). */
export function convexHull(pts: P2[]): P2[] {
  const s = [...pts].sort((a, b) => a[0] - b[0] || a[1] - b[1]);
  const cross = (o: P2, a: P2, b: P2) => (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
  const half = (list: P2[]) => {
    const h: P2[] = [];
    for (const p of list) {
      while (h.length >= 2 && cross(h[h.length - 2], h[h.length - 1], p) <= 1e-9) h.pop();
      h.push(p);
    }
    return h.slice(0, -1);
  };
  return [...half(s), ...half([...s].reverse())];
}

/**
 * Outline (one loop) of the union of simple polygons that overlap into one piece without holes: each edge is
 * split where another outline crosses or touches it, the pieces outside every other polygon are kept (an edge
 * shared from the same side once, from opposite sides never), and the pieces chain into the largest loop.
 */
export function unionPolygons(polys: P2[][]): P2[] {
  const eps = 1e-6;
  const ps = polys.map((p) => (area2(p) < 0 ? [...p].reverse() : p));
  const inside = (m: P2, p: P2[]) => {
    let c = false;
    for (const [a, b] of edgesOf(p)) if ((a[1] > m[1]) !== (b[1] > m[1]) && m[0] < a[0] + ((m[1] - a[1]) / (b[1] - a[1])) * (b[0] - a[0])) c = !c;
    return c;
  };
  const onEdge = (m: P2, [c, d]: [P2, P2]) => {
    const ex = d[0] - c[0], ez = d[1] - c[1];
    const t = Math.max(0, Math.min(1, ((m[0] - c[0]) * ex + (m[1] - c[1]) * ez) / (ex * ex + ez * ez)));
    return Math.hypot(c[0] + t * ex - m[0], c[1] + t * ez - m[1]) < eps;
  };
  const segs: [P2, P2][] = [];
  ps.forEach((p, i) => {
    for (const [a, b] of edgesOf(p)) {
      const dx = b[0] - a[0], dz = b[1] - a[1], len2 = dx * dx + dz * dz;
      const ts = [0, 1];
      ps.forEach((q, j) => {
        if (j === i) return;
        for (const [c, d] of edgesOf(q)) {
          const ex = d[0] - c[0], ez = d[1] - c[1], den = dx * ez - dz * ex;
          if (Math.abs(den) > 1e-9) {
            const t = ((c[0] - a[0]) * ez - (c[1] - a[1]) * ex) / den, u = ((c[0] - a[0]) * dz - (c[1] - a[1]) * dx) / den;
            if (t > 1e-9 && t < 1 - 1e-9 && u > -1e-9 && u < 1 + 1e-9) ts.push(t);
          }
          // Another outline's vertex on this edge (collinear overlaps, T-junctions).
          const t = ((c[0] - a[0]) * dx + (c[1] - a[1]) * dz) / len2;
          if (t > 1e-9 && t < 1 - 1e-9 && Math.hypot(a[0] + t * dx - c[0], a[1] + t * dz - c[1]) < eps) ts.push(t);
        }
      });
      ts.sort((x, y) => x - y);
      for (let k = 0; k + 1 < ts.length; k++) {
        if (ts[k + 1] - ts[k] < 1e-9) continue;
        const at = (t: number): P2 => [a[0] + t * dx, a[1] + t * dz];
        const m = at((ts[k] + ts[k + 1]) / 2);
        const keep = ps.every((q, j) => {
          if (j === i) return true;
          const on = edgesOf(q).find((e) => onEdge(m, e));
          if (!on) return !inside(m, q);
          const same = dx * (on[1][0] - on[0][0]) + dz * (on[1][1] - on[0][1]) > 0;
          return same && i < j;
        });
        if (keep) segs.push([at(ts[k]), at(ts[k + 1])]);
      }
    }
  });
  const used = segs.map(() => false), loops: P2[][] = [];
  for (let s = 0; s < segs.length; s++) {
    const loop: P2[] = [];
    for (let k = s; k >= 0 && !used[k];) {
      used[k] = true;
      loop.push(segs[k][0]);
      const end = segs[k][1];
      k = segs.findIndex((g, n) => !used[n] && Math.hypot(g[0][0] - end[0], g[0][1] - end[1]) < eps);
    }
    if (loop.length > 2) loops.push(loop);
  }
  const best = loops.reduce((x, y) => (Math.abs(area2(y)) > Math.abs(area2(x)) ? y : x));
  return best.filter((p, i) => {
    const a = best[(i + best.length - 1) % best.length], b = best[(i + 1) % best.length];
    return Math.abs((p[0] - a[0]) * (b[1] - p[1]) - (p[1] - a[1]) * (b[0] - p[0])) > 1e-6;
  });
}

/** Faceted normals for a low-count primitive (hex posts). */
export function faceted(g: THREE.BufferGeometry): THREE.BufferGeometry {
  const f = g.toNonIndexed();
  f.computeVertexNormals();
  return f;
}

/** Rounded box via ExtrudeGeometry of a rounded rectangle (w × d plan, height h, base at y = 0). */
export function roundBox(w: number, d: number, h: number, r: number, bevel = 0.15): THREE.BufferGeometry {
  const s = new THREE.Shape();
  const pts = roundRect(w - 2 * bevel, d - 2 * bevel, Math.max(0.01, r - bevel), 4);
  s.moveTo(pts[0][0], -pts[0][1]);
  for (const [x, z] of pts.slice(1)) s.lineTo(x, -z);
  const g = new THREE.ExtrudeGeometry(s, { depth: h - 2 * bevel, bevelEnabled: bevel > 0, bevelThickness: bevel, bevelSize: bevel, bevelSegments: 2 });
  g.rotateX(-Math.PI / 2);
  g.translate(0, bevel, 0);
  return toCreasedNormals(g, Math.PI / 5);
}
