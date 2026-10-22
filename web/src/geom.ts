// Board + layout search (geometry runs, gecko-core `geomsearch`): the free-parameter groups the
// Search tab offers, the `--free` spec sent to the server, readable parameter names, and progress
// in the shape the convergence chart reads.

import type { Breakdown, GeomProgress, GeomVar, Progress } from "./api";

/** Free-parameter groups (`gecko geom optimize --free` terms): [term, label, help, 3D only]. */
export const GEOM_GROUPS: [string, string, string, boolean][] = [
  ["stagger", "Stagger", "Vertical offset of each finger column.", false],
  ["splay", "Splay", "Rotation of each finger column.", false],
  ["spread", "Spread", "Horizontal gap between finger columns (never below 1u).", false],
  ["thumb", "Thumb cluster", "Thumb cluster position, rotation and key offsets.", false],
  ["row_curve", "Row curvature", "Keys of a column curve toward the palm (a key well).", true],
  ["col_curve", "Column curvature", "Columns curve around the hand from side to side.", true],
  ["lift", "Column height", "Each finger column raised or lowered.", true],
  ["tent", "Tenting", "Each half rotated up at its inner edge.", true],
  ["tilt", "Tilt", "Each half tilted toward (−) or away from (+) the user.", true],
  ["thumb3d", "Thumb 3D", "Thumb cluster tent, tilt, height and curvature.", true],
];

/** Groups of the CLI presets `2d` and `3d`. */
export const PRESET_2D = GEOM_GROUPS.filter((g) => !g[3]).map((g) => g[0]);
export const PRESET_3D = GEOM_GROUPS.map((g) => g[0]);

/** Search-tab settings of a board + layout search (persisted with the other search settings). */
export interface GeomSettings {
  groups: string[];
  /** `<board id>|<parameter>` → [lo, hi], replacing the default bounds. */
  bounds: Record<string, [number, number]>;
  mode: "bilevel" | "alternate";
  /** Layout swaps per board candidate (bilevel). */
  innerEvals: number;
  /** Keycap clearance per side, u. */
  margin: number;
  /** Collision penalty per u of overlap, in units of |start score| + 1. */
  penalty: number;
  keepEffort: boolean;
  /** Board candidates (with an evaluation budget). */
  evals: number;
}

export const GEOM_DEFAULTS: GeomSettings = {
  groups: PRESET_2D, bounds: {}, mode: "bilevel", innerEvals: 20_000, margin: 0, penalty: 10, keepEffort: false, evals: 400,
};

/** The groups part of the spec (no bounds): what `/api/geometries/{id}/free` expands. */
export function groupSpec(groups: string[]): string {
  return GEOM_GROUPS.map((g) => g[0]).filter((g) => groups.includes(g)).join(",");
}

/** Bound overrides for `board` that differ from the defaults in `vars`. */
export function boundsFor(s: GeomSettings, board: string, vars: GeomVar[]): Map<string, [number, number]> {
  const out = new Map<string, [number, number]>();
  for (const v of vars) {
    const b = s.bounds[`${board}|${v.name}`];
    if (b && b[0] < b[1] && (b[0] !== v.lo || b[1] !== v.hi)) out.set(v.name, b);
  }
  return out;
}

/** The `--free` spec for a run: the groups, then `name=lo:hi` for every edited bound. */
export function freeSpec(s: GeomSettings, board: string, vars: GeomVar[]): string {
  const terms = [groupSpec(s.groups)];
  for (const [name, [lo, hi]] of boundsFor(s, board, vars)) terms.push(`${name}=${num(lo)}:${num(hi)}`);
  return terms.filter(Boolean).join(",");
}

const num = (x: number) => String(Math.round(x * 1e6) / 1e6);

/** Variables with the edited bounds applied (what the run will search). */
export function withBounds(s: GeomSettings, board: string, vars: GeomVar[]): GeomVar[] {
  const b = boundsFor(s, board, vars);
  return vars.map((v) => { const x = b.get(v.name); return x ? { ...v, lo: x[0], hi: x[1] } : v; });
}

const FIELDS: Record<string, [string, string]> = {
  stagger: ["stagger", "u"], splay: ["splay", "°"], spread: ["spread", "u"], lift: ["height", "u"],
  anchor_x: ["position x", "u"], anchor_y: ["position y", "u"], rotate: ["rotation", "°"], row_pitch: ["row pitch", "u"],
  arc_radius: ["arc radius", "u"], row_curve: ["row curvature", "°"], col_curve: ["column curvature", "°"], center_col: ["curve centre", "col"],
  tent: ["tent", "°"], tilt: ["tilt", "°"], angle: ["angle", "°"],
};

/** Readable name and unit of a parameter: `fingers.2.stagger` → ["fingers col 3 stagger", "u"]. */
export function varLabel(name: string): [string, string] {
  const parts = name.split(".");
  const field = parts[parts.length - 1];
  const [word, unit] = FIELDS[field] ?? [field, ""];
  if (parts[0] === "split") return [`split ${word}`, unit];
  if (parts.length === 3) return [`${parts[0]} col ${Number(parts[1]) + 1} ${word}`, unit];
  return [`${parts[0]} ${word}`, unit];
}

/** Geometry-run progress as the convergence chart reads it (boards counted as evaluations). */
export function asProgress(p: GeomProgress): Progress {
  return { elapsed_ms: p.elapsed_ms, evals: p.candidates, best: p.best, best_chars: p.chars };
}

/** Weighted metrics that differ between two breakdowns, largest change of contribution first. */
export function metricChanges(start: Breakdown, best: Breakdown): { id: string; name: string; unit: string; start: number; best: number; delta: number }[] {
  const rows = [];
  for (const m of best.metrics) {
    const s = start.metrics.find((x) => x.id === m.id);
    if (!s || (m.weight === 0 && s.weight === 0)) continue;
    rows.push({ id: m.id, name: m.name, unit: m.unit, start: s.value, best: m.value, delta: m.weighted - s.weighted });
  }
  return rows.filter((r) => Math.abs(r.delta) > 1e-9).sort((a, b) => Math.abs(b.delta) - Math.abs(a.delta));
}
