import { describe, expect, it } from "vitest";
import type { Bench, Progress, WorkerTrace } from "./api";
import { ecdf, instanceBest, profileGaps, timeTo, ttt } from "./bench";
import { forceLayout, subgraph } from "./lon";
import { frames, schedule, settling, stn, workerLines } from "./process";

const w = (o: Partial<WorkerTrace>): WorkerTrace => ({ evals: 0, current: 1, best: 1, proposed: 0, improving: 0, worse: 0, worse_accepted: 0, chars: "abc", ...o });
const rec = (t: number, best_chars: string, workers?: WorkerTrace[]): Progress => ({ elapsed_ms: t, evals: 0, best: 1, best_chars, workers });

describe("process views", () => {
  it("splits worker lines and pools the schedule", () => {
    const ps = [rec(0, "abc", [w({ current: 5, temp: 2, proposed: 10, improving: 2, worse: 8, worse_accepted: 4 }), w({ current: 6, temp: 4, proposed: 10, improving: 0, worse: 2, worse_accepted: 0 })]), rec(10, "abc")];
    expect(workerLines(ps)).toEqual([[[0, 5]], [[0, 6]]]);
    expect(schedule(ps)).toEqual([{ t: 0, temp: 3, accept: 0.4, improving: 0.1 }]);
  });

  it("records when each symbol settles", () => {
    const ps = [rec(0, "abc"), rec(5, "abc"), rec(10, "bac"), rec(20, "bca")];
    const fs = frames(ps, null);
    expect(fs.map((f) => f.t)).toEqual([0, 10, 20]);
    expect(settling(fs)).toEqual([{ sym: "a", t: 20, moves: 2 }, { sym: "c", t: 20, moves: 1 }, { sym: "b", t: 10, moves: 1 }]);
    expect(frames([rec(0, "abc", [w({ chars: "cab" })])], 0)[0].chars).toEqual(["c", "a", "b"]);
  });
});

describe("search trajectory network", () => {
  it("merges nearby layouts into locations and links consecutive visits", () => {
    const ps = [
      rec(0, "", [w({ chars: "abcdef", current: 9 }), w({ chars: "fedcba", current: 8 })]),
      rec(1, "", [w({ chars: "bacdef", current: 7 }), w({ chars: "fedcba", current: 8 })]),
      rec(2, "", [w({ chars: "fedcab", current: 5 }), w({ chars: "fedcab", current: 6 })]),
    ];
    const g = stn(ps, 2);
    // abcdef and bacdef (2 slots apart) are one location; fedcba and fedcab too.
    expect(g.nodes.map((n) => [n.chars, n.score, n.visits, n.workers])).toEqual([["abcdef", 7, 2, [0]], ["fedcba", 5, 4, [1, 0]]]);
    expect(g.edges).toEqual([{ a: 0, b: 1, w: 1, worker: 0 }]);
    expect(g.best).toBe(1);
    expect(stn(ps, 0).nodes.length).toBe(4);
  });
});

describe("bench views", () => {
  const b = {
    results: [
      { algo: "sa", moves: "swap", scores: [10, 12], runs: [{ trace: [[5, 11], [9, 10]] }, { trace: [[3, 12]] }] },
      { algo: "hc", moves: "swap", scores: [11, 12], runs: [{ trace: [[1, 11]] }, { trace: [[2, 12]] }] },
    ],
  } as unknown as Bench;
  it("measures gaps to the best solver per instance", () => {
    expect(instanceBest(b)).toEqual([10, 12]);
    expect(profileGaps(b)).toEqual([[0, 0], [0, 0.1]]);
  });
  it("finds time to target and its ECDF", () => {
    expect(timeTo([[5, 11], [9, 10]], 10.5)).toBe(9);
    expect(timeTo([[5, 11]], 10)).toBe(Infinity);
    expect(ttt(b, 11)).toEqual([[5, Infinity], [1, Infinity]]);
    expect(ecdf([1, 2, Infinity], 2)).toBeCloseTo(2 / 3);
  });
});

describe("local optima network", () => {
  it("keeps edges among the best rows and merges repeats", () => {
    const edges = [5, 7, 2, 0, 7, 5, 1, 0, 5, 9, 1, 0, 5, 5, 3, 0];
    expect(subgraph([5, 7, 1], 2, edges)).toEqual({ rows: [5, 7], edges: [{ a: 0, b: 1, w: 3 }] });
  });
  it("lays out deterministically inside the unit square", () => {
    const e = [{ a: 0, b: 1, w: 1 }, { a: 1, b: 2, w: 1 }];
    const a = forceLayout(4, e, 100, 3), b = forceLayout(4, e, 100, 3);
    expect([...a]).toEqual([...b]);
    expect([...a].every((v) => v >= 0 && v <= 1)).toBe(true);
    // Connected nodes end up closer than the unconnected one is to them.
    const d = (i: number, j: number) => Math.hypot(a[2 * i] - a[2 * j], a[2 * i + 1] - a[2 * j + 1]);
    expect(d(0, 1)).toBeLessThan(Math.max(d(0, 3), d(1, 3)));
  });
});
