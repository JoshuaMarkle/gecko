// Loads the gecko-wasm scorer (same Rust code as the CLI). Built by `npm run wasm`.

import init, { AtlasView, Engine, corpus_from_text, corpus_stats, export_layout as exportLayoutRaw, flat2d as flat2dRaw, hand_costs } from "./wasm/pkg/gecko_wasm.js";
import type { Analysis, Flat, Geometry, Profile } from "./api";
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
