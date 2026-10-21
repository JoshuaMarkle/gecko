import { describe, expect, it } from "vitest";
import { AVERAGE, dimsVector, fingerChain, handLength, normalCdf, population, scaleDims, similarity, sizeBand, type V3 } from "./hand";

const dist = (a: V3, b: V3) => Math.hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2]);

describe("fingerChain", () => {
  const l: [number, number, number] = [3, 2, 1.5];
  it("keeps segment lengths and reaches a target within range", () => {
    const base: V3 = [0, 3, 3], target: V3 = [0.5, 0, 0];
    const pts = fingerChain(base, target, l);
    expect(pts).toHaveLength(4);
    pts.slice(1).forEach((p, i) => expect(dist(pts[i], p)).toBeCloseTo(l[i], 6));
    expect(dist(pts[3], target)).toBeLessThan(1e-3);
  });
  it("points straight at an unreachable target", () => {
    const pts = fingerChain([0, 0, 0], [0, 0, -20], l);
    expect(pts[3][2]).toBeCloseTo(-6.5, 6);
  });
});

describe("hand sizes", () => {
  it("bands the presets", () => {
    expect(sizeBand(handLength(AVERAGE))).toBe("Average");
    expect(sizeBand(handLength(scaleDims(AVERAGE, 0.88)))).toBe("Small");
    expect(sizeBand(handLength(scaleDims(AVERAGE, 1.12)))).toBe("Large");
  });
  it("normal CDF is symmetric", () => {
    expect(normalCdf(0)).toBeCloseTo(0.5, 6);
    expect(normalCdf(1.96)).toBeCloseTo(0.975, 3);
  });
  it("similarity is 1 for identical hands and falls with distance", () => {
    const v = dimsVector(AVERAGE);
    expect(similarity(v, v)).toBe(1);
    expect(similarity(v, dimsVector(scaleDims(AVERAGE, 1.1)))).toBeLessThan(similarity(v, dimsVector(scaleDims(AVERAGE, 1.02))));
  });
  it("population is deterministic", () => {
    expect(population(5, 3)).toEqual(population(5, 3));
  });
});
