// The product's 2D board (web/src/components/Board2D.tsx `KeyGrid`) at display scale: keys at the
// flat drawing's positions, --r-sm corners, hairline edges, finger or heat fills, the accent ring.
// Promo additions: legends that cross-fade in place when a layout swaps (`swap`), per-key pose
// numbers (`labels`), a column-by-column finger wipe (`wipe`), and n-gram arcs that draw on.

import type { ReactNode } from "react";
import { useCurrentFrame } from "remotion";
import { hash01 } from "../lib/rng";
import { data, symbols, SPECIAL } from "../lib/data";
import { clamp01, EASE } from "../lib/motion";
import { C, fingerColor, fingerInk, heat as heatRamp, heatText, KEY, MONO, U } from "../lib/tokens";
import { ICONS, LEGEND, LEGEND_MM } from "../three/legend";

const KEYS = data.board.keys;
const FLAT = data.board.flat;
const X0 = Math.min(...FLAT.map((f) => f[0])), Y0 = Math.min(...FLAT.map((f) => f[1]));
const X1 = Math.max(...FLAT.map((f, i) => f[0] + KEYS[i].size[0])), Y1 = Math.max(...FLAT.map((f, i) => f[1] + KEYS[i].size[1]));
/** Extra gap between the halves (u): Corne3D's default 36 mm spread, so the drawing matches the 3D board's top view. */
export const SPLIT_U = 36 / U;
const right = (i: number) => KEYS[i].finger[0] === "R";
/** Board size in key units (with the split). */
export const BOARD_U = { w: X1 - X0 + SPLIT_U, h: Y1 - Y0 };
/** Left edge of key i (u, from the board's left edge). */
const kx = (i: number) => FLAT[i][0] - X0 + (right(i) ? SPLIT_U : 0);

/** Centre of key i in px (board drawn at `u` px per unit, origin top-left). */
export const keyCentre = (i: number, u: number): [number, number] =>
  [(kx(i) + KEYS[i].size[0] / 2) * u, (FLAT[i][1] - Y0 + KEYS[i].size[1] / 2) * u];

export interface Arc2D { i: number; j: number; color: string; w: number }

export interface Board2DProps {
  /** px per key unit. */
  u: number;
  chars?: string;
  /** Legend swap in progress: the previous layout and 0..1 progress toward `chars`. */
  swap?: { from: string; t: number };
  fill?: "white" | "fingers" | "heat" | string[];
  /** Heat per key 0..1 (fill = "heat"). */
  heat?: number[];
  /** 0..1: finger colours wipe in column by column, left to right (over `fill`). */
  wipe?: number;
  accent?: number | null;
  /** 0..1: legends turn into each key's pose (x, y), column by column, digits scrambling until they settle. */
  labels?: number;
  arcs?: Arc2D[];
  /** 0..1 draw-on of the arcs. */
  arcReveal?: number;
  /** Fade (0..1) of shading-less white keys toward the flat product look (no effect when 1). */
  opacity?: number;
  children?: ReactNode;
}

/** A key's legend at the same size and weight as the 3D cap's (three/legend.ts), so the hand-off from the 3D top view is seamless. */
function Legend({ ch, x, y, size, color, opacity = 1 }: { ch: string; x: number; y: number; size: number; color: string; opacity?: number }) {
  const sp = SPECIAL[ch];
  const k = (LEGEND_MM / U) * size; // px per legend unit
  if (sp?.icon) {
    if (!ICONS[sp.icon]) return null;
    return <path d={ICONS[sp.icon]} transform={`translate(${x} ${y}) scale(${k})`} fill="none" stroke={color} strokeWidth={LEGEND.stroke}
      strokeLinejoin="round" strokeLinecap="round" opacity={opacity} />;
  }
  const text = sp ? sp.label : ch.toUpperCase();
  return <text x={x} y={y} dy="0.36em" textAnchor="middle" fill={color} opacity={opacity}
    style={{ fontSize: LEGEND.textSize(text) * k, fontWeight: LEGEND.weight }}>{text}</text>;
}

/** Key i's pose as two mono lines (x, y in u); digits scramble until `reveal` settles them. */
function PoseNumbers({ i, x, y, u, color, reveal }: { i: number; x: number; y: number; u: number; color: string; reveal: number }) {
  const frame = useCurrentFrame();
  const p = KEYS[i].pose;
  const lines = [p[0].toFixed(2), p[1].toFixed(2)];
  const scramble = (s: string, n: number) => reveal >= 1 ? s : [...s].map((c, j) => (c === "." || hash01(frame * 31 + i, j + n * 7) < reveal ? c : String(Math.floor(hash01(i + n, frame + j) * 10)))).join("");
  const fs = Math.max(11, u * 0.2);
  return (
    <text x={x} y={y} textAnchor="middle" fill={color} opacity={clamp01(reveal * 2)} style={{ fontFamily: MONO, fontSize: fs, fontWeight: 500 }}>
      <tspan x={x} dy={-fs * 0.2}>{scramble(lines[0], 0)}</tspan>
      <tspan x={x} dy={fs * 1.15}>{scramble(lines[1], 1)}</tspan>
    </text>
  );
}

export function Board2D({
  u, chars = data.layout.start, swap, fill = "white", heat, wipe, accent = null, labels = 0,
  arcs, arcReveal = 1, opacity = 1, children,
}: Board2DProps) {
  const sym = symbols(chars);
  const prev = swap ? symbols(swap.from) : null;
  const W = BOARD_U.w * u + 4, H = BOARD_U.h * u + 4;
  const gap = Math.max(2, u * 0.06);
  const r = Math.max(4, u * 0.1);
  const colFrac = (i: number) => (FLAT[i][0] - X0 + KEYS[i].size[0] / 2) / BOARD_U.w;
  const fillOf = (i: number): [string, string] => {
    if (accent === i) return [C.accent, C.white];
    if (wipe !== undefined) {
      const t = clamp01((wipe - colFrac(i) * 0.8) / 0.2);
      if (t > 0.5) return [fingerColor(KEYS[i].finger), fingerInk(KEYS[i].finger)];
    }
    if (Array.isArray(fill)) return [fill[i], KEY.ink];
    if (fill === "fingers") return [fingerColor(KEYS[i].finger), fingerInk(KEYS[i].finger)];
    if (fill === "heat" && heat) return [heatRamp(heat[i]), heatText(heat[i])];
    return [KEY.fill, KEY.ink];
  };
  const t = swap ? EASE.inOut(clamp01(swap.t)) : 1;
  return (
    <svg width={W} height={H} viewBox={`-2 -2 ${W} ${H}`} style={{ overflow: "visible", opacity }}>
      {KEYS.map((k, i) => {
        const [, fy, rot] = FLAT[i];
        const x = kx(i) * u, y = (fy - Y0) * u, w = k.size[0] * u - gap, h = k.size[1] * u - gap;
        const [bg] = fillOf(i);
        return (
          <g key={i} transform={rot ? `rotate(${rot} ${x + w / 2} ${y + h / 2})` : undefined}>
            <rect x={x + gap / 2} y={y + gap / 2} width={w} height={h} rx={r} fill={bg} stroke={accent === i ? C.accent : KEY.edge} strokeWidth={accent === i ? 3 : 1} />
          </g>
        );
      })}
      {/* legends above all keys; a swapped one cross-fades in place; with `labels` they turn into the key's pose numbers */}
      {KEYS.map((k, i) => {
        const [cx, cy] = keyCentre(i, u);
        const [, ink] = fillOf(i);
        const changing = !!prev && prev[i] !== sym[i] && t < 1;
        const num = labels > 0 ? clamp01((labels - colFrac(i) * 0.6) / 0.4) : 0;
        return (
          <g key={i}>
            {changing && <Legend ch={prev[i]} x={cx} y={cy} size={u} color={ink} opacity={1 - t} />}
            {num < 1 && <Legend ch={sym[i]} x={cx} y={cy} size={u} color={ink} opacity={(changing ? t : 1) * (1 - num)} />}
            {num > 0 && <PoseNumbers i={i} x={cx} y={cy} u={u} color={ink} reveal={num} />}
          </g>
        );
      })}
      {arcs?.map((a, n) => {
        const [p, q] = [keyCentre(a.i, u), keyCentre(a.j, u)];
        const dist = Math.hypot(q[0] - p[0], q[1] - p[1]);
        const mx = (p[0] + q[0]) / 2, my = (p[1] + q[1]) / 2 - Math.max(u * 0.3, dist * 0.35);
        const len = dist * 1.25 + 1;
        const reveal = clamp01((arcReveal - (n / arcs.length) * 0.5) / 0.5);
        return <path key={n} d={`M${p[0]},${p[1]} Q${mx},${my} ${q[0]},${q[1]}`} fill="none" stroke={a.color}
          strokeWidth={1.5 + 6 * a.w} strokeOpacity={0.25 + 0.75 * a.w} strokeLinecap="round"
          strokeDasharray={len} strokeDashoffset={len * (1 - reveal)} />;
      })}
      {children}
    </svg>
  );
}
