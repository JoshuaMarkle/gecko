import { describe, expect, it } from "vitest";
import type { Geometry, Key } from "./api";
import { checkLayout, rowsOf, swapChars, verdict } from "./format";

const key = (row: number): Key => ({ pos: [0, row, 0], rot: [0, 0, 0], size: [1, 1], finger: "LI", row, col: 0, home: false, stretch: false, effort: 1 });
const g: Geometry = { id: "t", name: "t", description: "", key: [key(0), key(0), key(1), key(1), key(1)] };

describe("format", () => {
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
  });
});
