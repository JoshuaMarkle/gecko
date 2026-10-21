// Board facts and a rough goodness estimate for the Board tab (unit-tested in board.test.ts).
// Geometry only: no corpus, so the estimate is a heuristic, labelled as such in the UI.

import type { Geometry } from "./api";
import { flatBounds, flatKeys } from "./format";

/** 1u in mm. */
export const UNIT_MM = 19.05;

export interface BoardStats {
  keys: number;
  split: boolean;
  rows: number;
  cols: number;
  thumbs: number;
  stagger: "ortholinear" | "row stagger" | "column stagger" | "row + column stagger";
  widthMm: number;
  depthMm: number;
  homeKeys: number;
  stretchKeys: number;
  /** Mean base effort of the non-thumb keys. */
  meanEffort: number;
  /** Normalized entropy of keys per non-thumb finger (1 = perfectly even). */
  evenness: number;
}

const isThumb = (f: string) => f === "LT" || f === "RT";

/** Largest spread of `v` within groups of `key`. */
function spread<T>(items: T[], key: (t: T) => number, v: (t: T) => number): number {
  const m = new Map<number, number[]>();
  for (const t of items) { const k = key(t); m.set(k, [...(m.get(k) ?? []), v(t)]); }
  let s = 0;
  for (const xs of m.values()) s = Math.max(s, Math.max(...xs) - Math.min(...xs));
  return s;
}

export function boardStats(g: Geometry): BoardStats {
  const keys = g.key;
  const main = keys.filter((k) => !isThumb(k.finger));
  const left = keys.filter((k) => k.finger[0] === "L"), right = keys.filter((k) => k.finger[0] === "R");
  const gap = left.length && right.length ? Math.min(...right.map((k) => k.pos[0])) - Math.max(...left.map((k) => k.pos[0] + k.size[0])) : 0;
  // Column stagger: one row's keys sit at different heights; row stagger: one column's keys shift sideways.
  const colStag = spread(main, (k) => k.row, (k) => k.pos[1]) > 0.1;
  const rowStag = spread(main.filter((k) => k.size[0] <= 1.01), (k) => k.col, (k) => k.pos[0]) > 0.1;
  const b = flatBounds(flatKeys(g));
  const perFinger = new Map<string, number>();
  for (const k of main) perFinger.set(k.finger, (perFinger.get(k.finger) ?? 0) + 1);
  const n = main.length || 1;
  let h = 0;
  for (const c of perFinger.values()) h -= (c / n) * Math.log(c / n);
  return {
    keys: keys.length,
    split: gap > 0.5,
    rows: new Set(main.map((k) => k.row)).size,
    cols: new Set(main.map((k) => k.col)).size,
    thumbs: keys.length - main.length,
    stagger: colStag && rowStag ? "row + column stagger" : colStag ? "column stagger" : rowStag ? "row stagger" : "ortholinear",
    widthMm: (b.x1 - b.x0) * UNIT_MM,
    depthMm: (b.y1 - b.y0) * UNIT_MM,
    homeKeys: keys.filter((k) => k.home).length,
    stretchKeys: keys.filter((k) => k.stretch).length,
    meanEffort: main.reduce((t, k) => t + k.effort, 0) / n,
    evenness: h / Math.log(8),
  };
}

const clamp01 = (x: number) => Math.max(0, Math.min(1, x));

/** Rough 0–100 estimate: comfort 40%, even spread 25%, thumb zone 20%, compactness 15%. */
export function boardScore(s: BoardStats): { total: number; comfort: number; spread: number; thumb: number; compact: number } {
  const comfort = clamp01((4 - s.meanEffort) / 2.5);
  const spread = clamp01(s.evenness);
  const perHand = s.thumbs / 2;
  const thumb = perHand === 0 ? 0 : clamp01(1 - Math.abs(perHand - 2.5) / 3);
  const compact = clamp01(1 - s.stretchKeys / Math.max(1, s.keys - s.thumbs) * 2);
  return { total: 100 * (0.4 * comfort + 0.25 * spread + 0.2 * thumb + 0.15 * compact), comfort, spread, thumb, compact };
}
