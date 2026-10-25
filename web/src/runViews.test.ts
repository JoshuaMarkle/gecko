import { describe, expect, it } from "vitest";
import { binEdge, eliteGrid, metricColumn, plotMetrics } from "./runViews";
import type { QdAxis, QdElite, Ranked } from "./api";

const ranked = (vals: Record<string, number>): Ranked => ({
  rank: 1, score: 0, chars: "", rows: [], hash: "",
  breakdown: { score: 0, metrics: Object.entries(vals).map(([id, value]) => ({ id, name: id, unit: "", dir: id === "I" ? "info" : "min", value, weight: 1, weighted: 0 })) },
});
const elite = (bins: number[], score: number): QdElite => ({ row: 0, cell: 0, bins, desc: [], score, chars: "", updates: 0 });
const axis = (bins: number): QdAxis => ({ name: "a", lo: 0, hi: 1, bins, auto: false });

describe("run view helpers", () => {
  it("reads metric columns", () => {
    const rows = [ranked({ sfb: 1, effort: 2, I: 3 }), ranked({ sfb: 4 })];
    expect([...metricColumn(rows, "sfb")]).toEqual([1, 4]);
    expect(Number.isNaN(metricColumn(rows, "effort")[1])).toBe(true);
    expect(plotMetrics(rows).map((m) => m.id)).toEqual(["sfb", "effort"]);
    expect(plotMetrics([])).toEqual([]);
  });

  it("projects elites onto two axes, best per cell", () => {
    const es = [elite([0, 1, 0], 5), elite([0, 1, 1], 3), elite([2, 0, 0], 7)];
    const g = eliteGrid(es, [axis(3), axis(2), axis(2)], 0, 1);
    expect([g.w, g.h, g.lo, g.hi]).toEqual([3, 2, 3, 7]);
    expect(g.cells).toEqual([null, null, 2, 1, null, null]);
    // One axis: a single row.
    const r = eliteGrid(es, [axis(3), axis(2), axis(2)], 2, -1);
    expect([r.w, r.h]).toEqual([2, 1]);
    expect(r.cells).toEqual([0, 1]);
    expect(binEdge({ name: "a", lo: 10, hi: 20, bins: 4, auto: true }, 1)).toBe(12.5);
  });
});
