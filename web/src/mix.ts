// Language composition (Language tab): how a recipe's parts add up to each symbol's share, and a
// live preview when the weights change. Mirrors gecko-core `Corpus::mix` (normalized parts,
// weighted) and `with_edits` (⌫ e of all keystrokes, as many mistyped keys drawn from the
// monograms, the real text scaled to 1 − 2e), so an unchanged recipe previews its built shares.

/** Part colors: five chromatic slots (validated for color-vision separation with the legend and
 *  gaps as second encoding); a sixth part and beyond fold into OTHER. Modelled keys are hatched. */
export const PART_COLORS = ["#4f8fd6", "#e0a030", "#8f6ad0", "#3fae6c", "#d04a8a"];
export const OTHER = "#adadad";
export const partColor = (i: number) => PART_COLORS[i] ?? OTHER;

export const BACKSPACE = "⌫";
export const ARROWS = ["←", "↓", "↑", "→"];

export interface PartShares { corpus: string; share: number; mono: Record<string, number> }

export interface Preview {
  /** Final share of each symbol (sums to 1). */
  mono: Record<string, number>;
  /** Per symbol, each part's contribution (same order as the parts), then the modelled keys'. */
  contrib: Record<string, number[]>;
  /** Share of all keystrokes from each part (after modelled keys), then the modelled keys. */
  parts: number[];
}

/** Shares of a weighted mix of parts plus modelled Backspace (`edits`) and arrows (`nav`). */
export function preview(parts: PartShares[], edits: number, nav: number): Preview {
  const tot = parts.reduce((a, p) => a + p.share, 0) || 1;
  const w = parts.map((p) => p.share / tot);
  const syms = new Set<string>();
  for (const p of parts) for (const s of Object.keys(p.mono)) syms.add(s);
  if (edits > 0) syms.add(BACKSPACE);
  if (nav > 0) ARROWS.forEach((a) => syms.add(a));
  // Text keeps 1 − 2e (edits) of the keystrokes; mistyped keys (e) follow the mix's monograms.
  // Arrows then take `nav` of everything (runs of arrows only; text keeps 1 − nav).
  const keepE = edits > 0 ? 1 - edits : 1;
  const keepN = 1 - nav;
  const mono: Record<string, number> = {};
  const contrib: Record<string, number[]> = {};
  const partTotals = new Array(parts.length + 1).fill(0);
  for (const s of syms) {
    const c = parts.map((p, i) => w[i] * (p.mono[s] ?? 0) * keepE * keepN);
    let modelled = 0;
    if (s === BACKSPACE && edits > 0) modelled += edits * keepN;
    if (nav > 0 && ARROWS.includes(s)) modelled += nav / ARROWS.length;
    contrib[s] = [...c, modelled];
    mono[s] = c.reduce((a, b) => a + b, 0) + modelled;
    c.forEach((v, i) => (partTotals[i] += v));
    partTotals[parts.length] += modelled;
  }
  return { mono, contrib, parts: partTotals };
}

export type SymbolGroup = "keys" | "symbols";

/** Letters, space and special keys vs punctuation and digits (the two composition charts). */
export function groupOf(s: string): SymbolGroup {
  return /^[!-~]$/.test(s) && !/^[a-z]$/.test(s) ? "symbols" : "keys";
}

/** Printable ASCII a full keyboard types (lowercase letters; capitals are Shift + letter). */
export const ASCII = Array.from({ length: 94 }, (_, i) => String.fromCharCode(33 + i)).filter((c) => !/[A-Z]/.test(c));

/** Printable ASCII symbols the charset lacks entirely. */
export const notInCharset = (charset: string) => ASCII.filter((c) => !charset.includes(c));

/** Symbols sorted by share, descending (ties by symbol). */
export const byShare = (mono: Record<string, number>, group: SymbolGroup) =>
  Object.keys(mono).filter((s) => groupOf(s) === group).sort((a, b) => mono[b] - mono[a] || a.localeCompare(b));

/** Weights equal to the saved recipe's (tolerance for slider rounding). */
export const sameWeights = (a: { corpus: string; weight: number }[], b: { corpus: string; weight: number }[]) =>
  a.length === b.length && a.every((p, i) => p.corpus === b[i].corpus && Math.abs(p.weight / sum(a) - b[i].weight / sum(b)) < 1e-9);
const sum = (v: { weight: number }[]) => v.reduce((x, p) => x + p.weight, 0) || 1;
