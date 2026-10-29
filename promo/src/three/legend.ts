// Keycap legend textures: the cap colour as ground, the glyph in ink (or white on the accent). Special keys get drawn icons, so the
// legends never depend on a font having ⇥ ⌫ ⇧ ⏎ glyphs. Cached per (symbol, ground, glyph colour).

import * as THREE from "three";
import { SPECIAL } from "../lib/data";

const SIZE = 512;
const cache = new Map<string, THREE.CanvasTexture>();

/** Key icons as SVG path data in a ±100 box, stroked (shared by the 3D legends and the 2D board). */
export const ICONS: Record<string, string> = {
  tab: "M-60 0H50M20 -30L50 0L20 30M62 -34V34",
  bksp: "M-62 0L-30 -34H62V34H-30ZM0 -14L28 14M28 -14L0 14",
  shift: "M0 -52L50 4H22V46H-22V4H-50Z",
  space: "M-64 -12V18H64V-12",
  enter: "M52 -44V16H-48M-22 -10L-48 16L-22 42",
  lower: "M-44 -28H44L0 40Z",
  raise: "M-44 28H44L0 -40Z",
  blank: "",
};

/** Legend stroke and type, in ±100-box units (shared with the 2D board so both draw the same legend). */
export const LEGEND = { stroke: 11, weight: 560, textSize: (text: string) => (text.length === 1 ? 128 : text.length <= 3 ? 76 : 60) } as const;
/** Millimetres per legend unit on a 1 u cap: `legendTexture` spans the 12.6 mm top with 512 px at 1.6 px per unit. */
export const LEGEND_MM = (12.6 / 512) * 1.6;

/** Draw a legend (text or icon) centred at (0, 0) in a ±100 box on any 2D context. */
export function drawLegend(g: CanvasRenderingContext2D, ch: string, color: string, scale = 1) {
  const sp = SPECIAL[ch];
  g.save();
  g.scale(scale, scale);
  g.fillStyle = g.strokeStyle = color;
  if (sp?.icon) {
    g.lineWidth = LEGEND.stroke; g.lineJoin = "round"; g.lineCap = "round";
    if (ICONS[sp.icon]) g.stroke(new Path2D(ICONS[sp.icon]));
  } else {
    const text = sp ? sp.label : ch.toUpperCase();
    g.font = `${LEGEND.weight} ${LEGEND.textSize(text)}px Inter, Helvetica, Arial, sans-serif`;
    g.textAlign = "center"; g.textBaseline = "middle";
    g.fillText(text, 0, text.length === 1 ? 6 : 2);
  }
  g.restore();
}

/** Legend texture for the dished top (UV 0..1 spans the top; aspect = top width / depth). */
export function legendTexture(ch: string, aspect: number, bg: string, fg: string): THREE.CanvasTexture {
  const id = `${ch}|${aspect.toFixed(2)}|${bg}|${fg}`;
  const hit = cache.get(id);
  if (hit) return hit;
  const c = document.createElement("canvas");
  c.width = SIZE;
  c.height = Math.round(SIZE / aspect);
  const g = c.getContext("2d")!;
  g.fillStyle = bg;
  g.fillRect(0, 0, c.width, c.height);
  g.translate(c.width / 2, c.height * 0.46);
  drawLegend(g, ch, fg, (c.width / 256) * 0.8);
  const t = new THREE.CanvasTexture(c);
  t.colorSpace = THREE.SRGBColorSpace;
  t.anisotropy = 8;
  cache.set(id, t);
  return t;
}

/** A canvas texture drawn at `px` per mm for a w × l mm face (canvas top = far end, origin at the top centre). */
export function faceTexture(id: string, w: number, l: number, bg: string, draw: (g: CanvasRenderingContext2D) => void, px = 40): THREE.CanvasTexture {
  const hit = cache.get(id);
  if (hit) return hit;
  const c = document.createElement("canvas");
  c.width = Math.round(w * px); c.height = Math.round(l * px);
  const g = c.getContext("2d")!;
  g.fillStyle = bg; g.fillRect(0, 0, c.width, c.height);
  g.scale(px, px);
  g.translate(w / 2, 0);
  draw(g);
  const t = new THREE.CanvasTexture(c);
  t.colorSpace = THREE.SRGBColorSpace;
  t.anisotropy = 8;
  cache.set(id, t);
  return t;
}
