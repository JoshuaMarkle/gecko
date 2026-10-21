// Rough estimators for the Search and Home views (unit-tested in estimate.test.ts): search-space
// size, and how likely past runs have found the optimum. Heuristics, labelled as such in the UI;
// the Gilmore–Lawler bound (#37), when a run reports it, is the hard evidence.

import type { RunSummary } from "./api";

/** log10(n!) via Stirling's series (exact enough for display; n = 0 → 0). */
export function log10Factorial(n: number): number {
  if (n < 2) return 0;
  if (n < 20) { let t = 0; for (let i = 2; i <= n; i++) t += Math.log10(i); return t; }
  const ln = n * Math.log(n) - n + 0.5 * Math.log(2 * Math.PI * n) + 1 / (12 * n) - 1 / (360 * n ** 3);
  return ln / Math.LN10;
}

/** `10^e` written for people: 3.6 × 10^6, or plain below 10^6. */
export function fmtPow10(e: number): string {
  if (e < 6) return Math.round(10 ** e).toLocaleString();
  const m = Math.floor(e);
  return `${(10 ** (e - m)).toFixed(1)} × 10^${m}`;
}

export interface Space {
  /** Slots that hold a symbol (layer slots included). */
  slots: number;
  /** Symbols kept in place (pins). */
  pinned: number;
  /** log10 of all arrangements of the slots. */
  total: number;
  /** log10 of arrangements of the unpinned symbols over the unpinned slots. */
  effective: number;
}

export function searchSpace(slots: number, pinned: number): Space {
  const free = Math.max(0, slots - pinned);
  return { slots, pinned, total: log10Factorial(slots), effective: log10Factorial(free) };
}

export interface Confidence {
  /** Matching finished runs. */
  runs: number;
  /** Best score among them. */
  best: number | null;
  /** Runs that reached the best (within 1e-6 relative). */
  hits: number;
  /** Estimated chance a single run finds this optimum (rediscovery rate). */
  perRun: number;
  /** Estimated chance the best known layout is the optimum. */
  chance: number;
  /** Mean run length (ms). */
  meanMs: number;
}

/**
 * Chance the best layout found so far is optimal, from how often independent runs on the same
 * inputs rediscover it. The first find is discounted (a layout is always found once), so the
 * per-run rate is (hits − 1) / runs and the chance that a better optimum would still have been
 * missed by every run is (1 − rate)^runs. One run, or one hit: 0.
 */
export function rediscovery(runs: RunSummary[]): Confidence {
  const done = runs.filter((r) => r.best_score !== null && r.status !== "running");
  const best = done.length ? Math.min(...done.map((r) => r.best_score!)) : null;
  const hits = best === null ? 0 : done.filter((r) => Math.abs(r.best_score! - best) <= 1e-6 * Math.max(1, Math.abs(best))).length;
  const n = done.length;
  const perRun = n ? Math.max(0, hits - 1) / n : 0;
  const chance = perRun > 0 ? 1 - (1 - perRun) ** n : 0;
  const ms = done.map((r) => r.elapsed_ms ?? 0);
  return { runs: n, best, hits, perRun, chance, meanMs: ms.length ? ms.reduce((a, b) => a + b, 0) / ms.length : 0 };
}

/** Expected time for one more find of the optimum: mean run length / per-run rate. */
export function timeToOptimum(c: Confidence): number | null {
  return c.perRun > 0 ? c.meanMs / c.perRun : null;
}
