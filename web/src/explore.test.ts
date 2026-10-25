import { describe, expect, it } from "vitest";
import { histogram, logGrid, paretoFront, rankOrder, sameTables, stabilityRange, swapList, symIds } from "./explore";
import type { Profile } from "./api";

describe("explore helpers", () => {
  it("ranks and filters", () => {
    expect([...rankOrder([3, 1, 2])]).toEqual([1, 2, 0]);
    expect([...rankOrder([3, 1, 2], (i) => i !== 1)]).toEqual([2, 0]);
  });

  it("finds the Pareto front", () => {
    const xs = [1, 2, 3, 2.5], ys = [5, 3, 1, 4];
    expect(paretoFront(xs, ys, [0, 1, 2, 3], true, true)).toEqual([0, 1, 2]);
    // Maximize y instead: only the leftmost/highest survive.
    expect(paretoFront(xs, ys, [0, 1, 2, 3], true, false)).toEqual([0]);
  });

  it("bins values", () => {
    expect(histogram([0, 0.5, 1, 2], 0, 2, 2)).toEqual([2, 2]);
  });

  it("measures weight stability", () => {
    const f = logGrid(0.25, 4, 5);
    expect(f[2]).toBe(1);
    expect(stabilityRange([7, 3, 3, 3, 9], f, 3)).toEqual({ lo: f[1], hi: f[3], below: 7, above: 9 });
    expect(stabilityRange([3, 3, 3, 3, 3], f, 3)).toEqual({ lo: f[0], hi: f[4], below: null, above: null });
  });

  it("maps symbols and swaps", () => {
    expect([...symIds("cab", "abc")!]).toEqual([2, 0, 1]);
    expect(symIds("abd", "abc")).toBeNull();
    const m = [0, 0.5, -1, 0.5, 0, 2, -1, 2, 0];
    expect(swapList(m, 3).map((s) => [s.i, s.j])).toEqual([[0, 2], [0, 1], [1, 2]]);
  });

  it("compares profile tables", () => {
    const p = { name: "a", description: "", weights: { sfb: 1 }, finger_target: [1], trigram_coverage: 0.99 } as Profile;
    expect(sameTables(p, { ...p, weights: { sfb: 2 } })).toBe(true);
    expect(sameTables(p, { ...p, trigram_coverage: 1 })).toBe(false);
  });
});
