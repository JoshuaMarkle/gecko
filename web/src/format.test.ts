import { describe, expect, it } from "vitest";
import type { Geometry, Key } from "./api";
import { checkLayout, fitChars, layerSlots, rowsOf, swapChars, undirected, verdict } from "./format";

const key = (row: number): Key => ({ pos: [0, row, 0], rot: [0, 0, 0], size: [1, 1], finger: "LI", row, col: 0, home: false, stretch: false, effort: 1 });
const g: Geometry = { id: "t", name: "t", description: "", key: [key(0), key(0), key(1), key(1), key(1)] };

describe("format", () => {
  it("merges directional pairs", () => {
    const u = undirected([{ ngram: "ab", freq: 1, kind: "sfb" }, { ngram: "cd", freq: 2.5, kind: "lsb" }, { ngram: "ba", freq: 2, kind: "sfb" }]);
    expect(u).toEqual([{ ngram: "ab", freq: 3, kind: "sfb" }, { ngram: "cd", freq: 2.5, kind: "lsb" }]);
  });
  it("splits rows by geometry", () => {
    expect(rowsOf("abcde", g)).toEqual(["ab", "cde"]);
  });
  it("swaps", () => {
    expect(swapChars("abcde", 0, 4)).toBe("ebcda");
  });
  it("judges direction", () => {
    expect(verdict("min", 1, 2)).toBe("better");
    expect(verdict("max", 1, 2)).toBe("worse");
    expect(verdict("min", 1.0, 1.01)).toBe("same");
    expect(verdict("info", 1, 9)).toBe("same");
  });
  it("validates layouts", () => {
    expect(checkLayout("abc", 3, "abc")).toBeNull();
    expect(checkLayout("aab", 3, "abc")).toMatch(/twice/);
    expect(checkLayout("ab", 3, "abc")).toMatch(/2 symbols/);
    expect(checkLayout("abz", 3, "abc")).toMatch(/charset/);
    expect(checkLayout("a\u00B7\u00B7\u23CE", 4, "abc")).toBeNull();
    expect(checkLayout("a\u21E7b\u21E7", 4, "abc")).toBeNull();
  });
  it("fits thumb fills and maps layer slots", () => {
    const k = (finger: Key["finger"], fill?: string): Key => ({ ...key(0), finger, fill });
    const t: Geometry = { id: "t", name: "t", description: "", key: [k("LI"), k("RI"), k("LT", "space"), k("RT", "none")] };
    expect(fitChars("ab", t)).toBe("ab\u2423\u00B7");
    expect(fitChars("abc", t)).toBe("abc");
    expect(layerSlots(t, "ab\u2423\u00B7")).toEqual([[0, 1, 2, 3]]);
    // Base with layer 1 held by LT: layer 1 skips the LT key.
    expect(layerSlots(t, "ab\u2460\u00B7" + "()" + "\u00B7")).toEqual([[0, 1, 2, 3], [4, 5, null, 6]]);
    // One-shot and toggle layers skip only the layer key.
    const t2: Geometry = { ...t, key: [...t.key, k("LT", "none")] };
    expect(layerSlots(t2, "ab\u24F5\u00B7\u00B7" + "()" + "\u00B7\u00B7")).toEqual([[0, 1, 2, 3, 4], [5, 6, null, 7, 8]]);
    expect(layerSlots(t2, "ab\u2776\u00B7\u00B7" + "()" + "\u00B7\u00B7")).toEqual([[0, 1, 2, 3, 4], [5, 6, null, 7, 8]]);
    expect(layerSlots(t2, "ab\u2460\u00B7\u00B7" + "()" + "\u00B7")).toEqual([[0, 1, 2, 3, 4], [5, 6, null, 7, null]]);
  });
});
