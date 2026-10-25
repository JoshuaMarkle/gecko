import { describe, expect, it } from "vitest";
import { byShare, groupOf, notInCharset, preview, sameWeights } from "./mix";

describe("preview", () => {
  const a = { corpus: "a", share: 3, mono: { a: 0.5, b: 0.5 } };
  const b = { corpus: "b", share: 1, mono: { a: 0.5, "(": 0.5 } };

  it("weights normalized parts and splits each symbol by part", () => {
    const p = preview([a, b], 0, 0);
    expect(p.mono.a).toBeCloseTo(0.5);
    expect(p.mono["("]).toBeCloseTo(0.125);
    expect(p.contrib.a.slice(0, 2).map((v) => +v.toFixed(3))).toEqual([0.375, 0.125]);
    expect(p.parts[0] + p.parts[1]).toBeCloseTo(1);
  });

  it("adds modelled Backspace at its share and keeps the sum at 1", () => {
    const p = preview([a, b], 0.05, 0);
    expect(p.mono["⌫"]).toBeCloseTo(0.05);
    expect(Object.values(p.mono).reduce((x, y) => x + y, 0)).toBeCloseTo(1);
    expect(p.parts[2]).toBeCloseTo(0.05);
  });
});

describe("symbol groups", () => {
  it("separates letters and keys from punctuation and digits", () => {
    expect(["a", "␣", "⏎", "⌫"].map(groupOf)).toEqual(["keys", "keys", "keys", "keys"]);
    expect(["(", "7", "~"].map(groupOf)).toEqual(["symbols", "symbols", "symbols"]);
    expect(byShare({ a: 0.1, b: 0.3, "(": 0.2 }, "keys")).toEqual(["b", "a"]);
  });

  it("lists ASCII missing from a charset", () => {
    const m = notInCharset("abcdefghijklmnopqrstuvwxyz,./;'-");
    expect(m).toContain("(");
    expect(m).not.toContain("a");
    expect(m).not.toContain("A");
  });

  it("compares recipes by normalized weights", () => {
    expect(sameWeights([{ corpus: "a", weight: 50 }, { corpus: "b", weight: 50 }], [{ corpus: "a", weight: 1 }, { corpus: "b", weight: 1 }])).toBe(true);
    expect(sameWeights([{ corpus: "a", weight: 2 }, { corpus: "b", weight: 1 }], [{ corpus: "a", weight: 1 }, { corpus: "b", weight: 1 }])).toBe(false);
  });
});
