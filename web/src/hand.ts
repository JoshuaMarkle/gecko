// Hand model for the Hand tab: measurements, presets, planar finger IK, and a synthetic
// population for the size and similarity charts. Lengths in mm. Pure (unit-tested in
// hand.test.ts). Not used by the scorer yet (#92): the "Use hand in optimization" toggle only
// previews. The population is synthetic until reference hand data lands (frontend.md).

export type FingerName = "thumb" | "index" | "middle" | "ring" | "pinky";
export const FINGER_NAMES: FingerName[] = ["thumb", "index", "middle", "ring", "pinky"];

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
export interface HandSettings { use: boolean; preset: Preset; custom: HandDims | null }

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

/** Wrist crease to middle fingertip. */
export const handLength = (d: HandDims) => d.palmLength + d.middle[0] + d.middle[1] + d.middle[2];

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

// ---------- IK ----------

export type V3 = [number, number, number];
const add = (a: V3, b: V3): V3 => [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
const scale = (a: V3, s: number): V3 => [a[0] * s, a[1] * s, a[2] * s];

/** Tip of a planar chain with base angle 0 and joint flexions `b`, `ratio·b` (2D: u forward, v up). */
function tip2(l: [number, number, number], b: number, ratio: number): [number, number] {
  const a = [0, b, b + ratio * b];
  let u = 0, v = 0;
  for (let i = 0; i < 3; i++) { u += l[i] * Math.cos(a[i]); v -= l[i] * Math.sin(a[i]); }
  return [u, v];
}

/**
 * Joint positions (base, 2 joints, tip) of a 3-segment finger reaching from `base` toward
 * `target` in the vertical plane through both. The two distal joints flex together (the
 * second by `ratio` times the first, as tendons couple PIP and DIP); the first flexion is found
 * by bisection so the chain's length matches the distance, then the chain is rotated onto the
 * target. Out of reach: the finger points straight at the target.
 */
export function fingerChain(base: V3, target: V3, l: [number, number, number], ratio = 0.7): V3[] {
  const dx = target[0] - base[0], dz = target[2] - base[2];
  const h = Math.hypot(dx, dz);
  const dir: V3 = h > 1e-9 ? [dx / h, 0, dz / h] : [0, 0, -1];
  const tu = h, tv = target[1] - base[1];
  const dist = Math.hypot(tu, tv);
  const reach = (b: number) => Math.hypot(...tip2(l, b, ratio));
  let b = 0;
  if (dist < reach(0)) {
    let lo = 0, hi = 1.9;
    if (reach(hi) > dist) b = hi;
    else {
      for (let i = 0; i < 40; i++) { const m = (lo + hi) / 2; if (reach(m) > dist) lo = m; else hi = m; }
      b = (lo + hi) / 2;
    }
  }
  const [pu, pv] = tip2(l, b, ratio);
  const rot = Math.atan2(tv, tu) - Math.atan2(pv, pu);
  const angles = [0, b, b + ratio * b];
  const pts: V3[] = [base];
  let cur = base;
  for (let i = 0; i < 3; i++) {
    const a = angles[i] - rot;
    const step = add(scale(dir, l[i] * Math.cos(a)), [0, -l[i] * Math.sin(a), 0]);
    cur = add(cur, step);
    pts.push(cur);
  }
  return pts;
}

export interface HandPose {
  /** Joint chains for thumb, index, middle, ring, pinky (base → tip). */
  fingers: V3[][];
  /** Palm outline: wrist (outer), wrist (inner), thumb base, index … pinky knuckles. */
  palm: V3[];
}

/**
 * Pose one hand so each fingertip rests on its target (three.js coordinates, y up, z toward
 * the user, units `u` per mm). `side` = "L" puts the thumb on the +x side. The knuckles sit in
 * a fixed arc behind and above the four finger targets; the wrist sits a palm length behind.
 */
export function poseHand(side: "L" | "R", targets: Record<FingerName, V3>, d: HandDims, u: number): HandPose {
  const s = side === "L" ? 1 : -1;
  const L = (x: [number, number, number]) => x.map((v) => v * u) as [number, number, number];
  const order: FingerName[] = ["index", "middle", "ring", "pinky"];
  const cx = order.reduce((t, f) => t + targets[f][0], 0) / 4;
  const cz = order.reduce((t, f) => t + targets[f][2], 0) / 4;
  const ty = Math.max(...order.map((f) => targets[f][1]));
  // Natural curl: PIP flexed 43°, DIP 30°, proximal phalanx pitched 9° down; knuckles go where
  // that pose puts the middle fingertip on its target.
  const [pu, pv] = tip2(L(d.middle), 0.75, 0.7);
  const pitch = -0.15;
  const du = pu * Math.cos(pitch) - pv * Math.sin(pitch), dv = pu * Math.sin(pitch) + pv * Math.cos(pitch);
  const kz = cz + du;
  const ky = ty - dv;
  const pw = d.palmWidth * u;
  // Index nearest the thumb side; small arch: middle knuckle furthest forward.
  const arch = [0.3, 0, 0.25, 0.7];
  const knuckles: V3[] = order.map((_, i) => [cx + s * pw * (0.5 - i / 3), ky - arch[i] * 4 * u, kz + arch[i] * 6 * u]);
  const wrist: V3 = [cx, ky - 10 * u, kz + d.palmLength * u];
  const thumbBase: V3 = [cx + s * pw * 0.55, ky - 14 * u, kz + d.palmLength * 0.62 * u];
  const fingers: V3[][] = [fingerChain(thumbBase, targets.thumb, L(d.thumb), 0.5)];
  order.forEach((f, i) => fingers.push(fingerChain(knuckles[i], targets[f], L(d[f]))));
  const wo: V3 = [wrist[0] - s * pw * 0.42, wrist[1], wrist[2]];
  const wi: V3 = [wrist[0] + s * pw * 0.38, wrist[1], wrist[2]];
  return { fingers, palm: [wo, wi, thumbBase, ...knuckles] };
}
