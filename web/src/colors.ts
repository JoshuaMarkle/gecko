// Data colors for the keyboard (context/design.md): a muted finger palette mirrored per hand,
// and a white → ink ramp for frequency heat. Chrome colors live in styles.css tokens.

import type { Finger } from "./api";

const BY_ORD = ["#cdbfe3", "#b3cde6", "#b9dcc6", "#efd8ae", "#e0e0e0"]; // pinky, ring, middle, index, thumb
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

/** t in [0, 1] → white..ink. */
export function heatColor(t: number): string {
  const x = Math.max(0, Math.min(1, t));
  const v = Math.round(255 - (255 - 20) * Math.pow(x, 0.8));
  const h = v.toString(16).padStart(2, "0");
  return `#${h}${h}${h}`;
}

/** Legend text color that stays readable on `heatColor(t)`. */
export const heatText = (t: number) => (t > 0.5 ? "#ffffff" : INK);
