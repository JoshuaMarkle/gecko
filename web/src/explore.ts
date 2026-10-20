// Pure helpers for the Explore view (unit-tested in explore.test.ts). Heavy statistics run in
// gecko-wasm (`AtlasView`); these shape its arrays for drawing.

import type { Profile } from "./api";

/** Rows sorted by score (best first), optionally filtered. */
export function rankOrder(scores: ArrayLike<number>, keep?: (i: number) => boolean): Uint32Array {
  const idx: number[] = [];
  for (let i = 0; i < scores.length; i++) if (!keep || keep(i)) idx.push(i);
  idx.sort((a, b) => scores[a] - scores[b] || a - b);
  return Uint32Array.from(idx);
}

/** Non-dominated rows for two objectives (`xMin`/`yMin`: lower is better), sorted by x. */
export function paretoFront(xs: ArrayLike<number>, ys: ArrayLike<number>, rows: ArrayLike<number>, xMin: boolean, yMin: boolean): number[] {
  const sx = xMin ? 1 : -1, sy = yMin ? 1 : -1;
  const r = Array.from(rows).sort((a, b) => sx * (xs[a] - xs[b]) || sy * (ys[a] - ys[b]));
  const out: number[] = [];
  let bestY = Infinity;
  for (const i of r) {
    const y = sy * ys[i];
    if (y < bestY) { out.push(i); bestY = y; }
  }
  return out;
}

export function histogram(values: ArrayLike<number>, lo: number, hi: number, bins: number): number[] {
  const out = new Array(bins).fill(0);
  const w = (hi - lo) / bins;
  if (!(w > 0)) return out;
  for (let i = 0; i < values.length; i++) {
    const b = Math.floor((values[i] - lo) / w);
    if (b >= 0 && b < bins) out[b]++;
    else if (values[i] === hi) out[bins - 1]++;
  }
  return out;
}

/** Contiguous range of weight factors around 1 where `best` stays the winner, and what replaces
 *  it just outside (`below`/`above` rows, or null if it wins to the end of the grid). */
export function stabilityRange(winners: ArrayLike<number>, factors: number[], best: number) {
  const one = factors.reduce((b, f, i) => (Math.abs(Math.log(f)) < Math.abs(Math.log(factors[b])) ? i : b), 0);
  if (winners[one] !== best) return { lo: 1, hi: 1, below: winners[one], above: winners[one] };
  let a = one, b = one;
  while (a > 0 && winners[a - 1] === best) a--;
  while (b < factors.length - 1 && winners[b + 1] === best) b++;
  return { lo: factors[a], hi: factors[b], below: a > 0 ? winners[a - 1] : null, above: b < factors.length - 1 ? winners[b + 1] : null };
}

/** Log-spaced factors from lo to hi (inclusive), with 1 exactly on the grid when lo < 1 < hi. */
export function logGrid(lo: number, hi: number, n: number): number[] {
  const out: number[] = [];
  for (let i = 0; i < n; i++) out.push(Math.exp(Math.log(lo) + ((Math.log(hi) - Math.log(lo)) * i) / (n - 1)));
  const k = out.reduce((b, f, i) => (Math.abs(Math.log(f)) < Math.abs(Math.log(out[b])) ? i : b), 0);
  out[k] = 1;
  return out;
}

/** Symbol ids of `chars` in an atlas alphabet, or null if the symbol sets differ. */
export function symIds(chars: string, alphabet: string): Uint8Array | null {
  const a = [...alphabet], c = [...chars];
  if (a.length !== c.length) return null;
  const out = new Uint8Array(c.length);
  for (let i = 0; i < c.length; i++) {
    const k = a.indexOf(c[i]);
    if (k < 0) return null;
    out[i] = k;
  }
  return out;
}

/** Slot-ordered chars of row `i` of a flat `syms` array. */
export function rowChars(syms: Uint8Array, slots: number, i: number, alphabet: string[]): string {
  let s = "";
  for (let k = 0; k < slots; k++) s += alphabet[syms[i * slots + k]];
  return s;
}

/** Profiles share metric tables (re-scoring raw vectors is exact) when everything but the weights matches. */
export function sameTables(a: Profile, b: Profile): boolean {
  const key = (p: Profile) => JSON.stringify([p.finger_target, p.trigram_coverage, p.sfb ?? null]);
  return key(a) === key(b);
}

export interface Swap { i: number; j: number; delta: number }

/** All slot-pair swaps from a swap-delta matrix, cheapest first. */
export function swapList(m: ArrayLike<number>, n: number): Swap[] {
  const out: Swap[] = [];
  for (let i = 0; i < n; i++) for (let j = i + 1; j < n; j++) out.push({ i, j, delta: m[i * n + j] });
  return out.sort((a, b) => a.delta - b.delta);
}
