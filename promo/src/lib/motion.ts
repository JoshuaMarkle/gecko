// Motion language: things ease to a stop. Ease-out entrances, ease-in-out moves, no bounce.
// All times are frames at FPS unless named `s`.

import { Easing } from "remotion";
import { FPS } from "./tokens";

export const sec = (s: number) => Math.round(s * FPS);

/** Durations (frames). */
export const DUR = { fast: 8, enter: 14, move: 24, slow: 45 } as const;
/** Stagger between siblings (frames). */
export const STAGGER = { tight: 2, item: 3, word: 4, line: 6 } as const;

export const EASE = {
  out: Easing.bezier(0.16, 1, 0.3, 1),     // entrances (expo-like out)
  inOut: Easing.bezier(0.65, 0, 0.35, 1),  // moves
  in: Easing.bezier(0.7, 0, 0.84, 0),      // exits, the dive
  linear: (t: number) => t,
} as const;

export const clamp01 = (t: number) => Math.max(0, Math.min(1, t));
export const lerp = (a: number, b: number, t: number) => a + (b - a) * t;

/** 0..1 as `frame` runs from `start` over `dur` frames, eased. */
export function progress(frame: number, start: number, dur: number, ease: (t: number) => number = EASE.out): number {
  if (dur <= 0) return frame >= start ? 1 : 0;
  return ease(clamp01((frame - start) / dur));
}

/** Piecewise interpolation through keyframes `[frame, value]` (sorted), eased per segment. */
export function interp(frame: number, keys: [number, number][], ease: (t: number) => number = EASE.inOut): number {
  if (frame <= keys[0][0]) return keys[0][1];
  for (let i = 1; i < keys.length; i++) {
    const [f1, v1] = keys[i];
    if (frame <= f1) {
      const [f0, v0] = keys[i - 1];
      return lerp(v0, v1, ease((frame - f0) / Math.max(1e-6, f1 - f0)));
    }
  }
  return keys[keys.length - 1][1];
}

/** Critically damped spring from 0 toward 1 after `start`: x(t) = 1 − (1 + ωt)e^(−ωt). No overshoot. */
export function spring(frame: number, start: number, settle: number = DUR.move): number {
  const t = (frame - start) / FPS;
  if (t <= 0) return 0;
  const w = 6 / (settle / FPS); // ≈ 98 % settled after `settle` frames
  return 1 - (1 + w * t) * Math.exp(-w * t);
}

/** Enter at `start`, leave at `exitAt` (if given): 0 → 1 → 0. */
export function presence(frame: number, start: number, exitAt?: number, enter: number = DUR.enter, exit: number = DUR.fast): number {
  const a = progress(frame, start, enter);
  const b = exitAt === undefined ? 0 : progress(frame, exitAt, exit, EASE.in);
  return a * (1 - b);
}
