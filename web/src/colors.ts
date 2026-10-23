// Data colors for the keyboard (context/design.md): a vivid finger palette mirrored per hand,
// and a white → ink ramp for frequency heat. Chrome colors live in styles.css tokens.

import type { Finger, PairKind } from "./api";

const BY_ORD = ["#a98ae0", "#72aeea", "#6fcf94", "#f5bf55", "#d6d6d6"]; // pinky, ring, middle, index, thumb
const ORD: Record<Finger, number> = { LP: 0, LR: 1, LM: 2, LI: 3, LT: 4, RT: 4, RI: 3, RM: 2, RR: 1, RP: 0 };

export const fingerColor = (f: Finger) => BY_ORD[ORD[f]];
export const fingerLegend: [string, string][] = [
  ["Pinky", BY_ORD[0]],
  ["Ring", BY_ORD[1]],
  ["Middle", BY_ORD[2]],
  ["Index", BY_ORD[3]],
];

export const INK = "#141414";
export const ACCENT = "#0066ff";
export const SUCCESS = "#24a148";
export const ERROR = "#da1e28";
export const MUTED = "#707070";
export const HAIR = "#e0e0e0";
/** Keycap fill of keys in a collision (geometry runs): a light tint of ERROR so legends stay readable. */
export const COLLIDE = "#f4a9ad";

/** N-gram arc colors: same-finger red, lateral stretch orange, scissors violet, other same-hand ink, alternation faint. */
export const PAIR_KINDS: Record<PairKind, [string, string]> = {
  sfb: ["SFB", ERROR],
  sfs: ["SFS", ERROR],
  lsb: ["LSB", "#ff832b"],
  scissor: ["Scissors", "#8a3ffc"],
  same_hand: ["Same hand", INK],
  alternate: ["Alternate", "#adadad"],
};

/** t in [0, 1] → white..ink. */
export function heatColor(t: number): string {
  const x = Math.max(0, Math.min(1, t));
  const v = Math.round(255 - (255 - 20) * Math.pow(x, 0.8));
  const h = v.toString(16).padStart(2, "0");
  return `#${h}${h}${h}`;
}

/** Legend text color that stays readable on `heatColor(t)`. */
export const heatText = (t: number) => (t > 0.5 ? "#ffffff" : INK);

/** Series colors for comparing a few solvers or workers in one chart (bench views): ink first,
 *  then the finger hues, never the accent (reserved for selection). */
export const SERIES = ["#141414", "#4f8fd6", "#e0a030", "#3fae6c", "#8f6ad0", "#ff832b", "#9a9a9a", "#d04a8a"];
export const seriesColor = (i: number) => SERIES[i % SERIES.length];
