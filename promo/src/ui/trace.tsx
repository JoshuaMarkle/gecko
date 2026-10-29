// Stat leaders: a dot on what the stat measures, thin ink runs joined by 45° turns (octilinear
// routing), a dot at the label. Neutral, never the accent. Drawn on along the path with `reveal`.

import { clamp01, EASE } from "../lib/motion";
import { LEADER } from "../lib/tokens";

type P = [number, number];

/**
 * Octilinear route a → b: a straight lead along the longer axis (`lead` of the straight part),
 * one 45° diagonal, then the rest straight. `tail` (px) forces a final horizontal run into b.
 */
export function octilinear(a: P, b: P, lead = 0.4, tail = 0): P[] {
  const sxb = Math.sign(b[0] - a[0]) || 1;
  const e: P = tail > 0 ? [b[0] - sxb * Math.min(tail, Math.abs(b[0] - a[0]) * 0.5), b[1]] : b;
  const dx = e[0] - a[0], dy = e[1] - a[1], ax = Math.abs(dx), ay = Math.abs(dy);
  const sx = Math.sign(dx), sy = Math.sign(dy);
  const d = Math.min(ax, ay), s1 = (Math.max(ax, ay) - d) * lead;
  const p1: P = ax >= ay ? [a[0] + sx * s1, a[1]] : [a[0], a[1] + sy * s1];
  const p2: P = [p1[0] + sx * d, p1[1] + sy * d];
  return [a, p1, p2, e, b].filter((p, i, all) => i === 0 || Math.hypot(p[0] - all[i - 1][0], p[1] - all[i - 1][1]) > 0.5);
}

const length = (pts: P[]) => pts.slice(1).reduce((t, p, i) => t + Math.hypot(p[0] - pts[i][0], p[1] - pts[i][1]), 0);

/** SVG group (place inside an <svg>): a dot at `from`, the routed leader, a dot at `to`. */
export function Trace({ from, to, reveal = 1, lead = 0.4, tail = 0, pad = true, via = true, color = LEADER.color, opacity = 1 }: {
  from: P; to: P; reveal?: number; lead?: number; tail?: number; pad?: boolean; via?: boolean; color?: string; opacity?: number;
}) {
  const t = clamp01(reveal);
  const pts = octilinear(from, to, lead, tail);
  const L = length(pts);
  const d = "M" + pts.map((p) => `${p[0].toFixed(1)},${p[1].toFixed(1)}`).join(" L");
  const padIn = EASE.out(clamp01(t * 5));
  const draw = clamp01((t - 0.1) / 0.8);
  const viaIn = EASE.out(clamp01((t - 0.85) / 0.15));
  return (
    <g opacity={opacity}>
      <path d={d} fill="none" stroke={color} strokeWidth={LEADER.width} strokeLinejoin="round" strokeLinecap="round"
        strokeDasharray={L} strokeDashoffset={L * (1 - draw)} />
      {pad && padIn > 0 && <circle cx={from[0]} cy={from[1]} r={LEADER.dot * padIn} fill={color} />}
      {via && viaIn > 0 && <circle cx={to[0]} cy={to[1]} r={LEADER.dot * 0.7 * viaIn} fill={color} />}
    </g>
  );
}
