import { describe, expect, it } from "vitest";
import type { Finger, Geometry, Key } from "./api";
import { boardScore, boardStats } from "./board";

const key = (x: number, y: number, finger: Finger, row: number, col: number): Key =>
  ({ pos: [x, y, 0], rot: [0, 0, 0], size: [1, 1], finger, row, col, home: row === 1, stretch: false, effort: row === 1 ? 1 : 2 });
const fingers: Finger[] = ["LP", "LR", "LM", "LI", "RI", "RM", "RR", "RP"];
const grid = (gap: number, stagger: number[]): Geometry => ({
  id: "t", name: "t", description: "",
  key: [0, 1, 2].flatMap((row) => fingers.map((f, c) => key(c + (c >= 4 ? gap : 0), row + stagger[c], f, row, c))),
});

describe("boardStats", () => {
  it("detects an ortholinear unibody", () => {
    const s = boardStats(grid(0, [0, 0, 0, 0, 0, 0, 0, 0]));
    expect(s).toMatchObject({ keys: 24, split: false, rows: 3, cols: 8, thumbs: 0, stagger: "ortholinear" });
    expect(s.evenness).toBeCloseTo(1, 6);
  });
  it("detects a column-staggered split", () => {
    const s = boardStats(grid(2, [0, 0.2, 0.4, 0.2, 0.2, 0.4, 0.2, 0]));
    expect(s.split).toBe(true);
    expect(s.stagger).toBe("column stagger");
    expect(boardScore(s).total).toBeGreaterThan(0);
  });
});
