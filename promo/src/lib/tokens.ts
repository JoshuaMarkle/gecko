// Promo tokens: the product's design tokens (context/design.md, web/src/styles.css, web/src/colors.ts)
// scaled for a 1920 × 1080 video. Every colour has one allowed meaning; use these, never literals.

export const W = 1920;
export const H = 1080;
export const FPS = 30;

/**
 * Promo look: the product's light theme (context/design.md, web/src/styles.css): white stage, near-black
 * ink, one electric blue. The keyboard is the only rich colour: white caps on a black plate.
 */
export const C = {
  ink: "#141414",      // text, primary marks, data lines
  inkSoft: "#262626",  // secondary lockups
  muted: "#707070",    // secondary copy, axis labels, eyebrows
  faint: "#adadad",    // tertiary text, ghost outlines
  canvas: "#ffffff",   // the stage
  panel: "#ffffff",    // cards on the stage
  soft: "#f3f3f3",     // tracks, the 2D viewport, empty cells
  field: "#f0f0f0",    // filled fields
  hairSoft: "#f0f0f0", // faintest dividers
  hair: "#e0e0e0",     // card edges, key outlines
  white: "#ffffff",    // type on the accent
  /** The one blue thing per frame: selection, the best value, the pressed key. */
  accent: "#0066ff",
  accentSoft: "#5c9dff", // glitching glyphs, glows
  accentDeep: "#0047b3", // blue shadows
  /** State only, as small marks. */
  success: "#24a148",
  warning: "#f1c21b",
  error: "#da1e28",
} as const;

/** The 2D board's keys match the 3D caps: white with black legends, a hairline edge. */
export const KEY = { fill: "#ffffff", ink: "#141414", edge: "#d4d4d4" } as const;

/** Finger palette (data): a blue ramp, pinky (deep) → index (pale), thumb grey; mirrored per hand. */
export const FINGER = ["#2563ff", "#4f86ff", "#86abff", "#bdd2ff", "#e3e7ef"] as const;
/** Legend colour readable on each finger fill. */
export const FINGER_INK = ["#ffffff", "#ffffff", "#0a0c11", "#0a0c11", "#0a0c11"] as const;
const ORD: Record<string, number> = { LP: 0, LR: 1, LM: 2, LI: 3, LT: 4, RT: 4, RI: 3, RM: 2, RR: 1, RP: 0 };
export const fingerColor = (f: string) => FINGER[ORD[f] ?? 4];
export const fingerInk = (f: string) => FINGER_INK[ORD[f] ?? 4];
export const fingerName = ["Pinky", "Ring", "Middle", "Index", "Thumb"] as const;

/** N-gram arc colours (the product's PAIR_KINDS). */
export const PAIR = { sfb: C.error, lsb: "#ff832b", scissor: "#8a3ffc", same_hand: C.ink, alternate: C.faint } as const;

/** Series colours for comparing a few lines: ink first, then blues; the accent stays for the best. */
export const SERIES = ["#141414", "#4f86ff", "#9dbcff", "#2a5fd6", "#707070", "#adadad", "#3d6fe0", "#b0c8ff"] as const;

/** t in [0, 1] → white..blue..deep navy, the promo's frequency ramp. */
export function heat(t: number): string {
  const x = Math.pow(Math.max(0, Math.min(1, t)), 0.8);
  const stops: [number, [number, number, number]][] = [[0, [243, 245, 249]], [0.55, [0, 102, 255]], [1, [8, 26, 74]]];
  let i = 1;
  while (i < stops.length - 1 && x > stops[i][0]) i++;
  const [a, ca] = stops[i - 1], [b, cb] = stops[i];
  const f = (x - a) / (b - a);
  const h = (k: number) => Math.round(ca[k] + (cb[k] - ca[k]) * f).toString(16).padStart(2, "0");
  return `#${h(0)}${h(1)}${h(2)}`;
}
export const heatText = (t: number) => (t > 0.4 ? C.white : C.ink);

/** Type: Inter 650 / 450 / 300, mono for layout strings, n-grams and numbers in tables. */
export const FONT = `"Inter", -apple-system, "Helvetica Neue", Arial, sans-serif`;
export const MONO = `"JetBrains Mono", ui-monospace, Menlo, monospace`;
export const WEIGHT = { head: 650, text: 450, light: 300 } as const;

/** Type scale: the web's px × 2, never under 28 px on screen (display sizes on top). */
export const TYPE = {
  hint: 28,      // 11 → 22, floored at 28
  body: 28,      // 12 → 24, floored
  pane: 32,      // pane heading 13 → 26, floored and lifted
  eyebrow: 32,
  stat: 56,      // big number 28 → 56
  headline: 96,
  display: 160,  // the hero number
  wordmark: 220,
} as const;

/** 8 px grid and frame margins. */
export const S = (n: number) => n * 8;
export const MARGIN = { x: 120, y: 96 } as const;

/** Radii (web px × 2). */
export const R = { xs: 8, sm: 12, md: 16 } as const;

/** Hairline width on screen. */
export const HAIRLINE = 2;

/** Stat leaders (ui/trace.tsx): thin ink runs with 45° turns, a small dot at each end. Neutral, never the accent. */
export const LEADER = { color: "#141414", width: 2, dot: 5 } as const;

/** Chart style: thin ink lines, hairline axes, muted mono tick labels. */
export const CHART = { line: 3, axis: HAIRLINE, grid: C.hairSoft, tick: C.muted, tickSize: 22 } as const;

/** 3D materials. Units in the 3D scene are millimetres. White caps on a black board, blue switch light. */
export const MAT = {
  cap: "#f6f7f9",       // keycap PBT white
  capGray: "#b9bcc2",   // the outer-column and thumb caps (grey PBT modifiers)
  capLegend: KEY.ink,
  housing: "#e9ebef",   // MX housing (white top)
  housingBottom: "#1b1d23",
  stem: "#0066ff",      // blue stems
  led: "#d9dce3",       // per-switch LED window (unlit)
  plate: "#1a1c22",     // anodized switch plate
  bottom: "#0e0f13",    // bottom plate
  pcb: "#101116",       // black soldermask PCB, white silkscreen
  pcbEdge: "#2a2c33",
  pcbSilk: "#e9ecf2",
  pcbTrace: "#23262e",  // traces under the black soldermask
  silk: "#e9ecf2",      // the nice!nano's silkscreen
  copper: "#c9b37a",    // ENIG pads
  metal: "#c4c7cf",     // USB-C shell, crystal can, TRRS ring
  chip: "#121316",      // IC packages
  skin: "#eceae6",      // the hands: porcelain white
  stage: C.canvas,      // the 3D stage: background, floor and fog, all white
  floor: C.canvas,
  fog: C.canvas,
} as const;

/** One unit (u) = 19.05 mm. */
export const U = 19.05;
