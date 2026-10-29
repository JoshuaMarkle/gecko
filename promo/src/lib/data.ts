// Typed access to generated/promo-data.json (scripts/export-data.mjs: real CLI, API and WASM output).

import raw from "../generated/promo-data.json";

/** `[x, y, z, rx, ry, rz]`: board-space position (u; x right, y toward the user, z up) and rotation (degrees). */
export type Pose = [number, number, number, number, number, number];
export interface BoardKey { pose: Pose; size: [number, number]; finger: string; row: number; col: number; home: boolean; stretch: boolean; effort: number; fill: string | null }
export interface Stats { score: number; sfb: number; rolls: number; alternate: number; pinky_ring: number; reach: number; effort: number }
export interface TraceStep { evals: number; ms: number; best: number; best_chars: string; chars: string; current: number; stats: Stats }
export interface BoardSnap { gen: number; best: number; collisions: number; poses: Pose[] }
export interface Range { name: string; lo: number; hi: number; start: number; value: number }
export interface StlFile { file: string; bytes: number }
export interface BuildFile { file: string; bytes: number; shells: number; size_mm: [number, number, number]; triangles: number }

/** Wrist angles (degrees): ulnar (+) / radial (−) deviation, extension (+) / flexion (−), pronation (90 = palm down). */
export interface Posture { deviation: number; extension: number; pronation: number }
/** gecko-core `hand::costs_with` for the average hand (WASM `hand_costs`): per-key costs, fingertip shortfall (mm), resting posture (L, R). */
export interface HandCosts { size: string; reach: number[]; align: number[]; wrist: number[]; miss: number[]; posture: [Posture, Posture] }

export interface PromoData {
  generated: string; geometry: string; corpus: string; profile: string;
  board: { name: string; description: string; keys: BoardKey[]; flat: [number, number, number][] };
  layout: {
    run: string; algo: string; seed: number; evals: number; start: string; final: string; pins: string;
    qwerty: Stats; best: Stats; snapshots: number; trace: TraceStep[]; workers: number[][]; temp: number[][];
  };
  boardOpt: { run: string; start: number; best: number; snapshots: BoardSnap[]; final: Pose[]; ranges: Range[]; convergence: [number, number][] };
  stl: { plate: StlFile; case: StlFile; bottom: StlFile; build: { files: BuildFile[] } & Record<string, unknown> };
  hand: HandCosts;
  perf: { full_per_sec: number; delta_per_sec: number; build_per_sec: number; rebuild_per_sec: number };
  language: {
    corpus: { chars: number; charset: string; description: string; classes_per_mille: { class: string; share: number }[] } & Record<string, unknown>;
    books: Record<string, unknown>;
    opening: string;
    /** Chapter I's first paragraphs (the document page). */
    chapter: string[];
  };
  analyze: { analysis: { bigrams: { freq: number; kind: string; ngram: string }[] } & Record<string, unknown> } & Record<string, unknown>;
  metrics: unknown;
  profileToml: string;
  engram: string[];
  search: {
    /** Explore atlas: `[x, y, score]` per local optimum (PCA "fingers" map), `best` row. */
    atlas: { run: string; points: [number, number, number][]; best: number };
    /** MAP-Elites: best score per cell, `grid[yBin][xBin]` (null = empty). */
    qd: { run: string; axes: { name: string; lo: number; hi: number; bins: number }[]; grid: (number | null)[][] };
    /** NSGA-II front: `[rank, ...objective values]`. */
    pareto: { objectives: string[]; front: number[][] };
  };
}

export const data = raw as unknown as PromoData;

/** Letter bigram matrix of the corpus (`analyze` bigrams, a–z × a–z, % of bigrams). */
export function bigramMatrix(): { labels: string[]; m: number[][] } {
  const labels = [..."abcdefghijklmnopqrstuvwxyz"];
  const m = labels.map(() => labels.map(() => 0));
  for (const b of data.analyze.analysis.bigrams) {
    const [x, y] = [...b.ngram];
    const i = labels.indexOf(x), j = labels.indexOf(y);
    if (i >= 0 && j >= 0) m[i][j] += b.freq;
  }
  return { labels, m };
}

/** A layout string as one symbol per key. */
export const symbols = (chars: string): string[] => [...chars];

/** Legends for GECKO's special symbols; `icon` names a drawn glyph (three/legend.ts `ICONS`). */
export const SPECIAL: Record<string, { label: string; icon?: string }> = {
  "⇥": { label: "Tab", icon: "tab" },
  "⌫": { label: "Bksp", icon: "bksp" },
  "⎈": { label: "Ctrl" },
  "⇧": { label: "Shift", icon: "shift" },
  "⎋": { label: "Esc" },
  "␣": { label: "Space", icon: "space" },
  "⏎": { label: "Enter", icon: "enter" },
  "①": { label: "Lower", icon: "lower" },
  "②": { label: "Raise", icon: "raise" },
  "·": { label: "", icon: "blank" },
};

/** Display legend of a symbol: letters upper-case, specials by label. */
export const legendOf = (ch: string) => SPECIAL[ch]?.label ?? ch.toUpperCase();

/** Format helpers (tabular, product style). */
export const fmt = {
  n: (x: number, d = 2) => x.toFixed(d),
  pct: (x: number, d = 1) => `${x.toFixed(d)}%`,
  big: (x: number) => x >= 1e6 ? `${(x / 1e6).toFixed(x >= 1e7 ? 0 : 1)}M` : x >= 1e3 ? `${Math.round(x / 1e3)}k` : `${Math.round(x)}`,
  int: (x: number) => Math.round(x).toLocaleString("en-US"),
};
