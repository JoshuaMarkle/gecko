// Hand-rolled SVG charts (no chart library): convergence, metric contributions, finger load,
// a 2D heatmap of the layout, and the worst n-grams.

import { useLayoutEffect, useRef, useState } from "react";
import type { Analysis, Geometry, MetricValue, Ngram, Progress } from "../api";
import { ACCENT, fingerColor, heatColor, heatText, HAIR, INK, MUTED } from "../colors";
import { fmt, fmtTime, scoredMetrics } from "../format";

function useSize<T extends HTMLElement>() {
  const ref = useRef<T>(null);
  const [size, setSize] = useState({ w: 600, h: 240 });
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const ro = new ResizeObserver(([e]) => setSize({ w: Math.max(200, e.contentRect.width), h: Math.max(120, e.contentRect.height) }));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  return [ref, size] as const;
}

function niceTicks(lo: number, hi: number, n = 5): number[] {
  if (!(hi > lo)) return [lo];
  const step0 = (hi - lo) / n;
  const mag = Math.pow(10, Math.floor(Math.log10(step0)));
  const step = [1, 2, 2.5, 5, 10].map((m) => m * mag).find((s) => s >= step0) ?? step0;
  const out = [];
  for (let v = Math.ceil(lo / step) * step; v <= hi + 1e-9; v += step) out.push(+v.toFixed(10));
  return out;
}

export function Convergence({ progress, reference, refName }: { progress: Progress[]; reference?: number; refName: string }) {
  const [ref, { w, h }] = useSize<HTMLDivElement>();
  if (progress.length === 0) return <div className="empty">Run an optimization to see the best score over time.</div>;
  const m = { l: 48, r: 16, t: 10, b: 26 };
  const xs = progress.map((p) => p.elapsed_ms);
  const ys = progress.map((p) => p.best);
  const xMax = Math.max(...xs, 1);
  const lo = Math.min(...ys, reference ?? Infinity) * 0.98;
  // Skip the first few (random-start) points so the interesting tail is not flattened.
  const yHi = Math.max(ys[Math.floor(ys.length * 0.1)] ?? ys[0], lo + 1e-6);
  const sx = (x: number) => m.l + (x / xMax) * (w - m.l - m.r);
  const sy = (y: number) => m.t + (1 - (Math.min(y, yHi) - lo) / Math.max(yHi - lo, 1e-9)) * (h - m.t - m.b);
  let d = "";
  progress.forEach((p, i) => {
    d += i === 0 ? `M${sx(p.elapsed_ms)},${sy(p.best)}` : `H${sx(p.elapsed_ms)}V${sy(p.best)}`;
  });
  d += `H${sx(xMax)}`;
  return (
    <div ref={ref} style={{ height: "100%" }}>
      <svg width={w} height={h}>
        {niceTicks(lo, yHi).map((t) => (
          <g key={t}>
            <line x1={m.l} x2={w - m.r} y1={sy(t)} y2={sy(t)} stroke={HAIR} />
            <text x={m.l - 6} y={sy(t) + 4} textAnchor="end">{fmt(t)}</text>
          </g>
        ))}
        {niceTicks(0, xMax / 1000).map((t) => (
          <text key={t} x={sx(t * 1000)} y={h - 8} textAnchor="middle">{t}s</text>
        ))}
        {reference !== undefined && reference <= yHi && (
          <g>
            <line x1={m.l} x2={w - m.r} y1={sy(reference)} y2={sy(reference)} stroke={MUTED} strokeDasharray="4 3" />
            <text x={w - m.r} y={sy(reference) - 4} textAnchor="end">{refName} {fmt(reference)}</text>
          </g>
        )}
        <path d={d} fill="none" stroke={INK} strokeWidth={1.5} />
        <circle cx={sx(xs[xs.length - 1])} cy={sy(ys[ys.length - 1])} r={3} fill={ACCENT} />
        <text className="ink" x={m.l + 6} y={m.t + 12}>best {fmt(ys[ys.length - 1], 3)} · {fmtTime(xs[xs.length - 1])}</text>
      </svg>
    </div>
  );
}

export function MetricBars({ cur, ref: refA, refName }: { cur: Analysis; ref?: Analysis; refName: string }) {
  const [box, { w }] = useSize<HTMLDivElement>();
  const ms = scoredMetrics(cur.breakdown.metrics);
  const refOf = (id: string) => refA?.breakdown.metrics.find((m) => m.id === id);
  const all = ms.flatMap((m) => [m.weighted, refOf(m.id)?.weighted ?? 0]);
  const lo = Math.min(0, ...all);
  const hi = Math.max(0, ...all, 1e-9);
  const lw = 130;
  const vw = Math.max(80, w - lw - 70);
  const sx = (v: number) => lw + ((v - lo) / (hi - lo)) * vw;
  const rowH = 22;
  return (
    <div ref={box}>
      <svg width={w} height={ms.length * rowH + 24}>
        <text x={lw} y={12}>contribution to score (lower is better) — dark: current, light: {refName}</text>
        {ms.map((m: MetricValue, i) => {
          const y = 22 + i * rowH;
          const r = refOf(m.id);
          return (
            <g key={m.id}>
              <text className="ink" x={0} y={y + 12}>{m.name}</text>
              {r && <rect x={Math.min(sx(0), sx(r.weighted))} y={y + 10} width={Math.abs(sx(r.weighted) - sx(0))} height={6} fill={HAIR} rx={1} />}
              <rect x={Math.min(sx(0), sx(m.weighted))} y={y + 3} width={Math.max(1, Math.abs(sx(m.weighted) - sx(0)))} height={7} fill={INK} rx={1} />
              <text x={lw + vw + 8} y={y + 12}>{fmt(m.weighted)}</text>
            </g>
          );
        })}
        <line x1={sx(0)} x2={sx(0)} y1={18} y2={22 + ms.length * rowH} stroke={MUTED} />
      </svg>
    </div>
  );
}

export function FingerLoad({ a, refA, refName }: { a: Analysis; refA?: Analysis; refName: string }) {
  const [box, { w, h }] = useSize<HTMLDivElement>();
  const idx = a.fingers.map((_, i) => i).filter((i) => a.finger_load[i] > 0 || a.finger_target[i] > 0 || (refA?.finger_load[i] ?? 0) > 0);
  const max = Math.max(...idx.map((i) => Math.max(a.finger_load[i], a.finger_target[i], refA?.finger_load[i] ?? 0)), 1);
  const m = { l: 34, r: 10, t: 16, b: 22 };
  const bw = (w - m.l - m.r) / Math.max(idx.length, 1);
  const sy = (v: number) => m.t + (1 - v / max) * (h - m.t - m.b);
  return (
    <div ref={box} style={{ height: "100%" }}>
      <svg width={w} height={h}>
        <text x={m.l} y={10}>% of keystrokes per finger — bar: current, light: {refName}, tick: target</text>
        {niceTicks(0, max, 4).map((t) => (
          <g key={t}>
            <line x1={m.l} x2={w - m.r} y1={sy(t)} y2={sy(t)} stroke={HAIR} />
            <text x={m.l - 6} y={sy(t) + 4} textAnchor="end">{t}</text>
          </g>
        ))}
        {idx.map((fi, k) => {
          const x = m.l + k * bw;
          const v = a.finger_load[fi];
          const r = refA?.finger_load[fi];
          return (
            <g key={fi}>
              {r !== undefined && <rect x={x + bw * 0.2} y={sy(r)} width={bw * 0.6} height={sy(0) - sy(r)} fill={HAIR} rx={2} />}
              <rect x={x + bw * 0.3} y={sy(v)} width={bw * 0.4} height={sy(0) - sy(v)} fill={fingerColor(a.fingers[fi])} stroke={INK} strokeWidth={0.75} rx={2} />
              <line x1={x + bw * 0.15} x2={x + bw * 0.85} y1={sy(a.finger_target[fi])} y2={sy(a.finger_target[fi])} stroke={INK} strokeWidth={1.5} />
              <text x={x + bw / 2} y={h - 8} textAnchor="middle">{a.fingers[fi]} {fmt(v, 1)}</text>
            </g>
          );
        })}
      </svg>
    </div>
  );
}

/** 2D heatmap of the board: key size from geometry, fill from keystroke share. */
export function Heatmap({ geometry, chars, a, selected, onKey }: { geometry: Geometry; chars: string; a: Analysis; selected: number | null; onKey: (i: number) => void }) {
  const [box, { w, h }] = useSize<HTMLDivElement>();
  const ks = geometry.key;
  const x0 = Math.min(...ks.map((k) => k.pos[0]));
  const x1 = Math.max(...ks.map((k) => k.pos[0] + k.size[0]));
  const y0 = Math.min(...ks.map((k) => k.pos[1]));
  const y1 = Math.max(...ks.map((k) => k.pos[1] + k.size[1]));
  const u = Math.min((w - 20) / (x1 - x0), (h - 30) / (y1 - y0), 64);
  const ox = (w - u * (x1 - x0)) / 2;
  const max = Math.max(...a.heat, 1e-9);
  return (
    <div ref={box} style={{ height: "100%" }}>
      <svg width={w} height={h}>
        {ks.map((k, i) => {
          const t = a.heat[i] / max;
          const x = ox + (k.pos[0] - x0) * u;
          const y = 8 + (k.pos[1] - y0) * u;
          return (
            <g key={i} onClick={() => onKey(i)} style={{ cursor: "pointer" }}>
              <rect x={x + 2} y={y + 2} width={k.size[0] * u - 4} height={k.size[1] * u - 4} rx={6} fill={heatColor(t)} stroke={selected === i ? ACCENT : HAIR} strokeWidth={selected === i ? 2 : 1} />
              <text x={x + (k.size[0] * u) / 2} y={y + (k.size[1] * u) / 2 + 1} textAnchor="middle" style={{ fill: heatText(t), fontWeight: 600, fontSize: Math.max(11, u * 0.3) }}>{(chars[i] ?? "").toUpperCase()}</text>
              <text x={x + (k.size[0] * u) / 2} y={y + k.size[1] * u - 8} textAnchor="middle" style={{ fill: heatText(t), fontSize: 9, opacity: 0.8 }}>{fmt(a.heat[i], 1)}</text>
            </g>
          );
        })}
      </svg>
    </div>
  );
}

function NgramTable({ title, rows }: { title: string; rows: Ngram[] }) {
  const max = Math.max(...rows.map((r) => r.freq), 1e-9);
  return (
    <div>
      <div style={{ fontWeight: 600, marginBottom: 4 }}>{title}</div>
      <table className="tbl">
        <thead><tr><th>n-gram</th><th>% </th><th></th></tr></thead>
        <tbody>
          {rows.map((r) => (
            <tr key={r.ngram}>
              <td className="l mono">{r.ngram.replace(/ /g, "␣")}</td>
              <td>{fmt(r.freq, 3)}</td>
              <td style={{ width: "45%" }}><span style={{ display: "inline-block", height: 6, borderRadius: 2, background: INK, width: `${(100 * r.freq) / max}%` }} /></td>
            </tr>
          ))}
          {rows.length === 0 && <tr><td className="muted">none</td><td /><td /></tr>}
        </tbody>
      </table>
    </div>
  );
}

export function WorstNgrams({ a }: { a: Analysis }) {
  return (
    <div className="grid4">
      <NgramTable title="SFB" rows={a.worst_sfb} />
      <NgramTable title="SFS" rows={a.worst_sfs} />
      <NgramTable title="LSB" rows={a.worst_lsb} />
      <NgramTable title="Scissors" rows={a.worst_scissors} />
    </div>
  );
}
