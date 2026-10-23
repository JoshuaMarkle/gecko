// Export tab (context/web.md "Export"): the working model as files. Pure string builders,
// unit-tested in export.test.ts; the view only adds downloads, the clipboard and PNG rasterizing.

import type { Analysis, Geometry } from "./api";
import { fingerColor, heatColor, heatText, INK, MUTED } from "./colors";
import { EMPTY, flatBounds, flatKeys, fmt, layerKey, layerSlots, LAYER_KEYS, ONESHOT_KEYS, rowsOf, scoredMetrics, SHIFT, TOGGLE_KEYS } from "./format";

/** Everything an export describes: the layout on its board, plus stats when it was scored. */
export interface ExportInfo {
  name: string;
  geometry: Geometry;
  /** Flat layered string (base keys, then each layer's slots; `layerSlots`). */
  chars: string;
  a: Analysis | null;
  corpus?: string;
  profile?: string;
  ref?: { name: string; a: Analysis };
  run?: { id: string; algo: string; holdout?: { corpus: string; score?: number; retained: number } };
}

export type KeyColor = "plain" | "fingers" | "heat";
export interface ImageOptions { color: KeyColor; layers: boolean; title: boolean }

/** Token glyph → gecko-core token name (`layout::TOKENS`, first name), written `{name}` in layout files. */
const TOKEN_FILE: Record<string, string> = {
  "␣": "space", "⏎": "enter", "⌫": "bspc", "⇥": "tab", "⎋": "esc",
  "←": "left", "↓": "down", "↑": "up", "→": "right", [SHIFT]: "shift",
  ...Object.fromEntries(LAYER_KEYS.map((k, i) => [k, `layer${i + 1}`])),
  ...Object.fromEntries(ONESHOT_KEYS.map((k, i) => [k, `os${i + 1}`])),
  ...Object.fromEntries(TOGGLE_KEYS.map((k, i) => [k, `tg${i + 1}`])),
};

/** Token glyph → keycap legend for KLE (a word reads better than a glyph off GECKO). */
const TOKEN_LEGEND: Record<string, string> = {
  "␣": "Space", "⏎": "Enter", "⌫": "Bksp", "⇥": "Tab", "⎋": "Esc",
  "←": "←", "↓": "↓", "↑": "↑", "→": "→", [SHIFT]: "Shift", [EMPTY]: "",
  ...Object.fromEntries(LAYER_KEYS.map((k, i) => [k, `L${i + 1}`])),
  ...Object.fromEntries(ONESHOT_KEYS.map((k, i) => [k, `OS${i + 1}`])),
  ...Object.fromEntries(TOGGLE_KEYS.map((k, i) => [k, `TG${i + 1}`])),
};

/** Symbols per layer, one per key (`·` where the layer has no key). */
export function layerChars(g: Geometry, chars: string): string[][] {
  const a = [...chars];
  return layerSlots(g, chars).map((slots) => slots.map((s) => (s === null ? EMPTY : a[s] ?? EMPTY)));
}

/** File name from a layout name: `Colemak DH (edited)` → `colemak-dh-edited`. */
export function slug(name: string): string {
  return name.toLowerCase().normalize("NFKD").replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "") || "layout";
}

const tomlStr = (s: string) => `"${s.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"`;
const fileSym = (c: string) => (TOKEN_FILE[c] ? `{${TOKEN_FILE[c]}}` : c);

/** A GECKO layout file (`data/layouts/*.toml`, gecko-core `Layout::from_toml`): base rows,
 *  then one `[[layer]]` per layer. Tokens are written `{name}`. */
export function layoutToml(e: ExportInfo): string {
  const g = e.geometry;
  const rows = (keys: string[]) => `[${splitRows(g, keys.map(fileSym)).map((r) => tomlStr(r.join(""))).join(", ")}]`;
  const layers = layerChars(g, e.chars);
  const score = e.a ? ` · score ${fmt(e.a.breakdown.score, 3)}` : "";
  const ctx = [e.corpus, e.profile].filter(Boolean).join(" / ");
  let out = `name = ${tomlStr(e.name)}\ngeometry = ${tomlStr(g.id)}\nsource = ${tomlStr(`GECKO export${ctx ? ` · ${ctx}` : ""}${score}`)}\nrows = ${rows(layers[0])}\n`;
  layers.slice(1).forEach((l, i) => { out += `\n[[layer]]\nname = "layer${i + 1}"\nrows = ${rows(l)}\n`; });
  return out;
}

/** Per-key items split into the geometry's rows (like `rowsOf`, for multi-char items). */
function splitRows<T>(g: Geometry, items: T[]): T[][] {
  const out: T[][] = [];
  items.forEach((x, i) => {
    if (i === 0 || g.key[i]?.row !== g.key[i - 1]?.row) out.push([]);
    out[out.length - 1].push(x);
  });
  return out;
}

/** Plain text: base rows, then each layer's rows after a blank line (`layoutText`). */
export function layoutTextExport(e: ExportInfo): string {
  return layerChars(e.geometry, e.chars).map((l) => rowsOf(l.join(""), e.geometry).join("\n")).join("\n\n") + "\n";
}

/** Layout and stats as JSON (schema 1). */
export function layoutJson(e: ExportInfo): string {
  const g = e.geometry;
  const layers = layerChars(g, e.chars).map((l) => rowsOf(l.join(""), g));
  const a = e.a;
  return JSON.stringify({
    schema: 1,
    name: e.name,
    geometry: g.id,
    chars: e.chars,
    rows: layers[0],
    layers: layers.slice(1),
    corpus: e.corpus ?? null,
    profile: e.profile ?? null,
    score: a ? a.breakdown.score : null,
    reference: e.ref ? { name: e.ref.name, score: e.ref.a.breakdown.score } : null,
    metrics: a ? a.breakdown.metrics.map(({ id, name, unit, dir, value, weight, weighted }) => ({ id, name, unit, dir, value, weight, weighted })) : [],
    finger_load: a ? Object.fromEntries(a.fingers.map((f, i) => [f, a.finger_load[i]])) : null,
    run: e.run ?? null,
  }, null, 2) + "\n";
}

const r4 = (v: number) => Math.round(v * 1e4) / 1e4;

/** keyboard-layout-editor.com JSON (gecko-core `kle::export` with legends) of the board's 2D
 *  drawing (`flatKeys`, so curved boards come out flattened without overlaps): each key placed
 *  by its rotation origin so rotations survive; base legend first, each layer's symbol after it.
 *  Colors follow the image's key color (heat uses the base layer). */
export function kleJson(e: ExportInfo, color: KeyColor = "plain"): string {
  const g = e.geometry;
  const layers = layerChars(g, e.chars);
  const heat = heatOf(e);
  const rows: unknown[] = [{ name: e.name, notes: `${g.name} · exported from GECKO` }];
  flatKeys(g).forEach((k, i) => {
    const props: Record<string, number | string> = { r: r4(k.rot), rx: r4(k.x + k.w / 2), ry: r4(k.y + k.h / 2), x: r4(-k.w / 2), y: r4(-k.h / 2), w: r4(k.w), h: r4(k.h) };
    if (color === "fingers") props.c = fingerColor(g.key[i].finger);
    if (color === "heat") { props.c = heatColor(heat.t(0, i)); props.t = heatText(heat.t(0, i)); }
    const legends = layers.map((l) => TOKEN_LEGEND[l[i]] ?? l[i]);
    while (legends.length > 1 && legends[legends.length - 1] === "") legends.pop();
    rows.push([props, legends.join("\n")]);
  });
  return `[\n${rows.map((r) => JSON.stringify(r)).join(",\n")}\n]\n`;
}

const UNIT_MM = 19.05;
const mm = (u: number) => Math.round(u * UNIT_MM * 1e3) / 1e3;

/** Ergogen config (docs.ergogen.xyz, v4): the board's 2D drawing (`flatKeys`, as KLE) as one
 *  single-key zone per key, anchored at the key's center in mm (1u = 19.05 mm; ergogen's y
 *  points up and its rotation is counter-clockwise, so both flip) with keycap `width`/`height`,
 *  plus a `plate` outline of 14 mm switch cutouts. Each key carries its base legend, layer
 *  legends, finger, row and column as custom key attributes. Each zone is one line of JSON
 *  flow mapping (valid YAML). */
export function ergogenYaml(e: ExportInfo): string {
  const g = e.geometry;
  const layers = layerChars(g, e.chars);
  const zones = flatKeys(g).map((k, i) => {
    const gk = g.key[i];
    const legends = layers.map((l) => TOKEN_LEGEND[l[i]] ?? l[i]);
    const key: Record<string, unknown> = { name: `k${i}`, width: mm(k.w), height: mm(k.h), label: legends[0], finger: gk.finger, gecko_row: gk.row, gecko_col: gk.col };
    if (legends.length > 1) key.layers = legends.slice(1);
    const zone = { anchor: { shift: [mm(k.x + k.w / 2), mm(-(k.y + k.h / 2))], rotate: r4(-k.rot) || 0 }, key, columns: { c: {} }, rows: { r: {} } };
    return `    k${i}: ${JSON.stringify(zone)}`;
  });
  return `# ${e.name} on ${g.name}, exported from GECKO. Load at ergogen.xyz or with the ergogen CLI.\n`
    + `# One zone per key at its center (mm, y up, rotation counter-clockwise); 1u = 19.05 mm.\n`
    + `meta:\n  name: ${JSON.stringify(e.name)}\n`
    + `points:\n  zones:\n${zones.join("\n")}\n`
    + `outlines:\n  plate:\n    - what: rectangle\n      where: true\n      size: 14\n`;
}

const csvCell = (v: string | number) => (typeof v === "number" ? `${v}` : /[",\n]/.test(v) ? `"${v.replace(/"/g, '""')}"` : v);

/** Every metric as CSV: value, weight and weighted contribution (and the reference's value). */
export function statsCsv(e: ExportInfo): string {
  if (!e.a) return "";
  const ref = e.ref;
  const head = ["id", "name", "unit", "direction", "value", "weight", "weighted", ...(ref ? [`${ref.name} value`] : [])];
  const rows = e.a.breakdown.metrics.map((m) => [m.id, m.name, m.unit, m.dir, m.value, m.weight, m.weighted,
    ...(ref ? [ref.a.breakdown.metrics.find((x) => x.id === m.id)?.value ?? ""] : [])]);
  rows.push(["score", "Total score", "", "min", e.a.breakdown.score, "", "", ...(ref ? [ref.a.breakdown.score] : [])]);
  return [head, ...rows].map((r) => r.map(csvCell).join(",")).join("\n") + "\n";
}

/** A shareable Markdown summary: context line, the layout in a code block, weighted metrics. */
export function statsMarkdown(e: ExportInfo): string {
  const a = e.a, ref = e.ref;
  const ctx = [e.geometry.name, e.corpus, e.profile && `profile ${e.profile}`].filter(Boolean).join(" · ");
  let out = `## ${e.name}\n\n${ctx}\n\n\`\`\`\n${layoutTextExport(e)}\`\`\`\n`;
  if (!a) return out;
  const delta = ref ? a.breakdown.score - ref.a.breakdown.score : null;
  out += `\nScore **${fmt(a.breakdown.score, 3)}** (lower is better)${delta !== null ? `, ${delta <= 0 ? "" : "+"}${fmt(delta, 3)} vs ${ref!.name}` : ""}.\n`;
  if (e.run?.holdout) out += `On held-out ${e.run.holdout.corpus} it keeps ${fmt(100 * e.run.holdout.retained, 1)}% of the gain.\n`;
  const ms = scoredMetrics(a.breakdown.metrics);
  if (ms.length) {
    out += `\n| Metric | Value |${ref ? ` ${ref.name} |` : ""}\n|---|---:|${ref ? "---:|" : ""}\n`;
    for (const m of ms) {
      const u = m.unit === "%" ? "%" : "";
      const r = ref?.a.breakdown.metrics.find((x) => x.id === m.id);
      out += `| ${m.name} | ${fmt(m.value)}${u} |${ref ? ` ${r ? `${fmt(r.value)}${u}` : ""} |` : ""}\n`;
    }
  }
  return out + "\n_Made with GECKO._\n";
}

/** Heat per (layer, key) as t in [0, 1] of the busiest slot. */
function heatOf(e: ExportInfo): { t: (layer: number, key: number) => number } {
  const slots = layerSlots(e.geometry, e.chars);
  const heat = e.a?.heat ?? [];
  const max = Math.max(...heat, 1e-9);
  return { t: (l, k) => { const s = slots[l]?.[k]; return s === null || s === undefined ? 0 : (heat[s] ?? 0) / max; } };
}

const esc = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
const FONT = "Inter, -apple-system, 'Helvetica Neue', Arial, sans-serif";
const LAYER_MODE: Record<string, string> = { hold: "hold", oneshot: "one-shot", toggle: "toggle" };

/** The layout on its physical board as a standalone SVG (the Export preview and the SVG/PNG
 *  download): one board per layer when `layers`, a title and score line when `title`.
 *  Ported to gecko-core `interop::svg` (`gecko layouts svg`): keep sizes and colors in sync. */
export function boardSvg(e: ExportInfo, o: ImageOptions): string {
  const g = e.geometry;
  const U = 56, PAD = 28, GAP = 36, CAP = 22;
  const ks = flatKeys(g);
  const b = flatBounds(ks);
  const bw = (b.x1 - b.x0) * U, bh = (b.y1 - b.y0) * U;
  const all = layerChars(g, e.chars);
  const shown = o.layers ? all : all.slice(0, 1);
  const base = all[0];
  const heat = heatOf(e);
  const captions = shown.length > 1;
  let y = PAD;
  let body = "";
  if (o.title) {
    const sub = [g.name, e.corpus, e.profile].filter(Boolean).join(" · ");
    const score = e.a ? `score ${fmt(e.a.breakdown.score, 3)}${e.ref ? ` · ${e.ref.name} ${fmt(e.ref.a.breakdown.score, 3)}` : ""}` : "";
    body += `<text x="${PAD}" y="${y + 18}" font-size="20" font-weight="650" fill="${INK}">${esc(e.name)}</text>`;
    body += `<text x="${PAD}" y="${y + 40}" font-size="13" fill="${MUTED}">${esc([sub, score].filter(Boolean).join(" · "))}</text>`;
    y += 64;
  }
  // Layers in a grid, two per row, so a layered layout stays close to the board's aspect.
  const cols = shown.length > 1 ? 2 : 1;
  const cell = bh + (captions ? CAP : 0);
  shown.forEach((keys, l) => {
    const ox = PAD + (l % cols) * (bw + GAP);
    let oy = y + Math.floor(l / cols) * (cell + GAP);
    if (captions) {
      const lk = l ? layerKey(base, l) : null;
      const cap = l === 0 ? "Base" : `Layer ${l}${lk ? ` · ${LAYER_MODE[lk.mode]} ${TOKEN_LEGEND[lk.glyph]}` : ""}`;
      body += `<text x="${r4(ox)}" y="${r4(oy + 14)}" font-size="13" font-weight="600" fill="${INK}">${esc(cap)}</text>`;
      oy += CAP;
    }
    ks.forEach((k, i) => {
      const sym = keys[i];
      const off = sym === EMPTY;
      const t = heat.t(l, i);
      const fill = off ? "#f3f3f3" : o.color === "fingers" ? fingerColor(g.key[i].finger) : o.color === "heat" ? heatColor(t) : "#ffffff";
      const ink = o.color === "heat" && !off ? heatText(t) : INK;
      const x = ox + (k.x - b.x0) * U, ky = oy + (k.y - b.y0) * U;
      const w = k.w * U - 4, h = k.h * U - 4, cx = x + 2 + w / 2, cy = ky + 2 + h / 2;
      const legend = TOKEN_LEGEND[sym] ?? sym;
      const size = [...legend].length > 2 ? 11 : 18;
      body += `<g${k.rot ? ` transform="rotate(${r4(k.rot)} ${r4(cx)} ${r4(cy)})"` : ""}>`
        + `<rect x="${r4(x + 2)}" y="${r4(ky + 2)}" width="${r4(w)}" height="${r4(h)}" rx="6" fill="${fill}" stroke="${off ? "#e0e0e0" : "#c6c6c6"}"/>`
        + (legend ? `<text x="${r4(cx)}" y="${r4(cy + size * 0.36)}" text-anchor="middle" font-size="${size}" font-weight="600" fill="${ink}">${esc(legend)}</text>` : "")
        + `</g>`;
    });
  });
  const rows = Math.ceil(shown.length / cols);
  y += rows * cell + (rows - 1) * GAP;
  const W = Math.ceil(Math.max(cols * bw + (cols - 1) * GAP + 2 * PAD, o.title ? 420 : 0)), H = Math.ceil(y + PAD);
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${W}" height="${H}" viewBox="0 0 ${W} ${H}" font-family="${FONT}">`
    + `<rect width="${W}" height="${H}" fill="#ffffff"/>${body}</svg>`;
}
