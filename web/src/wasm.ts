// Loads the gecko-wasm scorer (same Rust code as the CLI). Built by `npm run wasm`.

import init, { Engine } from "./wasm/pkg/gecko_wasm.js";
import type { Analysis, Geometry, Profile } from "./api";

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

  free() {
    this.engine.free();
  }
}
