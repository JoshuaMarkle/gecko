// Hand model for the Hand tab: measurements, presets, hand placement and finger IK, a typing
// planner with overlapping finger motion, and a synthetic population for the size and
// similarity charts. Lengths in mm. Pure (unit-tested in hand.test.ts). The scorer runs the
// same skeleton, placement and IK in Rust (`gecko_core::hand`, context/hand-model.md); "Use hand
// in optimization" sends this hand as the profile's `hand` (`withHand`). The population is
// synthetic until reference hand data lands. hand.parity.test.ts checks the skeleton, curl,
// finger and thumb IK and placement against the Rust port (fixtures/hand-parity.json).

import type { Profile } from "./api";

export type FingerName = "thumb" | "index" | "middle" | "ring" | "pinky";
export const FINGER_NAMES: FingerName[] = ["thumb", "index", "middle", "ring", "pinky"];
/** The four fingers that rest on the home row, index first. */
export const FOUR: FingerName[] = ["index", "middle", "ring", "pinky"];

/** Segment lengths base → tip: thumb metacarpal, proximal, distal; fingers proximal, middle, distal. */
export interface HandDims {
  thumb: [number, number, number];
  index: [number, number, number];
  middle: [number, number, number];
  ring: [number, number, number];
  pinky: [number, number, number];
  /** Across the index–pinky knuckles. */
  palmWidth: number;
  /** Wrist crease to the middle knuckle. */
  palmLength: number;
  /** Thumb tip to pinky tip, spread flat. */
  span: number;
  /** Comfortable fingertip travel from the home key. */
  reach: number;
}

export type Preset = "small" | "average" | "large" | "custom";
export interface HandSettings { use: boolean; preset: Preset; custom: HandDims | null; wrist?: Wrist; body?: Body }

/** Arms for G03 (mm): elbow-to-elbow distance while typing and forearm length. Unset: average arms scaled with the hand. */
export interface Body { elbowWidth: number; forearm: number }
/** Average arms (`gecko_core::hand::BODY`). */
export const BODY: Body = { elbowWidth: 490, forearm: 255 };
/** The arms G03 uses for a hand: `body`, else `BODY` scaled with hand length. */
export const bodyOf = (h: HandSettings): Body => {
  if (h.body) return h.body;
  const s = handLength(handDims(h)) / handLength(AVERAGE);
  return { elbowWidth: BODY.elbowWidth * s, forearm: BODY.forearm * s };
};

/**
 * View adjustments on top of the solved hand placement, mirrored for the left hand: wrist
 * height (mm, + up), distance (mm, + toward the user), spread (mm, + away from the board's
 * centre), angle (degrees, + fingers turned toward the centre), tilt (degrees, roll about the
 * forearm, + lifts the thumb side so the pinky side is lower), and follow (share of a reaching
 * finger's excursion the whole hand follows; 0 keeps the wrist planted).
 */
export interface Wrist { height: number; distance: number; spread: number; angle: number; tilt: number; follow: number }
export const WRIST0: Wrist = { height: 0, distance: 0, spread: 0, angle: 0, tilt: 0, follow: 0.3 };
export const wristOf = (h: HandSettings): Wrist => ({ ...WRIST0, ...h.wrist });

/** Average adult hand (pooled sexes), rounded from published phalanx and palm measurements. */
export const AVERAGE: HandDims = {
  thumb: [46, 32, 23],
  index: [40, 23, 18],
  middle: [45, 27, 19],
  ring: [42, 26, 19],
  pinky: [33, 18, 17],
  palmWidth: 84,
  palmLength: 105,
  span: 200,
  reach: 40,
};

/** Small / large presets: the average scaled to about the 8th / 92nd percentile of hand length. */
export const PRESET_SCALE: Record<Exclude<Preset, "custom">, number> = { small: 0.92, average: 1, large: 1.08 };

export function scaleDims(d: HandDims, s: number): HandDims {
  const seg = (x: [number, number, number]) => x.map((v) => v * s) as [number, number, number];
  return {
    thumb: seg(d.thumb), index: seg(d.index), middle: seg(d.middle), ring: seg(d.ring), pinky: seg(d.pinky),
    palmWidth: d.palmWidth * s, palmLength: d.palmLength * s, span: d.span * s, reach: d.reach * s,
  };
}

export function handDims(h: HandSettings): HandDims {
  return h.preset === "custom" && h.custom ? h.custom : scaleDims(AVERAGE, PRESET_SCALE[h.preset === "custom" ? "average" : h.preset]);
}

/** The profile `hand` block the scorer reads (`gecko_core::hand::HandSpec`, snake_case mm). */
export interface HandSpec {
  size: "small" | "average" | "large";
  dims?: { thumb: number[]; index: number[]; middle: number[]; ring: number[]; pinky: number[]; palm_width: number; palm_length: number; span: number; reach: number };
  body?: { elbow_width: number; forearm: number };
}

export function handSpec(h: HandSettings): HandSpec {
  const body = h.body ? { body: { elbow_width: h.body.elbowWidth, forearm: h.body.forearm } } : {};
  if (h.preset !== "custom" || !h.custom) return { size: h.preset === "custom" ? "average" : h.preset, ...body };
  const d = h.custom;
  return {
    size: "average",
    dims: { thumb: d.thumb, index: d.index, middle: d.middle, ring: d.ring, pinky: d.pinky, palm_width: d.palmWidth, palm_length: d.palmLength, span: d.span, reach: d.reach },
    ...body,
  };
}

/**
 * G01 / G03 / G05 / G06 weights and reference spreads (english / ansi-30) added when a profile has
 * none (`data/profiles/hand.toml`); G03 and G06 are board metrics, weighted in natural units (scale 1).
 */
export const HAND_WEIGHTS: Record<string, [number, number]> = { G01: [1, 0.1983], G03: [1, 1], G05: [0.5, 0.05973], G06: [1, 1] };

/**
 * The profile the scorer gets: with "Use hand in optimization" on, `hand` is this hand and G01 /
 * G03 / G05 / G06 get the hand profile's weights unless the profile already sets them (0 included);
 * off, no `hand` (they then use the average hand, at whatever weight the profile gives them).
 */
export function withHand(p: Profile, h: HandSettings): Profile {
  const { hand: _old, ...rest } = p;
  if (!h.use) return rest;
  const weights = { ...p.weights }, scale = { ...p.scale };
  for (const [id, [w, s]] of Object.entries(HAND_WEIGHTS)) {
    if (!(id in weights)) { weights[id] = w; scale[id] ??= s; }
  }
  return { ...rest, hand: handSpec(h), weights, scale };
}

/** Wrist crease to middle fingertip. */
export const handLength = (d: HandDims) => d.palmLength + d.middle[0] + d.middle[1] + d.middle[2];
export const fingerLength = (d: HandDims, f: FingerName) => d[f][0] + d[f][1] + d[f][2];

/** Every measurement as a flat vector (fixed order), for distances. */
export function dimsVector(d: HandDims): number[] {
  return [...d.thumb, ...d.index, ...d.middle, ...d.ring, ...d.pinky, d.palmWidth, d.palmLength, d.span, d.reach];
}

// ---------- size distribution ----------

/** Hand length in the population: normal, sd 5.5% of the mean (adult pooled). */
export const LENGTH_MEAN = handLength(AVERAGE);
export const LENGTH_SD = 0.055 * LENGTH_MEAN;

export function normalPdf(x: number, mu: number, sd: number): number {
  const z = (x - mu) / sd;
  return Math.exp(-0.5 * z * z) / (sd * Math.sqrt(2 * Math.PI));
}

/** Standard normal CDF (Abramowitz–Stegun 7.1.26). */
export function normalCdf(z: number): number {
  const t = 1 / (1 + 0.3275911 * Math.abs(z) / Math.SQRT2);
  const y = 1 - ((((1.061405429 * t - 1.453152027) * t + 1.421413741) * t - 0.284496736) * t + 0.254829592) * t * Math.exp(-(z * z) / 2);
  return z >= 0 ? 0.5 * (1 + y) : 0.5 * (1 - y);
}

/** Size band of a hand length: below / within / above one sd of the mean. */
export function sizeBand(len: number): "Small" | "Average" | "Large" {
  const z = (len - LENGTH_MEAN) / LENGTH_SD;
  return z < -1 ? "Small" : z > 1 ? "Large" : "Average";
}

// ---------- similarity ----------

function rng(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function gauss(r: () => number): number {
  return Math.sqrt(-2 * Math.log(Math.max(r(), 1e-12))) * Math.cos(2 * Math.PI * r());
}

/** Synthetic population: one overall scale (sd 5.5%) times per-measurement shape noise (sd 4%). */
export function population(n = 600, seed = 7): number[][] {
  const r = rng(seed);
  const base = dimsVector(AVERAGE);
  return Array.from({ length: n }, () => {
    const s = 1 + 0.055 * gauss(r);
    return base.map((v) => v * s * (1 + 0.04 * gauss(r)));
  });
}

/** Similarity in (0, 1]: exp of minus the RMS log-ratio over all measurements, in units of 10%. */
export function similarity(a: number[], b: number[]): number {
  let ss = 0;
  for (let i = 0; i < a.length; i++) {
    const d = Math.log(a[i] / b[i]) / 0.1;
    ss += d * d;
  }
  return Math.exp(-Math.sqrt(ss / a.length));
}

/** Typicality: mean similarity to the population. */
export function typicality(v: number[], pop: number[][]): number {
  let t = 0;
  for (const p of pop) t += similarity(v, p);
  return t / pop.length;
}

// ---------- hand skeleton ----------

export type V3 = [number, number, number];
export type Side = "L" | "R";
const add = (a: V3, b: V3): V3 => [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
export const sub3 = (a: V3, b: V3): V3 => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
const scale = (a: V3, s: number): V3 => [a[0] * s, a[1] * s, a[2] * s];
const lerp3 = (a: V3, b: V3, t: number): V3 => [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
export const dist3 = (a: V3, b: V3) => Math.hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2]);
const dot3 = (a: V3, b: V3) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const cross3 = (a: V3, b: V3): V3 => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
const norm3 = (a: V3): V3 => scale(a, 1 / (Math.hypot(a[0], a[1], a[2]) || 1));
const perp3 = (v: V3, d: V3): V3 => norm3(sub3(v, scale(d, dot3(v, d))));

/**
 * Right hand, palm down, in mm: x toward the pinky, y up, z toward the wrist (fingers point to
 * −z), origin at the centre of the wrist crease. The left hand is its exact mirror (x → −x).
 */
export interface HandLayout {
  /** Index, middle, ring, pinky knuckles (MCP joints). */
  knuckles: V3[];
  /** Thumb base (CMC joint). */
  thumb: V3;
  /** Where the thumb tip rests in the natural typing pose. */
  thumbRest: V3;
  /** Palm outline, closed: radial wrist, thumb base, index … pinky knuckles, ulnar edge, ulnar wrist. */
  outline: V3[];
}

/** Knuckle arch: drop below the middle knuckle (y) and setback toward the wrist (z), index → pinky, as fractions of palm length. */
const ARCH_Y = [-0.03, 0, -0.025, -0.07];
const ARCH_Z = [0.02, 0, 0.04, 0.13];

export function handLayout(d: HandDims): HandLayout {
  const kw = 0.76 * d.palmWidth; // index–pinky knuckle centres
  const pl = d.palmLength;
  const knuckles: V3[] = FOUR.map((_, i) => [kw * (i / 3 - 0.5), ARCH_Y[i] * pl, -pl * (1 - ARCH_Z[i])]);
  const thumb: V3 = [-0.42 * d.palmWidth, -0.12 * pl, -0.24 * pl];
  const thumbRest: V3 = [knuckles[0][0] - 0.1 * d.palmWidth, -0.35 * pl, -(0.72 * pl + 0.4 * fingerLength(d, "thumb"))];
  const edge = (d.palmWidth - kw) / 2;
  const outline: V3[] = [
    [-0.36 * d.palmWidth, -0.06 * pl, 0],
    thumb,
    [knuckles[0][0] - edge * 0.6, knuckles[0][1], knuckles[0][2] + 4],
    ...knuckles,
    [knuckles[3][0] + edge, knuckles[3][1] - 2, knuckles[3][2] + 8],
    [0.5 * d.palmWidth, -0.06 * pl, -0.45 * pl],
    [0.4 * d.palmWidth, -0.06 * pl, 0],
  ];
  return { knuckles, thumb, thumbRest, outline };
}

/** Tip of a planar chain with base angle 0 and joint flexions `b`, `ratio·b` (2D: u forward, v up). */
function tip2(l: [number, number, number], b: number, ratio: number): [number, number] {
  const a = [0, b, b + ratio * b];
  let u = 0, v = 0;
  for (let i = 0; i < 3; i++) { u += l[i] * Math.cos(a[i]); v -= l[i] * Math.sin(a[i]); }
  return [u, v];
}

/**
 * Natural typing curl: PIP flexed 54°, DIP 0.7 of that (tendon coupling), proximal phalanx
 * pitched 11° down. Average middle finger: tip 50 mm ahead of and 52 mm below its knuckle.
 */
const CURL = 0.95;
const COUPLE = 0.7;
const PITCH = -0.2;

/** Fingertip offset from the knuckle in the natural curl: [forward, drop], same units as `l`. */
export function naturalTip(l: [number, number, number]): [number, number] {
  const [pu, pv] = tip2(l, CURL, COUPLE);
  return [pu * Math.cos(PITCH) - pv * Math.sin(PITCH), -(pu * Math.sin(PITCH) + pv * Math.cos(PITCH))];
}

/**
 * Lean of each finger's flexion plane (index → pinky, radians: 2°, 4°, 8°, 15°): the plane's up
 * axis is turned about the hand's long axis toward the pinky, so a curling finger closes toward
 * the thumb side (flexed fingers converge on the scaphoid, the pinky most). Heuristic.
 */
export const LEAN = [0.035, 0.07, 0.14, 0.26];

/** Up axis of finger `i`'s (index 0 … pinky 3) flexion plane, hand-local (right-hand frame). */
export const fingerUp = (i: number): V3 => [Math.sin(LEAN[i]), Math.cos(LEAN[i]), 0];

/** Natural fingertip offset from knuckle `i`, hand-local: `naturalTip` in the leaned plane. */
export function naturalTip3(l: [number, number, number], i: number): V3 {
  const [fwd, drop] = naturalTip(l), up = fingerUp(i);
  return [-drop * up[0], -drop * up[1], -fwd];
}

/** Joint limits (radians, flexion positive): MCP from 20° hyperextension, PIP, DIP. */
const MCP: [number, number] = [-0.35, 1.5];
const PIP: [number, number] = [-0.05, 1.9];
const DIP: [number, number] = [0, 1.4];

/**
 * Joint positions (base, 2 joints, tip) of a 3-segment finger reaching from `base` to `target`
 * in its flexion plane: the plane through both that holds the unit `up` axis (fixed in the hand,
 * leaned per finger: `fingerUp`; default straight up). For each knuckle (MCP) flexion the last two segments
 * close the gap exactly when they can (the DIP angle from the remaining distance, the PIP angle
 * from its direction), with every joint clamped to its limits. The pose kept misses the target
 * least, then sits closest to the tendon coupling DIP ≈ `ratio`·PIP with the knuckle near its
 * resting pitch `rest`, so a near key curls the finger at the knuckle instead of buckling it.
 * Out of reach: the finger points straight at the target.
 */
export function fingerChain(base: V3, target: V3, l: [number, number, number], ratio = COUPLE, rest = -PITCH, up: V3 = [0, 1, 0]): V3[] {
  const { tu, tv, dir } = fingerPlane(base, target, up);
  const clamp = (x: number, [lo, hi]: [number, number]) => Math.max(lo, Math.min(hi, x));
  const total = l[0] + l[1] + l[2];
  // Pose for MCP flexion `a`: [cost, a, PIP, DIP] (angles below the horizontal, flexion positive).
  const solve = (a: number): [number, number, number, number] => {
    const du = tu - l[0] * Math.cos(a), dv = tv + l[0] * Math.sin(a);
    const D = Math.hypot(du, dv);
    const cg = (D * D - l[1] * l[1] - l[2] * l[2]) / (2 * l[1] * l[2]);
    const g = clamp(Math.acos(Math.max(-1, Math.min(1, cg))), DIP);
    const b = clamp(Math.atan2(-dv, du) - Math.atan2(l[2] * Math.sin(g), l[1] + l[2] * Math.cos(g)) - a, PIP);
    const eu = l[0] * Math.cos(a) + l[1] * Math.cos(a + b) + l[2] * Math.cos(a + b + g) - tu;
    const ev = -l[0] * Math.sin(a) - l[1] * Math.sin(a + b) - l[2] * Math.sin(a + b + g) - tv;
    const miss = (eu * eu + ev * ev) / (total * total);
    return [1e4 * miss + (g - ratio * b) ** 2 + 0.15 * (a - rest) ** 2, a, b, g];
  };
  let angles: number[];
  if (Math.hypot(tu, tv) >= total) {
    const a = Math.atan2(-tv, tu);
    angles = [a, a, a];
  } else {
    const n = 64, step = (MCP[1] - MCP[0]) / n;
    let best = solve(MCP[0]);
    for (let i = 1; i <= n; i++) { const s = solve(MCP[0] + i * step); if (s[0] < best[0]) best = s; }
    // Refine around the best grid point (golden-section search).
    let lo = Math.max(MCP[0], best[1] - step), hi = Math.min(MCP[1], best[1] + step);
    for (let i = 0; i < 24; i++) {
      const m1 = hi - 0.618 * (hi - lo), m2 = lo + 0.618 * (hi - lo);
      if (solve(m1)[0] < solve(m2)[0]) hi = m2; else lo = m1;
    }
    const r = solve((lo + hi) / 2);
    if (r[0] < best[0]) best = r;
    angles = [best[1], best[1] + best[2], best[1] + best[2] + best[3]];
  }
  const pts: V3[] = [base];
  let cur = base;
  for (let i = 0; i < 3; i++) {
    const a = angles[i];
    cur = add(cur, add(scale(dir, l[i] * Math.cos(a)), scale(up, -l[i] * Math.sin(a))));
    pts.push(cur);
  }
  return pts;
}

/**
 * A target in a finger's flexion plane (`gecko_core::hand::plane`): its distance ahead (`tu`) and
 * above (`tv`, along `up`) the knuckle, and the unit forward direction (straight ahead, −z, when
 * the target is right above or below).
 */
export function fingerPlane(knuckle: V3, target: V3, up: V3): { tu: number; tv: number; dir: V3 } {
  const v = sub3(target, knuckle), tv = dot3(v, up);
  const h = sub3(v, scale(up, tv)), tu = Math.hypot(h[0], h[1], h[2]);
  return { tu, tv, dir: tu > 1e-9 ? scale(h, 1 / tu) : perp3([0, 0, -1], up) };
}

// ---------- placement ----------

/** Where a hand sits: wrist crease centre (world), yaw about the vertical axis and roll about the forearm (radians, + thumb side up). */
export interface Placement { wrist: V3; yaw: number; roll?: number }

/** Natural inward yaw of the right hand (fingers turned toward the body's midline, ~10°); the left hand mirrors it. */
export const YAW0 = 0.17;

/** Hand-local mm (right-hand frame, see `HandLayout`) → world, for `side`, at `u` world units per mm. */
export function toWorld(side: Side, pl: Placement, u: number, p: V3): V3 {
  const r = pl.roll ?? 0, cr = Math.cos(r), sr = Math.sin(r);
  const lx = p[0] * cr + p[1] * sr, ly = p[1] * cr - p[0] * sr;
  const x = (side === "L" ? -lx : lx) * u, z = p[2] * u;
  const c = Math.cos(pl.yaw), s = Math.sin(pl.yaw);
  return [pl.wrist[0] + x * c + z * s, pl.wrist[1] + ly * u, pl.wrist[2] - x * s + z * c];
}

/** Inverse of `toWorld`: world → hand-local mm (right-hand frame). */
export function toLocal(side: Side, pl: Placement, u: number, w: V3): V3 {
  const dx = (w[0] - pl.wrist[0]) / u, dy = (w[1] - pl.wrist[1]) / u, dz = (w[2] - pl.wrist[2]) / u;
  const c = Math.cos(pl.yaw), s = Math.sin(pl.yaw);
  const x = dx * c - dz * s, z = dx * s + dz * c;
  const lx = side === "L" ? -x : x;
  const r = pl.roll ?? 0, cr = Math.cos(r), sr = Math.sin(r);
  return [lx * cr - dy * sr, lx * sr + dy * cr, z];
}

/** Apply the view's wrist adjustments to a solved placement (mirrored per side). */
export function adjustPlacement(side: Side, pl: Placement, w: Wrist, u: number): Placement {
  const m = side === "L" ? -1 : 1, deg = Math.PI / 180;
  return {
    wrist: [pl.wrist[0] + m * w.spread * u, pl.wrist[1] + w.height * u, pl.wrist[2] + w.distance * u],
    yaw: pl.yaw + m * w.angle * deg,
    roll: (pl.roll ?? 0) + w.tilt * deg,
  };
}

/**
 * Place a hand over its four home targets (world, three.js axes: y up, z toward the user): the
 * wrist position, height, yaw and roll minimise each finger's deviation from its natural curl in
 * the hand's frame (forward and height error, plus a lighter sideways error; the leaned planes put
 * each natural fingertip a little toward the thumb), with weak pulls toward the natural
 * yaw and no roll; the shorter pinky so settles the hand slightly pinky-side down. Solved in the right-hand frame
 * (left-hand targets are mirrored first), so mirrored targets give exactly mirrored hands.
 */
export function placeHand(side: Side, home: Record<FingerName, V3>, d: HandDims, u: number): Placement {
  const m = side === "L" ? -1 : 1;
  const T = FOUR.map((f) => [(m * home[f][0]) / u, home[f][1] / u, home[f][2] / u]);
  const lay = handLayout(d);
  const nat = FOUR.map((f) => naturalTip(d[f]));
  const nat3 = FOUR.map((f, i) => naturalTip3(d[f], i));
  // Variables: wrist x, z, yaw, wrist height, roll (right-hand frame, mm and radians).
  const cost = (q: number[]) => {
    const [wx, wz, yaw, wy, roll] = q;
    const pl: Placement = { wrist: [wx, wy, wz], yaw, roll };
    let e = 300 * (yaw - YAW0) ** 2 + 300 * roll * roll;
    for (let i = 0; i < 4; i++) {
      const v = sub3(sub3(toLocal("R", pl, 1, T[i] as V3), lay.knuckles[i]), nat3[i]);
      e += v[2] * v[2] + 0.3 * v[0] * v[0] + v[1] * v[1];
    }
    return e;
  };
  // Start with the middle knuckle its natural reach behind and above the middle target.
  const c0 = Math.cos(YAW0), s0 = Math.sin(YAW0), k1 = lay.knuckles[1];
  const kx = T[1][0] + s0 * nat[1][0], kz = T[1][2] + c0 * nat[1][0];
  let p = [kx - (k1[0] * c0 + k1[2] * s0), kz - (-k1[0] * s0 + k1[2] * c0), YAW0, T[1][1] + nat[1][1] - k1[1], 0];
  let best = cost(p);
  const step = [8, 8, 0.06, 8, 0.06];
  for (let it = 0; it < 200; it++) {
    let moved = false;
    for (let k = 0; k < p.length; k++) for (const sg of [1, -1]) {
      const q = [...p];
      q[k] += sg * step[k];
      const c = cost(q);
      if (c < best - 1e-9) { best = c; p = q; moved = true; }
    }
    if (!moved) { for (let k = 0; k < step.length; k++) step[k] /= 2; if (step[0] < 0.01) break; }
  }
  return { wrist: [m * p[0] * u, p[3] * u, p[1] * u], yaw: m * p[2], roll: p[4] };
}

export interface HandPose {
  /** Joint chains for thumb, index, middle, ring, pinky (base → tip). */
  fingers: V3[][];
  /** Flexion axis of each finger's plane, index → pinky (unit, toward the pinky side). */
  axes: V3[];
  /** Closed palm outline (see `HandLayout.outline`). */
  palm: V3[];
  /** Centre of the wrist crease. */
  wrist: V3;
}

/** Pose a placed hand (shifted by `offset`, world units) so each fingertip reaches its target. */
export function poseHand(side: Side, pl: Placement, targets: Record<FingerName, V3>, d: HandDims, u: number, offset: V3 = [0, 0, 0]): HandPose {
  const lay = handLayout(d);
  const W = (p: V3) => add(toWorld(side, pl, u, p), offset);
  const L = (x: [number, number, number]) => x.map((v) => v * u) as [number, number, number];
  const knuckles = lay.knuckles.map(W);
  const thumb = new Thumb(d);
  const local = sub3(targets.thumb, offset);
  const fingers: V3[][] = [thumb.joints(thumb.solve(toLocal(side, pl, u, local)).q).map(W)];
  const axes: V3[] = [];
  const m = side === "L" ? -1 : 1;
  FOUR.forEach((f, i) => {
    const up = norm3(sub3(W(add(lay.knuckles[i], fingerUp(i))), knuckles[i]));
    fingers.push(fingerChain(knuckles[i], targets[f], L(d[f]), COUPLE, -PITCH, up));
    // Mirroring flips the cross product: × side keeps the axis toward the pinky.
    axes.push(scale(norm3(cross3(fingerPlane(knuckles[i], targets[f], up).dir, up)), m));
  });
  return { fingers, axes, palm: lay.outline.map(W), wrist: W([0, 0, 0]) };
}

// ---------- thumb ----------

/** `v` rotated by `a` radians about unit `axis` (Rodrigues). */
function rotate3(axis: V3, a: number, v: V3): V3 {
  const s = Math.sin(a), c = Math.cos(a), k = cross3(axis, v), d = dot3(axis, v) * (1 - c);
  return [v[0] * c + k[0] * s + axis[0] * d, v[1] * c + k[1] * s + axis[1] * d, v[2] * c + k[2] * s + axis[2] * d];
}

/** Rest flexion of the thumb's MCP and IP (≈ 17°). */
const THUMB_CURL = 0.3;
/** CMC flexion and abduction about rest, MCP and IP absolute (radians). */
const THUMB_LIMIT = [[-0.6, 0.6], [-0.5, 0.7], [-0.2, 1.05], [-0.3, 1.4]];
/** Deviation from rest that costs 1 per joint (radians). */
export const THUMB_SCALE = [0.45, 0.45, 0.6, 0.8];

/**
 * The 4-DoF thumb in the right-hand frame (`gecko_core::hand::Thumb`): CMC flexion (tip sweeps
 * toward the fingers) and palmar abduction (tip moves away from the palm) about fixed axes at the
 * CMC, then MCP and IP flexion. Angles `[cmc flexion, cmc abduction, mcp, ip]`, CMC relative to rest.
 */
export class Thumb {
  base: V3; d0: V3; pad0: V3; flex: V3; abd: V3; l: [number, number, number];
  neutral: number[] = [0, 0, THUMB_CURL, THUMB_CURL];

  constructor(d: HandDims) {
    const lay = handLayout(d);
    this.base = lay.thumb;
    this.d0 = norm3(sub3(lay.thumbRest, lay.thumb));
    const across: V3 = [1, 0, 0], down: V3 = [0, -1, 0];
    this.pad0 = perp3([0.8, -0.6, 0], this.d0);
    this.flex = norm3(cross3(this.d0, across));
    this.abd = norm3(cross3(this.d0, down));
    this.l = d.thumb;
    for (let i = 0; i < 4; i++) this.neutral = this.solveWith(lay.thumbRest, this.neutral, [true, true, true, true]).q;
  }

  /** CMC, MCP, IP, tip (hand-local mm). */
  joints(q: number[]): V3[] {
    const r = (v: V3) => rotate3(this.abd, q[1], rotate3(this.flex, q[0], v));
    const m = r(this.d0), pad = r(this.pad0), axis = norm3(cross3(m, pad));
    const p = rotate3(axis, q[2], m), dist = rotate3(axis, q[2] + q[3], m);
    const j1 = add(this.base, scale(m, this.l[0])), j2 = add(j1, scale(p, this.l[1]));
    return [this.base, j1, j2, add(j2, scale(dist, this.l[2]))];
  }

  /** IK toward `target` (hand-local mm): angles and fingertip shortfall (mm). */
  solve(target: V3): { q: number[]; miss: number } {
    return this.solveWith(target, this.neutral, [true, true, true, true]);
  }

  private limit(j: number): [number, number] {
    const off = j < 2 ? this.neutral[j] : 0;
    return [THUMB_LIMIT[j][0] + off, THUMB_LIMIT[j][1] + off];
  }

  /** Levenberg–Marquardt on the tip error (weighted to dominate) plus each joint's deviation from rest. */
  private solveWith(target: V3, start: number[], free: boolean[]): { q: number[]; miss: number } {
    const w = 100 / (this.l[0] + this.l[1] + this.l[2]);
    const resid = (q: number[]) => {
      const e = sub3(this.joints(q)[3], target);
      return [w * e[0], w * e[1], w * e[2], ...q.map((v, j) => (free[j] ? (v - this.neutral[j]) / THUMB_SCALE[j] : 0))];
    };
    const energy = (r: number[]) => r.reduce((a, x) => a + x * x, 0);
    let q = [...start], r = resid(q), lambda = 1e-2;
    for (let it = 0; it < 60; it++) {
      const jac = r.map(() => [0, 0, 0, 0]);
      for (let j = 0; j < 4; j++) {
        if (!free[j]) continue;
        const qh = [...q];
        qh[j] += 1e-6;
        const rh = resid(qh);
        for (let i = 0; i < r.length; i++) jac[i][j] = (rh[i] - r[i]) / 1e-6;
      }
      const a = [0, 1, 2, 3].map((j) => [0, 1, 2, 3].map((k) => jac.reduce((acc, row) => acc + row[j] * row[k], 0)));
      const b = [0, 1, 2, 3].map((j) => -jac.reduce((acc, row, i) => acc + row[j] * r[i], 0));
      for (let j = 0; j < 4; j++) a[j][j] += lambda * (1 + a[j][j]);
      const delta = solve4(a, b);
      const next = q.map((v, j) => (free[j] ? Math.min(Math.max(v + delta[j], this.limit(j)[0]), this.limit(j)[1]) : v));
      const rn = resid(next);
      if (energy(rn) < energy(r)) {
        const gain = energy(r) - energy(rn);
        q = next; r = rn; lambda = Math.max(lambda * 0.3, 1e-7);
        if (gain < 1e-12) break;
      } else {
        lambda *= 10;
        if (lambda > 1e8) break;
      }
    }
    const e = sub3(this.joints(q)[3], target);
    return { q, miss: Math.hypot(e[0], e[1], e[2]) };
  }
}

/** Solve the 4 × 4 system `a x = b` (Gaussian elimination, partial pivoting); zeros if singular. */
function solve4(a: number[][], b: number[]): number[] {
  a = a.map((r) => [...r]); b = [...b];
  for (let c = 0; c < 4; c++) {
    let p = c;
    for (let i = c + 1; i < 4; i++) if (Math.abs(a[i][c]) > Math.abs(a[p][c])) p = i;
    if (Math.abs(a[p][c]) < 1e-14) continue;
    [a[c], a[p]] = [a[p], a[c]];
    [b[c], b[p]] = [b[p], b[c]];
    for (let r = c + 1; r < 4; r++) {
      const f = a[r][c] / a[c][c];
      for (let k = c; k < 4; k++) a[r][k] -= f * a[c][k];
      b[r] -= f * b[c];
    }
  }
  const x = [0, 0, 0, 0];
  for (let c = 3; c >= 0; c--) {
    if (Math.abs(a[c][c]) < 1e-14) continue;
    let v = b[c];
    for (let k = c + 1; k < 4; k++) v -= a[c][k] * x[k];
    x[c] = v / a[c][c];
  }
  return x;
}

// ---------- typing ----------

/**
 * One keystroke: which finger presses where (world, the keycap top), the character typed, the
 * board key (none: a virtual key, e.g. a space bar the board lacks) and the keycap's normal
 * (default straight up; tilted on 3D boards), along which the fingertip presses.
 */
export interface Stroke { side: Side; finger: FingerName; pos: V3; ch: string; key?: number; n?: V3 }

/** Why a press could not come sooner: the rhythm (target speed), the same finger still busy, or the previous press not yet released (serial motion). */
export type Limit = "rhythm" | "finger" | "serial";

export interface Press extends Stroke {
  /** Where the fingertip starts its move from. */
  from: V3;
  /** Move start, over the key (ready), key down, key released (seconds). */
  depart: number;
  ready: number;
  arrive: number;
  up: number;
  /** Back home after the release, or Infinity when the finger goes straight on to its next key. */
  back: number;
  limit: Limit;
  /** How many times faster than its natural pace the finger had to move to make this key (1: natural). */
  rush: number;
}

export interface FingerStats { presses: number; travel: number; busy: number }
export interface TypingStats {
  targetWpm: number;
  /** Speed of the plan, and the speed this text reaches with every finger at its natural pace. */
  wpm: number;
  naturalWpm: number;
  /** Keys whose finger had to move faster than natural (rush > 1.05), and the largest rush. */
  rushed: number;
  maxRush: number;
  /** Share of the typing time with two or more fingers moving, waiting over a key, or pressing at once. */
  overlap: number;
  /** Presses whose finger set off before the previous key was released. */
  early: number;
  /** Presses held up (strict: rushed) by the same finger's previous key, and by the previous release. */
  fingerWaits: number;
  serialWaits: number;
  /** Consecutive presses that switch hands. */
  alternation: number;
  /** Fingertip travel per press, mm. */
  travel: number;
  /** Presses followed by a return to the home key. */
  homeReturns: number;
  perFinger: Record<string, FingerStats>;
}

export interface Plan { presses: Press[]; byFinger: Record<string, Press[]>; end: number; stats: TypingStats }

export const fingerKey = (side: Side, f: FingerName) => `${side}${f}`;

/** Fingertip movement time for a distance in mm: Fitts-style log law (0 for no move). */
export function moveTime(mm: number): number {
  return mm < 0.5 ? 0 : 0.05 + 0.075 * Math.log2(1 + mm / 10);
}
/** Extra time to lift off and re-press the same key. */
const REPRESS = 0.06;
/** Shortest pause at home that makes a return trip worth it. */
const DWELL = 0.05;
/** Wait over the next key at 100% parallelism (s; grows with its square): fingers set off early and hover, ready. */
const PREP = 0.25;

/** Shortest move a finger can be rushed into, s. */
const MIN_MOVE = 0.02;

/**
 * Schedule keystrokes at `wpm` (5 chars per word). Fingers move at a natural pace
 * (`moveTime`) and set off as early as `parallel` (0–3) allows: up to `parallel` of their
 * travel before the previous release (its square above 1), and up to `parallel`²·PREP before they are needed, then
 * they wait over the key. 0: one finger at a time (each starts after the previous release);
 * 1: the next finger travels during the previous press; above 1 keys roll over (each held into
 * the next press) and fingers get ready ever earlier; 3: every finger heads for its next key as
 * soon as it is free. Between presses a finger returns home when there is time for the round
 * trip, otherwise it goes straight on to its next key.
 *
 * By default each press waits until its finger can make it at its natural pace, so fast text
 * slows down (`stats.wpm` is the speed reached). `strict`: keys go down exactly on the rhythm;
 * a finger that cannot make it moves faster (`Press.rush`) and a finger needed again soon lets
 * go of its key early; the stats still report the natural speed.
 */
export function planTyping(strokes: Stroke[], home: (side: Side, f: FingerName) => V3, wpm: number, parallel: number, u: number, strict = false): Plan {
  const dt = 60 / (Math.max(1, wpm) * 5);
  const presses = schedule(strokes, home, dt, parallel, u, strict);
  const natural = strict ? schedule(strokes, home, dt, parallel, u, false) : presses;
  const byFinger: Record<string, Press[]> = {};
  for (const pr of presses) (byFinger[fingerKey(pr.side, pr.finger)] ??= []).push(pr);
  const end = presses.reduce((t, pr) => Math.max(t, pr.up, Number.isFinite(pr.back) ? pr.back : 0), 0);
  return { presses, byFinger, end, stats: typingStats(presses, byFinger, home, wpm, dt, u, natural) };
}

function schedule(strokes: Stroke[], home: (side: Side, f: FingerName) => V3, dt: number, parallel: number, u: number, strict: boolean): Press[] {
  const p = Math.max(0, Math.min(3, parallel));
  const roll = Math.max(0, Math.min(1, p - 1));
  const lead = p * p * PREP;
  // Key down time; rollover holds longer but a finger lets go early when its own next key needs it.
  const base = Math.min(0.09, 0.45 * dt);
  const hold = Math.min(0.09 + 0.06 * roll, (0.45 + 0.75 * roll) * dt);
  const mm = (a: V3, b: V3) => dist3(a, b) / u;
  const state: Record<string, { pos: V3; free: number; last: Press | null }> = {};
  const presses: Press[] = [];
  // `prevFree`: when the previous key could be released (rollover only holds it longer, it never gates the next key).
  let prevArrive = -Infinity, prevFree = -Infinity;
  for (const s of strokes) {
    const k = fingerKey(s.side, s.finger);
    const h = home(s.side, s.finger);
    const st = (state[k] ??= { pos: h, free: -Infinity, last: null });
    const m = moveTime(mm(st.pos, s.pos)) + (st.last && mm(st.pos, s.pos) < 0.5 ? REPRESS : 0);
    const c: [number, Limit][] = [[prevArrive + dt, "rhythm"], [st.free + m, "finger"], [prevFree + (1 - Math.min(p, 1)) * m, "serial"]];
    let [natural, limit] = c[0];
    for (const [t, l] of c) if (t > natural + 1e-9) [natural, limit] = [t, l];
    const first = moveTime(mm(h, s.pos)) + 0.1;
    let arrive = Math.max(strict ? prevArrive + dt : natural, first);
    // A finger needed again too soon lets go of its previous key early.
    let free = st.free;
    if (st.last && free + m > arrive) free = Math.max(st.last.arrive + MIN_MOVE, arrive - m);
    // Set off as early as parallelism allows (not before `f`), so as to be ready by `arrive`.
    const setOff = (f: number, mt: number) => Math.max(f, prevFree - p * Math.max(1, p) * mt, arrive - mt - lead);
    let from = h, depart: number, mt = moveTime(mm(h, s.pos));
    if (!st.last) depart = setOff(-Infinity, mt);
    else {
      const out = moveTime(mm(st.pos, h));
      const atHome = mm(st.pos, h) < 0.5;
      const rel = st.last.up;
      // Home only if the finger would rest there before it needs to set off again.
      if (rel + (atHome ? 0 : out) + DWELL + mt + lead <= arrive) {
        if (!atHome) st.last.back = rel + out;
        depart = setOff(rel + (atHome ? 0 : out) + DWELL, mt);
      } else {
        from = st.pos; depart = free; mt = m;
        st.last.up = Math.min(st.last.up, depart);
      }
    }
    // Natural pace: a longer path (e.g. via home) makes the key later instead of the move faster.
    if (!strict && depart + mt > arrive) arrive = depart + mt;
    depart = Math.min(depart, arrive - MIN_MOVE);
    const ready = Math.min(arrive, depart + mt);
    const rush = mt > 0 && depart + mt > arrive ? mt / (arrive - depart) : 1;
    const pr: Press = { ...s, from, depart, ready, arrive, up: arrive + hold, back: Infinity, limit, rush };
    presses.push(pr);
    st.pos = s.pos; st.free = arrive + base; st.last = pr;
    prevArrive = arrive; prevFree = arrive + base;
  }
  for (const [k, st] of Object.entries(state)) {
    if (!st.last) continue;
    const [side, f] = [k[0] as Side, k.slice(1) as FingerName];
    st.last.back = st.last.up + moveTime(mm(st.pos, home(side, f)));
  }
  return presses;
}

function typingStats(presses: Press[], byFinger: Record<string, Press[]>, home: (side: Side, f: FingerName) => V3, targetWpm: number, dt: number, u: number, natural: Press[]): TypingStats {
  const n = presses.length;
  const perFinger: Record<string, FingerStats> = {};
  if (!n) return { targetWpm, wpm: 0, naturalWpm: 0, rushed: 0, maxRush: 1, overlap: 0, early: 0, fingerWaits: 0, serialWaits: 0, alternation: 0, travel: 0, homeReturns: 0, perFinger };
  const t0 = presses[0].depart, t1 = presses[n - 1].up;
  const span = Math.max(t1 - t0, 1e-6);
  let travel = 0, returns = 0;
  for (const [k, list] of Object.entries(byFinger)) {
    const h = home(k[0] as Side, k.slice(1) as FingerName);
    let tr = 0, busy = 0;
    for (const pr of list) {
      tr += dist3(pr.from, pr.pos) / u;
      busy += pr.up - pr.depart;
      if (Number.isFinite(pr.back)) { tr += dist3(pr.pos, h) / u; busy += pr.back - pr.up; if (pr !== list[list.length - 1]) returns++; }
    }
    travel += tr;
    perFinger[k] = { presses: list.length, travel: tr, busy: Math.min(1, busy / span) };
  }
  // Sweep over [depart, up] intervals for the time with 2+ fingers active.
  const ev: [number, number][] = presses.flatMap((pr) => [[pr.depart, 1], [pr.up, -1]] as [number, number][]).sort((a, b) => a[0] - b[0] || a[1] - b[1]);
  let active = 0, last = t0, multi = 0;
  for (const [t, dlt] of ev) { if (active >= 2) multi += t - last; active += dlt; last = t; }
  let alt = 0, early = 0;
  for (let i = 1; i < n; i++) {
    if (presses[i].side !== presses[i - 1].side) alt++;
    if (presses[i].depart < presses[i - 1].up - 1e-9) early++;
  }
  const wpmOf = (ps: Press[]) => ps.length / 5 / ((ps[ps.length - 1].arrive - ps[0].arrive + dt) / 60);
  const rushedOnes = presses.filter((pr) => pr.rush > 1.05);
  // Strict plans never wait, so count the presses that had to rush instead.
  const late = natural === presses ? presses : rushedOnes;
  return {
    targetWpm,
    wpm: wpmOf(presses),
    naturalWpm: wpmOf(natural),
    rushed: rushedOnes.length,
    maxRush: presses.reduce((m, pr) => Math.max(m, pr.rush), 1),
    overlap: multi / span,
    early: n > 1 ? early / (n - 1) : 0,
    fingerWaits: late.filter((pr) => pr.limit === "finger").length,
    serialWaits: late.filter((pr) => pr.limit === "serial").length,
    alternation: n > 1 ? alt / (n - 1) : 0,
    travel: travel / n,
    homeReturns: returns / n,
    perFinger,
  };
}

/** Minimum-jerk easing (smooth start and stop, like a real reaching movement). */
const minJerk = (t: number) => t * t * t * (10 + t * (6 * t - 15));

/** How far a pressed key (and the fingertip on it) goes down, mm. */
export const KEY_TRAVEL = 3.5;

/** Depth of a press at time `t` as a fraction of KEY_TRAVEL: down and back up over the hold. */
const pressDepth = (pr: Press, t: number) => Math.sin((Math.PI * (t - pr.arrive)) / (pr.up - pr.arrive));

/**
 * Write each board key's press state at time `t` into `out` (indexed by key): 0 when up,
 * otherwise the depth fraction (> 0 for the whole time the key is down).
 */
export function keysDown(plan: Plan, t: number, out: Float32Array): void {
  out.fill(0);
  for (const pr of plan.presses) {
    if (pr.key === undefined || pr.key < 0 || pr.key >= out.length || t < pr.arrive || t >= pr.up) continue;
    out[pr.key] = Math.max(out[pr.key], pressDepth(pr, t), 1e-3);
  }
}

export type FingerState = "rest" | "move" | "ready" | "press";

/**
 * A finger's commanded fingertip at time `t`: at home, moving (minimum-jerk path with a lift
 * arc), waiting just above its key, or pressing (a short dip into the key). `u` = world units per mm.
 */
export function fingerAt(list: Press[] | undefined, home: V3, t: number, u: number): { pos: V3; state: FingerState } {
  let cur = home;
  for (const pr of list ?? []) {
    if (t < pr.depart) return { pos: cur, state: "rest" };
    if (t < pr.ready) return { pos: travel(pr.from, pr.pos, (t - pr.depart) / Math.max(pr.ready - pr.depart, 1e-6), u), state: "move" };
    if (t < pr.arrive) return { pos: pr.pos, state: "ready" };
    if (t < pr.up) {
      const dip = KEY_TRAVEL * u * pressDepth(pr, t);
      const n = pr.n ?? [0, 1, 0];
      return { pos: [pr.pos[0] - n[0] * dip, pr.pos[1] - n[1] * dip, pr.pos[2] - n[2] * dip], state: "press" };
    }
    if (Number.isFinite(pr.back)) {
      if (t < pr.back) return { pos: travel(pr.pos, home, (t - pr.up) / Math.max(pr.back - pr.up, 1e-6), u), state: "move" };
      cur = home;
    } else cur = pr.pos;
  }
  return { pos: cur, state: "rest" };
}

function travel(a: V3, b: V3, t: number, u: number): V3 {
  const x = Math.max(0, Math.min(1, t));
  const p = lerp3(a, b, minJerk(x));
  const mm = dist3(a, b) / u;
  const lift = (mm < 0.5 ? 3 : Math.min(8, 3 + 0.1 * mm)) * u;
  p[1] += lift * Math.sin(Math.PI * x);
  return p;
}

/** Hand shift (world, horizontal) from the fingers' current excursions from home: the hand follows `follow` of each (a third of that for the thumb). */
export function handShift(excursions: [FingerName, V3][], follow = WRIST0.follow): V3 {
  const o: V3 = [0, 0, 0];
  for (const [f, e] of excursions) {
    const k = f === "thumb" ? follow / 3 : follow;
    o[0] += k * e[0]; o[2] += k * e[2];
  }
  return o;
}
