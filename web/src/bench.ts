// Bench views (research/studies/search-space-visualization.md §3.5): performance profiles
// (Dolan & Moré 2002) and time-to-target ECDFs from a saved `gecko bench` file. Instances are
// the seeds; solvers are algo × moves.

import type { Bench, BenchResult } from "./api";

export const solverName = (r: BenchResult) => (r.moves === "swap" ? r.algo : `${r.algo} [${r.moves}]`);

/** Best score per seed over all solvers. */
export function instanceBest(b: Bench): number[] {
  const best: number[] = [];
  for (const r of b.results) r.scores.forEach((s, i) => { if (Number.isFinite(s)) best[i] = Math.min(best[i] ?? Infinity, s); });
  return best;
}

/** Per solver, its gap to the best solver on each instance, (score − best) / |best|, sorted.
 *  The performance profile ρ(τ) is the share of gaps ≤ τ. */
export function profileGaps(b: Bench): number[][] {
  const best = instanceBest(b);
  return b.results.map((r) => r.scores.map((s, i) => (Number.isFinite(s) ? (s - best[i]) / Math.max(Math.abs(best[i]), 1e-12) : Infinity)).sort((x, y) => x - y));
}

/** Time (ms) at which a seed's best first reached ≤ target; Infinity if never. */
export function timeTo(trace: [number, number][], target: number): number {
  for (const [t, s] of trace) if (s <= target + 1e-12) return t;
  return Infinity;
}

/** Per solver, sorted times to reach `target` over its seeds (ECDF of time-to-target). */
export function ttt(b: Bench, target: number): number[][] {
  return b.results.map((r) => (r.runs ?? []).map((run) => timeTo(run.trace, target)).sort((x, y) => x - y));
}

/** Share of `sorted` values ≤ x. */
export function ecdf(sorted: number[], x: number): number {
  if (!sorted.length) return 0;
  let k = 0;
  while (k < sorted.length && sorted[k] <= x) k++;
  return k / sorted.length;
}
