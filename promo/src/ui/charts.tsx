// Charts, hand-rolled SVG like the product's Charts.tsx: hairline axes, muted tabular tick labels,
// thin ink lines, the accent only on the highlighted point. Every chart takes `reveal` (0..1) and
// draws itself on: lines by dash, bars by height, cells by a diagonal sweep.

import type { CSSProperties, ReactNode } from "react";
import { clamp01, EASE, lerp } from "../lib/motion";
import { C, CHART, heat, HAIRLINE, MONO, R, TYPE, WEIGHT } from "../lib/tokens";

export function niceTicks(lo: number, hi: number, n = 4): number[] {
  if (!(hi > lo)) return [lo];
  const step0 = (hi - lo) / n;
  const mag = Math.pow(10, Math.floor(Math.log10(step0)));
  const step = [1, 2, 2.5, 5, 10].map((m) => m * mag).find((s) => s >= step0) ?? step0;
  const out: number[] = [];
  for (let v = Math.ceil(lo / step) * step; v <= hi + 1e-9; v += step) out.push(+v.toFixed(10));
  return out;
}

/** The product's card: white, 1 px (2 px on screen) hairline, --r-md, a 650 title row with a muted note. */
export function Card({ title, note, children, width, height, pad = 28, style }: {
  title?: string; note?: string; children: ReactNode; width?: number; height?: number; pad?: number; style?: CSSProperties;
}) {
  return (
    <div style={{ width, height, background: C.panel, border: `${HAIRLINE}px solid ${C.hair}`, borderRadius: R.md, padding: pad,
      display: "flex", flexDirection: "column", gap: 16, boxSizing: "border-box", overflow: "hidden", ...style }}>
      {title && (
        <div style={{ display: "flex", alignItems: "baseline", gap: 16, fontSize: TYPE.pane, fontWeight: WEIGHT.head, whiteSpace: "nowrap" }}>
          <span>{title}</span>
          {note && <span style={{ fontWeight: WEIGHT.text, color: C.muted, fontSize: TYPE.hint }}>{note}</span>}
        </div>
      )}
      <div style={{ flex: 1, minHeight: 0, position: "relative" }}>{children}</div>
    </div>
  );
}

const tick: CSSProperties = { fontSize: CHART.tickSize, fill: C.muted, fontVariantNumeric: "tabular-nums" };

export interface Series { points: [number, number][]; color?: string; width?: number; step?: boolean; opacity?: number }

/**
 * Line chart (convergence and friends). `step` draws best-so-far stairs like the product's
 * convergence. `threshold` is a dashed reference line; `highlight` marks one point in accent.
 */
export function LineChart({ series, w, h, reveal = 1, xLabel, yLabel, threshold, highlight, logX = false, yDomain, xFormat, yFormat }: {
  series: Series[]; w: number; h: number; reveal?: number; xLabel?: string; yLabel?: string;
  threshold?: { y: number; label: string }; highlight?: [number, number] | null; logX?: boolean; yDomain?: [number, number];
  xFormat?: (x: number) => string; yFormat?: (y: number) => string;
}) {
  const m = { l: 84, r: 24, t: 16, b: 56 };
  const all = series.flatMap((s) => s.points);
  const tx = (x: number) => (logX ? Math.log10(Math.max(1, x)) : x);
  const xs = all.map((p) => tx(p[0])), ys = all.map((p) => p[1]);
  const x0 = Math.min(...xs), x1 = Math.max(...xs);
  const [y0, y1] = yDomain ?? [Math.min(...ys, threshold?.y ?? Infinity), Math.max(...ys)];
  const sx = (x: number) => m.l + ((tx(x) - x0) / Math.max(1e-9, x1 - x0)) * (w - m.l - m.r);
  const sy = (y: number) => m.t + (1 - (Math.min(y1, Math.max(y0, y)) - y0) / Math.max(1e-9, y1 - y0)) * (h - m.t - m.b);
  const r = EASE.inOut(clamp01(reveal));
  const clipW = m.l + r * (w - m.l - m.r) + 2;
  const id = `clip${Math.round(w)}x${Math.round(h)}x${series.length}`;
  return (
    <svg width={w} height={h} style={{ overflow: "visible" }}>
      <defs><clipPath id={id}><rect x={0} y={-20} width={clipW} height={h + 40} /></clipPath></defs>
      {niceTicks(y0, y1).map((v) => (
        <g key={v}>
          <line x1={m.l} x2={w - m.r} y1={sy(v)} y2={sy(v)} stroke={CHART.grid} strokeWidth={HAIRLINE} />
          <text x={m.l - 14} y={sy(v)} dy="0.35em" textAnchor="end" style={tick}>{yFormat ? yFormat(v) : v}</text>
        </g>
      ))}
      <line x1={m.l} x2={w - m.r} y1={h - m.b} y2={h - m.b} stroke={C.hair} strokeWidth={HAIRLINE} />
      {xLabel && <text x={w - m.r} y={h - 12} textAnchor="end" style={tick}>{xLabel}</text>}
      {xFormat && [x0, x1].map((v, i) => <text key={i} x={i ? w - m.r : m.l} y={h - m.b + 30} textAnchor={i ? "end" : "start"} style={tick}>{xFormat(logX ? 10 ** v : v)}</text>)}
      {yLabel && <text x={m.l} y={m.t - 2} style={tick}>{yLabel}</text>}
      {threshold && (
        <g>
          <line x1={m.l} x2={w - m.r} y1={sy(threshold.y)} y2={sy(threshold.y)} stroke={C.muted} strokeWidth={HAIRLINE} strokeDasharray="8 8" />
          <text x={w - m.r} y={sy(threshold.y) - 10} textAnchor="end" style={tick}>{threshold.label}</text>
        </g>
      )}
      <g clipPath={`url(#${id})`}>
        {series.map((s, n) => {
          let d = "";
          s.points.forEach(([x, y], i) => {
            d += i === 0 ? `M${sx(x)},${sy(y)}` : s.step ? `H${sx(x)}V${sy(y)}` : `L${sx(x)},${sy(y)}`;
          });
          return <path key={n} d={d} fill="none" stroke={s.color ?? C.ink} strokeWidth={s.width ?? CHART.line} strokeOpacity={s.opacity ?? 1}
            strokeLinejoin="round" strokeLinecap="round" />;
        })}
      </g>
      {highlight && r >= 1 && (
        <g>
          <circle cx={sx(highlight[0])} cy={sy(highlight[1])} r={14} fill={C.accent} opacity={0.15} />
          <circle cx={sx(highlight[0])} cy={sy(highlight[1])} r={7} fill={C.accent} stroke={C.panel} strokeWidth={3} />
        </g>
      )}
    </svg>
  );
}

/** Vertical bars with labels under them (key frequency, finger load). `colors` per bar (data colours). */
export function Bars({ values, labels, w, h, reveal = 1, colors, max, format, mono = true }: {
  values: number[]; labels: string[]; w: number; h: number; reveal?: number; colors?: string[]; max?: number;
  format?: (v: number) => string; mono?: boolean;
}) {
  const top = max ?? Math.max(...values);
  const lb = 40, n = values.length;
  const bw = w / n, gap = Math.max(2, bw * 0.18);
  return (
    <svg width={w} height={h}>
      <line x1={0} x2={w} y1={h - lb} y2={h - lb} stroke={C.hair} strokeWidth={HAIRLINE} />
      {values.map((v, i) => {
        const t = EASE.out(clamp01(reveal * 1.6 - (i / n) * 0.6));
        const bh = (v / top) * (h - lb - 36) * t;
        return (
          <g key={i}>
            <rect x={i * bw + gap / 2} y={h - lb - bh} width={bw - gap} height={bh} rx={Math.min(6, (bw - gap) / 4)} fill={colors?.[i] ?? C.ink} />
            {format && t > 0.9 && <text x={i * bw + bw / 2} y={h - lb - bh - 10} textAnchor="middle" style={{ ...tick, fontSize: 18 }}>{format(v)}</text>}
            <text x={i * bw + bw / 2} y={h - 10} textAnchor="middle" style={{ ...tick, fill: C.ink, fontFamily: mono ? MONO : undefined, fontSize: Math.min(CHART.tickSize, bw * 0.7) }}>{labels[i]}</text>
          </g>
        );
      })}
    </svg>
  );
}

/** One stacked bar (corpus recipe: web 50 · chat 30 · books 20) with labels under each part. */
export function StackedBar({ parts, w, h = 64, reveal = 1 }: { parts: { label: string; weight: number; color?: string }[]; w: number; h?: number; reveal?: number }) {
  const total = parts.reduce((a, p) => a + p.weight, 0);
  let x = 0;
  const shades = [C.accent, "#79abff", "#c9d8ff", C.faint];
  return (
    <svg width={w} height={h + 64} style={{ overflow: "visible" }}>
      {parts.map((p, i) => {
        const pw = (p.weight / total) * w;
        const t = EASE.out(clamp01(reveal * parts.length - i));
        const el = (
          <g key={i}>
            <rect x={x + 2} y={0} width={Math.max(0, pw * t - 4)} height={h} rx={R.xs} fill={p.color ?? shades[i % shades.length]} />
            <text x={x + 4} y={h + 40} opacity={t} style={{ ...tick, fill: C.ink, fontSize: TYPE.body }}>
              {p.label} <tspan fill={C.muted}>{Math.round((p.weight / total) * 100)}</tspan>
            </text>
          </g>
        );
        x += pw;
        return el;
      })}
    </svg>
  );
}

/** Free-parameter range bars (Board tab): track lo..hi, a tick at the start value, the value moving. */
export function RangeBars({ rows, w, count = 1, accent = false }: {
  rows: { label: string; lo: number; hi: number; start: number; value: number; unit?: string }[]; w: number; count?: number; accent?: boolean;
}) {
  const rowH = 76, lw = 220, vw = 140, tw = w - lw - vw;
  return (
    <svg width={w} height={rows.length * rowH}>
      {rows.map((r, i) => {
        const v = lerp(r.start, r.value, EASE.inOut(clamp01(count)));
        const f = (x: number) => lw + clamp01((x - r.lo) / (r.hi - r.lo)) * tw; // a start outside [lo, hi] pins to the track's end
        const y = i * rowH + rowH / 2;
        return (
          <g key={i}>
            <text x={0} y={y} dy="0.35em" style={{ ...tick, fill: C.inkSoft, fontSize: TYPE.body }}>{r.label}</text>
            <rect x={lw} y={y - 4} width={tw} height={8} rx={4} fill={C.soft} />
            <rect x={Math.min(f(r.start), f(v))} y={y - 4} width={Math.abs(f(v) - f(r.start))} height={8} rx={4} fill={accent ? C.accent : C.ink} />
            <line x1={f(r.start)} x2={f(r.start)} y1={y - 14} y2={y + 14} stroke={C.faint} strokeWidth={HAIRLINE} />
            <circle cx={f(v)} cy={y} r={10} fill={C.panel} stroke={accent ? C.accent : C.ink} strokeWidth={3} />
            <text x={w} y={y} dy="0.35em" textAnchor="end" style={{ ...tick, fill: accent ? C.accent : C.ink, fontSize: TYPE.body, fontWeight: WEIGHT.head }}>
              {v.toFixed(1)}{r.unit ?? "°"}
            </text>
          </g>
        );
      })}
    </svg>
  );
}

/** Matrix heatmap (bigram frequencies): white → ink cells, optional row/column labels; sweeps in diagonally. */
export function Heatmap({ matrix, w, h, reveal = 1, rowLabels, colLabels, gamma = 0.5 }: {
  matrix: number[][]; w: number; h: number; reveal?: number; rowLabels?: string[]; colLabels?: string[]; gamma?: number;
}) {
  const lab = rowLabels ? 28 : 0;
  const rows = matrix.length, cols = matrix[0]?.length ?? 0;
  const cw = (w - lab) / cols, ch = (h - lab) / rows;
  const max = Math.max(...matrix.flat());
  return (
    <svg width={w} height={h}>
      {matrix.map((row, i) => row.map((v, j) => {
        const t = clamp01(reveal * 2 - (i + j) / (rows + cols));
        if (t <= 0) return null;
        return <rect key={`${i}.${j}`} x={lab + j * cw} y={lab + i * ch} width={cw - 1} height={ch - 1} fill={heat(Math.pow(v / max, gamma) * t)} />;
      }))}
      {colLabels?.map((l, j) => <text key={j} x={lab + j * cw + cw / 2} y={lab - 8} textAnchor="middle" style={{ ...tick, fontFamily: MONO, fontSize: Math.min(18, cw) }}>{l}</text>)}
      {rowLabels?.map((l, i) => <text key={i} x={lab - 8} y={lab + i * ch + ch / 2} dy="0.35em" textAnchor="end" style={{ ...tick, fontFamily: MONO, fontSize: Math.min(18, ch) }}>{l}</text>)}
    </svg>
  );
}

/** Explore atlas scatter: heat by score (brighter = better), best in ink, picked in accent with a ring. */
export function Scatter({ points, w, h, reveal = 1, best, picked, xLabel, yLabel, bg = true }: {
  points: { x: number; y: number; s: number }[]; w: number; h: number; reveal?: number; best?: number; picked?: number;
  xLabel?: string; yLabel?: string; bg?: boolean;
}) {
  const m = 24;
  const xs = points.map((p) => p.x), ys = points.map((p) => p.y), ss = points.map((p) => p.s);
  const [x0, x1, y0, y1, s0, s1] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys), Math.min(...ss), Math.max(...ss)];
  const sx = (x: number) => m + ((x - x0) / Math.max(1e-9, x1 - x0)) * (w - 2 * m);
  const sy = (y: number) => h - m - ((y - y0) / Math.max(1e-9, y1 - y0)) * (h - 2 * m);
  const n = points.length;
  return (
    <svg width={w} height={h}>
      {bg && <rect x={0} y={0} width={w} height={h} rx={R.sm} fill={C.soft} />}
      {points.map((p, i) => {
        const t = clamp01(reveal * 1.5 - (i / n) * 0.5);
        if (t <= 0 || i === best || i === picked) return null;
        return <circle key={i} cx={sx(p.x)} cy={sy(p.y)} r={4 * t} fill={heat(0.25 + 0.6 * (1 - (p.s - s0) / Math.max(1e-9, s1 - s0)))} />;
      })}
      {best !== undefined && reveal >= 1 && <circle cx={sx(points[best].x)} cy={sy(points[best].y)} r={7} fill={C.ink} />}
      {picked !== undefined && reveal >= 1 && <circle cx={sx(points[picked].x)} cy={sy(points[picked].y)} r={8} fill={C.accent} stroke={C.soft} strokeWidth={3} />}
      {xLabel && <text x={w - 12} y={h - 12} textAnchor="end" style={tick}>{xLabel}</text>}
      {yLabel && <text x={12} y={28} style={tick}>{yLabel}</text>}
    </svg>
  );
}

/** MAP-Elites grid: one cell per behaviour bin, heat by the cell's best score (empty cells --soft). */
export function EliteGrid({ cells, w, h, reveal = 1, best }: { cells: (number | null)[][]; w: number; h: number; reveal?: number; best?: [number, number] }) {
  const rows = cells.length, cols = cells[0]?.length ?? 0;
  const cw = w / cols, ch = h / rows;
  const vals = cells.flat().filter((v): v is number => v !== null);
  const lo = Math.min(...vals), hi = Math.max(...vals);
  return (
    <svg width={w} height={h}>
      {cells.map((row, i) => row.map((v, j) => {
        const t = clamp01(reveal * 2 - (i + j) / (rows + cols));
        const isBest = best && best[0] === i && best[1] === j;
        const fill = v === null ? C.soft : isBest && reveal >= 1 ? C.accent : heat((1 - (v - lo) / Math.max(1e-9, hi - lo)) * 0.85 * t + 0.08);
        return <rect key={`${i}.${j}`} x={j * cw + 1.5} y={i * ch + 1.5} width={cw - 3} height={ch - 3} rx={3} fill={fill} opacity={v === null ? 1 : clamp01(t * 2)} />;
      }))}
    </svg>
  );
}

/**
 * The live signal line of scene 7: an ink polyline that runs as a moving noisy signal (`phase`
 * scrolls it) and morphs (`morph` 0..1) into the target shape, e.g. a chart's outline. Both are
 * resampled to the same point count so the morph is per point.
 */
export function DataLine({ samples, target, w, h, morph = 0, phase = 0, width = 3, color = C.ink }: {
  /** Signal values in [0, 1] (e.g. letter frequencies along the page). */
  samples: number[];
  /** Target points in px (same frame), or none to stay a signal. */
  target?: [number, number][];
  w: number; h: number; morph?: number; phase?: number; width?: number; color?: string;
}) {
  const N = 240;
  const sig = Array.from({ length: N }, (_, i) => {
    const u = i / (N - 1);
    const k = (u * samples.length + phase) % samples.length;
    const a = samples[Math.floor(k)], b = samples[(Math.floor(k) + 1) % samples.length];
    const v = a + (b - a) * (k - Math.floor(k));
    return [u * w, h / 2 - (v - 0.5) * h * 0.8] as [number, number];
  });
  const resample = (pts: [number, number][]) => Array.from({ length: N }, (_, i) => {
    const x = (i / (N - 1)) * (pts.length - 1), j = Math.min(pts.length - 2, Math.floor(x)), f = x - j;
    return [lerp(pts[j][0], pts[j + 1][0], f), lerp(pts[j][1], pts[j + 1][1], f)] as [number, number];
  });
  const tg = target ? resample(target) : sig;
  const m = EASE.inOut(clamp01(morph));
  const d = sig.map((p, i) => `${i ? "L" : "M"}${lerp(p[0], tg[i][0], m).toFixed(1)},${lerp(p[1], tg[i][1], m).toFixed(1)}`).join("");
  return (
    <svg width={w} height={h} style={{ overflow: "visible" }}>
      <path d={d} fill="none" stroke={color} strokeWidth={width} strokeLinejoin="round" strokeLinecap="round" />
    </svg>
  );
}
