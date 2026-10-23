// Loads the gecko-wasm scorer (same Rust code as the CLI). Built by `npm run wasm`.

import init, { AtlasView, CaseBuild, Engine, corpus_from_text, corpus_stats, export_layout as exportLayoutRaw, flat2d as flat2dRaw, hand_costs, param_board, profile_hash } from "./wasm/pkg/gecko_wasm.js";
import type { Analysis, Flat, Geometry, ParamGeometry, PathResult, Profile } from "./api";
import type { HandSpec } from "./hand";

let ready: Promise<unknown> | null = null;

export class Scorer {
  private constructor(private engine: Engine) {}

  static async create(geometry: Geometry, corpus: Uint8Array): Promise<Scorer> {
    ready ??= init();
    await ready;
    // Extra fields such as `id` are ignored by serde.
    return new Scorer(new Engine(JSON.stringify(geometry), corpus));
  }

  analyze(chars: string, profile: Profile, top = 10): Analysis {
    return JSON.parse(this.engine.analyze(chars, JSON.stringify(profile), top)).analysis as Analysis;
  }

  /** Score change for swapping each pair of slots (`m[i * n + j]`). */
  swapMatrix(chars: string, profile: Profile): Float32Array {
    return this.engine.swap_matrix(chars, JSON.stringify(profile));
  }

  /** Path relinking from `from` to `to` (same symbols): the cheapest swap at each step. */
  path(from: string, to: string, profile: Profile): PathResult {
    return JSON.parse(this.engine.path(from, to, JSON.stringify(profile))) as PathResult;
  }

  free() {
    this.engine.free();
  }
}

/** Parse an explore atlas into WASM memory (see gecko-wasm `AtlasView`). */
export async function loadAtlasView(optima: Uint8Array, edges: Uint8Array, samples: Uint8Array): Promise<AtlasView> {
  ready ??= init();
  await ready;
  return new AtlasView(optima, edges, samples);
}

export type { AtlasView };

/** Load the WASM module (idempotent). */
export async function wasmReady(): Promise<void> {
  ready ??= init();
  await ready;
}

/** 2D drawing of a geometry (`Geometry::flat2d`). Needs `wasmReady()` first; throws otherwise. */
export function flat2d(g: Geometry): Flat[] {
  const { flat: _f, ...rest } = g;
  return JSON.parse(flat2dRaw(JSON.stringify(rest))) as Flat[];
}

/** The `profile_hash` a run with this profile records (`Profile::hash`). */
export async function profileHash(p: Profile): Promise<string> {
  await wasmReady();
  return profile_hash(JSON.stringify(p));
}

/** Another analyzer's 30-key format (gecko-wasm `export_layout`, same as `gecko layouts export`).
 *  Needs `wasmReady()` first; throws when the board has no 3 × 10 alpha block. */
export function exportLayout(g: Geometry, name: string, chars: string, format: "oxeylyzer" | "genkey" | "cmini"): string {
  const { flat: _f, ...rest } = g;
  return exportLayoutRaw(JSON.stringify(rest), name, chars, format);
}

/** Summary statistics of a `.gkc` corpus (gecko-wasm `corpus_stats`). Percentages per table. */
export interface CorpusStats {
  name: string; chars: number; charset: string;
  mono: [string, number][]; bigrams: [string, number][]; skipgrams: [string, number][]; trigrams: [string, number][];
}

export async function corpusStats(bytes: Uint8Array, top = 20): Promise<CorpusStats> {
  await wasmReady();
  return JSON.parse(corpus_stats(bytes, top)) as CorpusStats;
}

/** Build a `.gkc` corpus from text in the browser (`gecko corpus build` on one text). */
export async function corpusFromText(name: string, charset: string, text: string): Promise<Uint8Array> {
  await wasmReady();
  return corpus_from_text(name, charset, text);
}

/** Wrist angles in degrees: ulnar (+) / radial (−) deviation, extension (+) / flexion (−), pronation (90 = palm down). */
export interface Posture { deviation: number; extension: number; pronation: number }

/**
 * Hand-model costs per key (gecko-core `hand::costs_with`): G01 reach, G05 press angle, G03 wrist
 * posture, fingertip shortfall in mm; and each hand's resting posture (L, R).
 */
export interface HandCosts { reach: number[]; align: number[]; wrist: number[]; miss: number[]; placed: [boolean, boolean]; posture: [Posture, Posture] }

export async function handCosts(g: Geometry, spec: HandSpec): Promise<HandCosts> {
  await wasmReady();
  const { flat: _f, ...rest } = g;
  return JSON.parse(hand_costs(JSON.stringify(rest), JSON.stringify(spec))) as HandCosts;
}

/** A keycap collision: keys `a`, `b` and the overlap depth (u). */
export interface Collision { a: number; b: number; depth: number }

/**
 * A parametric board with free parameters `names` set to `values` (gecko-wasm `param_board`, the
 * same generator and collision test as the geometry search). The board gets `id`. Needs
 * `wasmReady()` first; throws on parameters the generator rejects.
 */
export function paramBoard(param: ParamGeometry, names: string[], values: number[], id: string): { geometry: Geometry; collisions: Collision[] } {
  const r = JSON.parse(param_board(JSON.stringify(param), JSON.stringify(names), Float64Array.from(values))) as { geometry: Geometry; flat: Flat[]; collisions: Collision[] };
  return { geometry: { ...r.geometry, id, flat: r.flat }, collisions: r.collisions };
}

/** Case dimensions in mm (gecko-core `CaseParams`); the UI overrides a few of them. */
export type Switch = "mx" | "choc";
export interface CaseParams {
  switch: Switch; hole: number; clip: number; clip_pocket: number; plate: number; rim: number; cap_gap: number; travel: number;
  switch_depth: number; clearance: number; wall: number; wall_offset: number; bottom: number; bridge: number;
}
/** gecko-core `CaseReport`: key indices are base keys. */
export interface CaseReport {
  keys: number; pieces: { keys: number[]; size: [number, number, number] }[]; webs: number; junctions: number; walls: number; corners: number;
  dropped: number; blocked: number[]; cap_hits: number[]; lift: number;
}
export type CasePart = "plate" | "walls" | "bottom";
/** A board's plate, walls and bottom (gecko-wasm `CaseBuild`, same as `gecko build stl`): triangle corners in mm
 *  (STL space: z up, y = −board y) and the STL files (`case` = plate + walls). */
export interface CaseModel {
  params: CaseParams; report: CaseReport;
  mesh: Record<CasePart, Float32Array>;
  stl: Record<"plate" | "case" | "bottom", Uint8Array>;
}

/** Needs `wasmReady()` first; throws on a board or parameters the generator rejects. */
export function buildCase(g: Geometry, sw: Switch, overrides: Partial<CaseParams>): CaseModel {
  const { flat: _f, ...rest } = g;
  const b = new CaseBuild(JSON.stringify(rest), sw, JSON.stringify(overrides));
  try {
    const { params, report } = JSON.parse(b.report()) as { params: CaseParams; report: CaseReport };
    return {
      params, report,
      mesh: { plate: b.positions("plate"), walls: b.positions("walls"), bottom: b.positions("bottom") },
      stl: { plate: b.stl("plate"), case: b.stl("case"), bottom: b.stl("bottom") },
    };
  } finally {
    b.free();
  }
}
