// Small pure helpers for display and data shaping (unit-tested in format.test.ts).

import type { Dir, Geometry, MetricValue } from "./api";

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

/** Validate a layout string against a geometry and allowed charset. Returns an error or null. */
export function checkLayout(chars: string, keys: number, charset: string): string | null {
  const a = [...chars];
  if (a.length !== keys) return `${a.length} symbols, the board has ${keys} keys`;
  const seen = new Set<string>();
  for (const c of a) {
    if (seen.has(c)) return `'${c}' appears twice`;
    if (!charset.includes(c)) return `'${c}' is not in the corpus charset`;
    seen.add(c);
  }
  return null;
}
