import { describe, expect, it } from "vitest";
import type { Breakdown, GeomVar } from "./api";
import { asProgress, freeSpec, GEOM_DEFAULTS, groupSpec, metricChanges, PRESET_2D, PRESET_3D, varLabel, withBounds } from "./geom";

const vars: GeomVar[] = [
  { name: "fingers.0.stagger", lo: -0.75, hi: 0.75, start: 0 },
  { name: "split.tent", lo: 0, hi: 45, start: 0 },
];

describe("free spec", () => {
  it("lists groups in the CLI order", () => {
    expect(groupSpec(["tent", "stagger"])).toBe("stagger,tent");
    expect(groupSpec(PRESET_2D)).toBe("stagger,splay,spread,thumb");
    expect(PRESET_3D.length).toBe(10);
    expect(groupSpec(["bogus"])).toBe("");
  });
  it("adds only edited bounds of this board", () => {
    const s = { ...GEOM_DEFAULTS, groups: ["stagger", "tent"], bounds: {
      "corne|split.tent": [0, 30] as [number, number],
      "corne|fingers.0.stagger": [-0.75, 0.75] as [number, number],
      "ortho|split.tent": [5, 10] as [number, number],
      "corne|fingers.9.stagger": [0, 1] as [number, number],
    } };
    expect(freeSpec(s, "corne", vars)).toBe("stagger,tent,split.tent=0:30");
    expect(withBounds(s, "corne", vars)[1]).toEqual({ name: "split.tent", lo: 0, hi: 30, start: 0 });
    expect(freeSpec(s, "other", vars)).toBe("stagger,tent");
  });
});

describe("labels and progress", () => {
  it("names parameters", () => {
    expect(varLabel("fingers.2.stagger")).toEqual(["fingers col 3 stagger", "u"]);
    expect(varLabel("split.tent")).toEqual(["split tent", "°"]);
    expect(varLabel("thumbs.anchor_x")).toEqual(["thumbs position x", "u"]);
  });
  it("maps geometry progress for the convergence chart", () => {
    const p = asProgress({ elapsed_ms: 5, phase: "search", generation: 2, candidates: 17, best: 3.5, best_feasible: null, generation_best: 4, values: [], chars: "abc", collisions: 0 });
    expect(p).toEqual({ elapsed_ms: 5, evals: 17, best: 3.5, best_chars: "abc" });
  });
  it("ranks weighted metric changes", () => {
    const m = (id: string, value: number, weighted: number, weight = 1) => ({ id, name: id, unit: "", dir: "min" as const, value, weight, weighted });
    const a: Breakdown = { score: 0, metrics: [m("A", 1, 1), m("B", 2, 2), m("C", 3, 0, 0)] };
    const b: Breakdown = { score: 0, metrics: [m("A", 0.5, 0.9), m("B", 1, 1), m("C", 9, 0, 0)] };
    expect(metricChanges(a, b).map((r) => r.id)).toEqual(["B", "A"]);
  });
});
