// Pure helpers for the Saved view's multi-objective and MAP-Elites panels (unit-tested in
// runViews.test.ts): metric columns of ranked layouts, and a 2-axis projection of a QD grid.

import type { MetricValue, QdAxis, QdElite, Ranked } from "./api";

/** Metric `id`'s value for every layout (NaN where its breakdown lacks it). */
export function metricColumn(rows: Ranked[], id: string): Float64Array {
  return Float64Array.from(rows, (r) => r.breakdown.metrics.find((m) => m.id === id)?.value ?? NaN);
}

/** Metrics worth plotting: every non-informational metric of the first layout's breakdown. */
export function plotMetrics(rows: Ranked[]): MetricValue[] {
  return rows[0]?.breakdown.metrics.filter((m) => m.dir !== "info") ?? [];
}

export interface EliteGrid {
  w: number;
  h: number;
  /** Elite index (into `elites`) per cell, `x + y * w`; null = empty. The best elite wins when
   *  several project onto one cell (more than two axes). */
  cells: (number | null)[];
  /** Best and worst elite score. */
  lo: number;
  hi: number;
}

/** The grid over axes `ax` (columns) and `ay` (rows; -1 = a single row). */
export function eliteGrid(elites: QdElite[], axes: QdAxis[], ax: number, ay: number): EliteGrid {
  const w = axes[ax]?.bins ?? 1, h = ay >= 0 ? axes[ay]?.bins ?? 1 : 1;
  const cells: (number | null)[] = new Array(w * h).fill(null);
  let lo = Infinity, hi = -Infinity;
  elites.forEach((e, i) => {
    const k = (e.bins[ax] ?? 0) + (ay >= 0 ? e.bins[ay] ?? 0 : 0) * w;
    const cur = cells[k];
    if (cur === null || e.score < elites[cur].score) cells[k] = i;
    lo = Math.min(lo, e.score);
    hi = Math.max(hi, e.score);
  });
  return { w, h, cells, lo, hi };
}

/** Lower edge of bin `b` on `axis`. */
export const binEdge = (axis: QdAxis, b: number) => axis.lo + ((axis.hi - axis.lo) * b) / axis.bins;
