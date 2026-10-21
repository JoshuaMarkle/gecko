import { describe, expect, it } from "vitest";
import type { RunSummary } from "./api";
import { fmtPow10, log10Factorial, rediscovery, searchSpace } from "./estimate";

const run = (best_score: number | null, elapsed_ms = 1000): RunSummary =>
  ({ id: "x", created: "", algo: "sa", profile: "p", corpus: "c", geometry: "g", best_score, best: null, elapsed_ms, status: "done" });

describe("search space", () => {
  it("log10 n! matches exact values", () => {
    expect(log10Factorial(10)).toBeCloseTo(Math.log10(3628800), 9);
    expect(log10Factorial(30)).toBeCloseTo(32.42366, 4);
  });
  it("pins shrink the space", () => {
    const s = searchSpace(30, 4);
    expect(s.effective).toBeCloseTo(log10Factorial(26), 9);
    expect(fmtPow10(s.total)).toMatch(/× 10\^32/);
  });
});

describe("rediscovery", () => {
  it("is 0 with one hit", () => {
    expect(rediscovery([run(3), run(3.2)]).chance).toBe(0);
  });
  it("rises as runs agree", () => {
    const some = rediscovery([run(3), run(3), run(3.1), run(3.2)]);
    const all = rediscovery([run(3), run(3), run(3), run(3)]);
    expect(some.hits).toBe(2);
    expect(all.chance).toBeGreaterThan(some.chance);
    expect(all.chance).toBeLessThanOrEqual(1);
  });
});
