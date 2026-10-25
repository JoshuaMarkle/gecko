import { describe, expect, it } from "vitest";
import { flatKeys } from "./format";
import type { Analysis, Finger, Geometry, Key } from "./api";
import { boardSvg, ergogenYaml, kleJson, layerChars, layoutJson, layoutTextExport, layoutToml, slug, statsCsv, statsMarkdown, type ExportInfo } from "./export";

const key = (x: number, row: number, finger: Finger, rot = 0): Key => ({ pos: [x, row, 0], rot: [0, 0, rot], size: [1, 1], finger, row, col: x, home: false, stretch: false, effort: 1 });
// Two letters, then a thumb row: layer-1 hold key and space. Layer 1 skips the holding thumb.
const g: Geometry = { id: "tiny", name: "Tiny", description: "", key: [key(0, 0, "LP"), key(1, 0, "LR"), key(0, 1, "LT", 15), key(1, 1, "RT")] };
const chars = "ab①␣<2·";

const metric = (id: string, value: number, weight: number) => ({ id, name: `Metric ${id}`, unit: "%", dir: "min" as const, value, weight, weighted: value * weight });
const analysis = (score: number, v: number): Analysis => ({
  breakdown: { score, metrics: [metric("sfb", v, 1), metric("X01", 7, 0)] },
  finger_load: [60, 40], finger_target: [50, 50], fingers: ["LP", "LR"], heat: [4, 2, 0, 1, 1, 0, 0],
  worst_sfb: [], worst_sfs: [], worst_lsb: [], worst_scissors: [], bigrams: [], skipgrams: [],
});
const info: ExportInfo = { name: "My <layout>", geometry: g, chars, a: analysis(1.5, 2), corpus: "english", profile: "default", ref: { name: "QWERTY", a: analysis(2, 3) } };

describe("export", () => {
  it("splits layers per key", () => {
    expect(layerChars(g, chars)).toEqual([["a", "b", "①", "␣"], ["<", "2", "·", "·"]]);
  });
  it("names files", () => {
    expect(slug("Colemak-DH (edited)")).toBe("colemak-dh-edited");
    expect(slug("…")).toBe("layout");
  });
  it("writes a gecko layout file with tokens and layers", () => {
    expect(layoutToml(info)).toBe(
      'name = "My <layout>"\ngeometry = "tiny"\nsource = "GECKO export · english / default · score 1.500"\nrows = ["ab", "{layer1}{space}"]\n'
      + '\n[[layer]]\nname = "layer1"\nrows = ["<2", "··"]\n');
  });
  it("writes text and json", () => {
    expect(layoutTextExport(info)).toBe("ab\n①␣\n\n<2\n··\n");
    const j = JSON.parse(layoutJson(info));
    expect(j.rows).toEqual(["ab", "①␣"]);
    expect(j.layers).toEqual([["<2", "··"]]);
    expect(j.score).toBe(1.5);
    expect(j.finger_load).toEqual({ LP: 60, LR: 40 });
    expect(JSON.parse(layoutJson({ ...info, a: null })).metrics).toEqual([]);
  });
  it("writes KLE with rotation origins and layer legends", () => {
    const k = JSON.parse(kleJson(info, "fingers"));
    expect(k[0].name).toBe("My <layout>");
    expect(k).toHaveLength(5);
    expect(k[1]).toEqual([{ r: 0, rx: 0.5, ry: 0.5, x: -0.5, y: -0.5, w: 1, h: 1, c: "#a98ae0" }, "a\n<"]);
    expect(k[3][0].r).toBe(15);
    expect(k[3][1]).toBe("L1");
    expect(k[4][1]).toBe("Space");
  });
  it("writes ergogen zones at the key centers", () => {
    const y = ergogenYaml(info);
    expect(y).toContain("points:\n  zones:\n");
    expect(y).toContain("outlines:\n  plate:\n    - what: rectangle\n      where: true\n      size: 14\n");
    const zones = [...y.matchAll(/^ {4}(k\d+): (\{.*\})$/gm)].map((m) => [m[1], JSON.parse(m[2])] as const);
    expect(zones.map(([n]) => n)).toEqual(["k0", "k1", "k2", "k3"]);
    flatKeys(g).forEach((k, i) => {
      const z = zones[i][1];
      // Back from ergogen (mm, y up, counter-clockwise) to the 2D drawing (u, y down, clockwise).
      expect(z.anchor.shift[0] / 19.05 - k.w / 2).toBeCloseTo(k.x, 4);
      expect(-z.anchor.shift[1] / 19.05 - k.h / 2).toBeCloseTo(k.y, 4);
      expect(-z.anchor.rotate).toBeCloseTo(k.rot, 4);
      expect(z.key.width).toBeCloseTo(k.w * 19.05, 3);
      expect(z.key.finger).toBe(g.key[i].finger);
    });
    expect(zones[0][1].key).toMatchObject({ name: "k0", label: "a", layers: ["<"] });
    expect(zones[2][1].key.label).toBe("L1");
  });
  it("writes stats", () => {
    const csv = statsCsv(info).trim().split("\n");
    expect(csv[0]).toBe("id,name,unit,direction,value,weight,weighted,QWERTY value");
    expect(csv[1]).toBe("sfb,Metric sfb,%,min,2,1,2,3");
    expect(csv[3]).toBe("score,Total score,,min,1.5,,,2");
    expect(statsCsv({ ...info, a: null })).toBe("");
    const md = statsMarkdown(info);
    expect(md).toContain("```\nab\n");
    expect(md).toContain("Score **1.500** (lower is better), -0.500 vs QWERTY.");
    expect(md).toContain("| Metric sfb | 2.00% | 3.00% |");
    expect(md).not.toContain("X01");
  });
  it("draws the board as standalone svg", () => {
    const one = boardSvg(info, { color: "heat", layers: false, title: true });
    expect(one.startsWith('<svg xmlns="http://www.w3.org/2000/svg"')).toBe(true);
    expect(one.match(/<rect /g)).toHaveLength(1 + 4);
    expect(one).toContain("My &lt;layout&gt;");
    expect(one).toContain('transform="rotate(15 ');
    const both = boardSvg(info, { color: "plain", layers: true, title: false });
    expect(both.match(/<rect /g)).toHaveLength(1 + 8);
    expect(both).toContain(">Layer 1 · hold L1<");
    expect(both).toContain(">&lt;<");
  });
});
