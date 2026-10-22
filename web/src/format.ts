// Small pure helpers for display and data shaping (unit-tested in format.test.ts).

import type { Dir, Geometry, MetricValue, Pair } from "./api";

/** A key as drawn in 2D: top-left `x, y`, size `w, h`, rotation `rot` (degrees) about its center. */
export interface FlatKey { x: number; y: number; w: number; h: number; rot: number }

/** The board's 2D drawing: `geometry.flat` (gecko-core `Geometry::flat2d`) or, without it, x, y and z-rotation. */
export function flatKeys(g: Geometry): FlatKey[] {
  return g.key.map((k, i) => {
    const f = g.flat?.[i];
    return { x: f ? f.pos[0] : k.pos[0], y: f ? f.pos[1] : k.pos[1], w: k.size[0], h: k.size[1], rot: f ? f.rot : k.rot[2] };
  });
}

/** Bounding box of drawn keys, rotation included. */
export function flatBounds(ks: FlatKey[]): { x0: number; x1: number; y0: number; y1: number } {
  let x0 = Infinity, x1 = -Infinity, y0 = Infinity, y1 = -Infinity;
  for (const k of ks) {
    const t = (k.rot * Math.PI) / 180, c = Math.abs(Math.cos(t)), s = Math.abs(Math.sin(t));
    const hw = (k.w * c + k.h * s) / 2, hh = (k.w * s + k.h * c) / 2, cx = k.x + k.w / 2, cy = k.y + k.h / 2;
    x0 = Math.min(x0, cx - hw); x1 = Math.max(x1, cx + hw); y0 = Math.min(y0, cy - hh); y1 = Math.max(y1, cy + hh);
  }
  return { x0, x1, y0, y1 };
}

/** Merge directional pairs (ab, ba) into one undirected pair; frequencies add. Most frequent first. */
export function undirected(pairs: Pair[]): Pair[] {
  const m = new Map<string, Pair>();
  for (const p of pairs) {
    const k = [...p.ngram].sort().join("");
    const q = m.get(k);
    if (q) q.freq += p.freq;
    else m.set(k, { ...p, ngram: k });
  }
  return [...m.values()].sort((a, b) => b.freq - a.freq);
}

/** First letter upper-cased. */
export const cap = (s: string) => s[0].toUpperCase() + s.slice(1);

export function fmt(v: number, digits = 2): string {
  if (!Number.isFinite(v)) return "–";
  if (Math.abs(v) >= 1000) return v.toFixed(0);
  if (Math.abs(v) >= 100) return v.toFixed(1);
  return v.toFixed(digits);
}

export function fmtTime(ms: number): string {
  return ms >= 60_000 ? `${Math.floor(ms / 60_000)}m ${Math.round((ms % 60_000) / 1000)}s` : `${(ms / 1000).toFixed(1)}s`;
}

export function fmtCount(n: number): string {
  if (n >= 1e9) return `${(n / 1e9).toFixed(2)}G`;
  if (n >= 1e6) return `${(n / 1e6).toFixed(1)}M`;
  if (n >= 1e3) return `${(n / 1e3).toFixed(0)}k`;
  return `${n}`;
}

/** Split slot-ordered chars into the geometry's rows (keys are listed row by row). */
export function rowsOf(chars: string, g: Geometry): string[] {
  const out: string[] = [];
  let cur = "";
  let last: number | null = null;
  [...chars].forEach((c, i) => {
    const row = g.key[i]?.row ?? 0;
    if (last !== null && row !== last) {
      out.push(cur);
      cur = "";
    }
    cur += c;
    last = row;
  });
  if (cur) out.push(cur);
  return out;
}

/** Layer-key glyphs `①②③` (gecko-core `layout::LAYER_KEYS`): the base key holding layer 1, 2, 3. */
export const LAYER_KEYS = ["\u2460", "\u2461", "\u2462"];
/** One-shot (`⓵⓶⓷`) and toggle (`❶❷❸`) layer keys (gecko-core `ONESHOT_KEYS`, `TOGGLE_KEYS`). */
export const ONESHOT_KEYS = ["\u24F5", "\u24F6", "\u24F7"];
export const TOGGLE_KEYS = ["\u2776", "\u2777", "\u2778"];
export type LayerMode = "hold" | "oneshot" | "toggle";
const MODE_KEYS: [LayerMode, string[]][] = [["hold", LAYER_KEYS], ["oneshot", ONESHOT_KEYS], ["toggle", TOGGLE_KEYS]];

/** The base key reaching layer `l` (1-based), its glyph and mode, or null. */
export function layerKey(base: string[], l: number): { key: number; glyph: string; mode: LayerMode } | null {
  for (const [mode, keys] of MODE_KEYS) {
    const key = base.indexOf(keys[l - 1]);
    if (key >= 0) return { key, glyph: keys[l - 1], mode };
  }
  return null;
}

/** Key `k` is on layer `l` reached through `lk` (gecko-core `Geometry::layered`): a hold layer
 *  skips the holding finger's keys, one-shot and toggle layers only the layer key. */
function onLayer(g: Geometry, lk: { key: number; mode: LayerMode }, k: number): boolean {
  return lk.mode === "hold" ? g.key[k].finger !== g.key[lk.key].finger : k !== lk.key;
}
const TOKEN_NAMES: Record<string, string> = {
  space: "\u2423", spc: "\u2423", enter: "\u23CE", ent: "\u23CE", ret: "\u23CE", bspc: "\u232B", backspace: "\u232B",
  tab: "\u21E5", esc: "\u238B", escape: "\u238B", none: "\u00B7", empty: "\u00B7", _: "\u00B7",
  shift: "\u21E7", sft: "\u21E7",
  left: "\u2190", down: "\u2193", up: "\u2191", right: "\u2192",
  layer1: LAYER_KEYS[0], l1: LAYER_KEYS[0], layer2: LAYER_KEYS[1], l2: LAYER_KEYS[1], layer3: LAYER_KEYS[2], l3: LAYER_KEYS[2],
  os1: ONESHOT_KEYS[0], oneshot1: ONESHOT_KEYS[0], os2: ONESHOT_KEYS[1], oneshot2: ONESHOT_KEYS[1], os3: ONESHOT_KEYS[2], oneshot3: ONESHOT_KEYS[2],
  tg1: TOGGLE_KEYS[0], toggle1: TOGGLE_KEYS[0], tg2: TOGGLE_KEYS[1], toggle2: TOGGLE_KEYS[1], tg3: TOGGLE_KEYS[2], toggle3: TOGGLE_KEYS[2],
};

/** A key's fill as a symbol (`Key.fill`: token name or one char). */
export function fillSymbol(fill: string | undefined): string | undefined {
  if (fill === undefined) return undefined;
  return [...fill].length === 1 ? fill : TOKEN_NAMES[fill.toLowerCase()];
}

/** `chars` on `g` (gecko-core `Layout::fit`): a layout listing only the keys without a fill
 *  (a 30-key layout on a thumb board) gets the fills; anything else is returned unchanged. */
export function fitChars(chars: string, g: Geometry): string {
  const a = [...chars];
  const open = g.key.filter((k) => k.fill === undefined).length;
  if (a.length === g.key.length || a.length !== open) return chars;
  let i = 0;
  return g.key.map((k) => fillSymbol(k.fill) ?? a[i++]).join("");
}

/**
 * Where each layer's keys sit in a flat layered string (gecko-core `Geometry::layered`):
 * `slot[layer][key]` is an index into `[...chars]`, or null where the layer has no key (its
 * layer key, and for a hold layer the holding finger). The base is keys 0..N; layer l follows.
 * A plain layout has one layer.
 */
export function layerSlots(g: Geometry, chars: string): (number | null)[][] {
  const n = g.key.length;
  const a = [...chars];
  const base = g.key.map((_, k) => k);
  if (a.length <= n) return [base];
  const out: (number | null)[][] = [base];
  let next = n;
  for (let l = 1; l <= LAYER_KEYS.length; l++) {
    const lk = layerKey(a.slice(0, n), l);
    if (!lk) break;
    out.push(g.key.map((_, k) => (onLayer(g, lk, k) ? next++ : null)));
  }
  return next === a.length ? out : [base];
}

/** Layout as editable text: base rows, then each layer's rows after a blank line (`·` on the
 *  keys the layer cannot use). */
export function layoutText(g: Geometry, chars: string): string {
  const a = [...chars];
  return layerSlots(g, chars).map((slots) => rowsOf(slots.map((s) => (s === null ? EMPTY : a[s])).join(""), g).join("\n")).join("\n\n");
}

/** Number of slots `chars` addresses on `g` (layer slots included). */
export function slotCount(g: Geometry, chars: string): number {
  const s = layerSlots(g, chars);
  return s.reduce((t, l) => t + l.filter((x) => x !== null).length, 0);
}

/** Swap the symbols on two slots. */
export function swapChars(chars: string, i: number, j: number): string {
  const a = [...chars];
  [a[i], a[j]] = [a[j], a[i]];
  return a.join("");
}

export type Verdict = "better" | "worse" | "near" | "same";

/** Compare a metric value with a reference value given its direction (5% relative band = near). */
export function verdict(dir: Dir, value: number, ref: number): Verdict {
  if (dir === "info") return "same";
  const diff = dir === "min" ? ref - value : value - ref;
  const scale = Math.max(Math.abs(ref), Math.abs(value), 1e-9);
  if (Math.abs(diff) / scale < 0.02) return "same";
  if (Math.abs(diff) / scale < 0.05) return "near";
  return diff > 0 ? "better" : "worse";
}

/** Metrics that carry weight in the profile, largest contribution first. */
export function scoredMetrics(ms: MetricValue[]): MetricValue[] {
  return ms.filter((m) => m.weight > 0 && m.dir !== "info").sort((a, b) => Math.abs(b.weighted) - Math.abs(a.weighted));
}

/** Special-key token glyphs (gecko-core `layout::TOKENS`) and the empty-slot glyph. */
export const TOKENS = "\u2423\u23CE\u232B\u21E5\u238B\u21E7\u2190\u2193\u2191\u2192" + LAYER_KEYS.join("") + ONESHOT_KEYS.join("") + TOGGLE_KEYS.join("");
export const EMPTY = "\u00B7";
/** Shift (gecko-core `layout::SHIFT`); like the empty slot it may repeat (one per hand). */
export const SHIFT = "\u21E7";

/** Validate a layout string against a geometry and allowed charset. Returns an error or null.
 *  Tokens are allowed whatever the charset; the empty-slot and Shift glyphs may repeat. */
export function checkLayout(chars: string, keys: number, charset: string): string | null {
  const a = [...chars];
  if (a.length !== keys) return `${a.length} symbols, the board has ${keys} keys`;
  const seen = new Set<string>();
  for (const c of a) {
    if (c === EMPTY || c === SHIFT) continue;
    if (seen.has(c)) return `'${c}' appears twice`;
    if (!charset.includes(c) && !TOKENS.includes(c)) return `'${c}' is not in the corpus charset`;
    seen.add(c);
  }
  return null;
}
