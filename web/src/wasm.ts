// Loads the gecko-wasm scorer (same Rust code as the CLI). Built by `npm run wasm`.

import init, { AtlasView, Engine, corpus_from_text, corpus_stats, flat2d as flat2dRaw } from "./wasm/pkg/gecko_wasm.js";
import type { Analysis, Flat, Geometry, Profile } from "./api";

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
