// Small pure helpers for display and data shaping (unit-tested in format.test.ts).

import type { Dir, Geometry, MetricValue, Pair } from "./api";

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
const TOKEN_NAMES: Record<string, string> = {
  space: "\u2423", spc: "\u2423", enter: "\u23CE", ent: "\u23CE", ret: "\u23CE", bspc: "\u232B", backspace: "\u232B",
  tab: "\u21E5", esc: "\u238B", escape: "\u238B",
  left: "\u2190", down: "\u2193", up: "\u2191", right: "\u2192", none: "\u00B7", empty: "\u00B7", _: "\u00B7",
  layer1: LAYER_KEYS[0], l1: LAYER_KEYS[0], layer2: LAYER_KEYS[1], l2: LAYER_KEYS[1], layer3: LAYER_KEYS[2], l3: LAYER_KEYS[2],
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
 * `slot[layer][key]` is an index into `[...chars]`, or null on the finger that holds that
 * layer's key. The base is keys 0..N; layer l follows, skipping the holding finger's keys.
 * A plain layout has one layer.
 */
export function layerSlots(g: Geometry, chars: string): (number | null)[][] {
  const n = g.key.length;
  const a = [...chars];
  const base = g.key.map((_, k) => k);
  if (a.length <= n) return [base];
  const out: (number | null)[][] = [base];
  let next = n;
  for (const t of LAYER_KEYS) {
    const h = a.slice(0, n).indexOf(t);
    if (h < 0) break;
    out.push(g.key.map((k) => (k.finger === g.key[h].finger ? null : next++)));
  }
  return next === a.length ? out : [base];
}

/** Layout as editable text: base rows, then each layer's rows after a blank line (`·` on the
 *  keys of the finger that holds the layer). */
export function layoutText(g: Geometry, chars: string): string {
  const a = [...chars];
  return layerSlots(g, chars).map((slots) => rowsOf(slots.map((s) => (s === null ? EMPTY : a[s])).join(""), g).join("\n")).join("\n\n");
}

/** Inverse of `layoutText`: one block is a plain layout (whitespace ignored); several blocks
 *  are the base and its layers, one symbol per key each, flattened like `layerSlots`. */
export function parseLayoutText(g: Geometry, text: string): { chars: string; error: string | null } {
  const blocks = text.split(/\n\s*\n/).map((b) => [...b.replace(/\s/g, "")]).filter((b) => b.length);
  if (blocks.length <= 1) return { chars: (blocks[0] ?? []).join(""), error: null };
  const n = g.key.length;
  const bad = blocks.findIndex((b) => b.length !== n);
  if (bad >= 0) return { chars: "", error: `${bad === 0 ? "base" : `layer ${bad}`} has ${blocks[bad].length} symbols, the board has ${n} keys` };
  let flat = blocks[0].join("");
  for (let l = 1; l < blocks.length; l++) {
    const h = blocks[0].indexOf(LAYER_KEYS[l - 1]);
    if (h < 0) return { chars: "", error: `layer ${l} needs a ${LAYER_KEYS[l - 1]} key on the base` };
    for (let k = 0; k < n; k++) {
      if (g.key[k].finger !== g.key[h].finger) flat += blocks[l][k];
      else if (blocks[l][k] !== EMPTY) return { chars: "", error: `layer ${l}: '${blocks[l][k]}' is on the finger that holds the layer` };
    }
  }
  return { chars: flat, error: null };
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
export const TOKENS = "\u2423\u23CE\u232B\u21E5\u238B\u2190\u2193\u2191\u2192\u2460\u2461\u2462";
export const EMPTY = "\u00B7";

/** Validate a layout string against a geometry and allowed charset. Returns an error or null.
 *  Tokens are allowed whatever the charset; the empty-slot glyph may repeat. */
export function checkLayout(chars: string, keys: number, charset: string): string | null {
  const a = [...chars];
  if (a.length !== keys) return `${a.length} symbols, the board has ${keys} keys`;
  const seen = new Set<string>();
  for (const c of a) {
    if (c === EMPTY) continue;
    if (seen.has(c)) return `'${c}' appears twice`;
    if (!charset.includes(c) && !TOKENS.includes(c)) return `'${c}' is not in the corpus charset`;
    seen.add(c);
  }
  return null;
}
