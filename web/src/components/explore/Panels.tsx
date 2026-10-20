// Explore panels (hand-rolled SVG, like Charts.tsx): families gallery, funnels, landscape
// (random vs optima + correlogram), discovery curve, weight stability, parallel coordinates.

import { useEffect, useMemo, useRef, useState } from "react";
import type { Geometry, MetricDef } from "../../api";
import { ACCENT, HAIR, INK, MUTED } from "../../colors";
import { histogram, stabilityRange } from "../../explore";
import { fmt, fmtTime } from "../../format";
import { niceTicks, useSize } from "../Charts";
import { Thumb } from "./Board";

const FAINT = "#adadad";

export interface Family { id: number; rows: number[]; leader: number; best: number; worst: number; d: number; chars: string }

export function Families({ families, geometry, bestChars, bestScore, picked, onSelect }: {
  families: Family[]; geometry: Geometry; bestChars: string; bestScore: number; picked: number | null; onSelect: (f: Family) => void;
}) {
  if (families.length === 0) return <div className="empty">No layouts in focus.</div>;
  return (
    <div className="gallery">
      {families.slice(0, 24).map((f) => (
        <button key={f.id} className={`famcard ${picked !== null && f.rows.includes(picked) ? "sel" : ""}`} onClick={() => onSelect(f)}
          title={`Family ${f.id + 1}: ${f.rows.length} layouts within the cut of their leader. Grey keys differ from the best layout.`}>
          <div className="famhead"><b>F{f.id + 1}</b><span>{f.rows.length} layout{f.rows.length === 1 ? "" : "s"}</span><span className="muted">d {f.d}</span></div>
          <Thumb geometry={geometry} chars={f.chars} diff={bestChars} />
          <div className="famfoot"><span>{fmt(f.best, 3)}</span><span className="muted">{f.best - bestScore > 1e-9 ? `+${fmt(f.best - bestScore, 3)}` : "best"}</span></div>
        </button>
      ))}
    </div>
  );
}

export interface Funnel { bottom: number; rows: number; hits: number; score: number; d: number }

export function Funnels({ funnels, totalHits, picked, onSelect }: { funnels: Funnel[]; totalHits: number; picked: number | null; onSelect: (f: Funnel) => void }) {
  const shown = funnels.slice(0, 14);
  const maxShare = Math.max(...shown.map((f) => f.hits / Math.max(totalHits, 1)), 1e-9);
  return (
    <table className="tbl">
      <thead><tr><th>bottom</th><th>score</th><th>d best</th><th>optima</th><th style={{ width: "34%" }}>basin (share of descents)</th></tr></thead>
      <tbody>
        {shown.map((f, k) => (
          <tr key={f.bottom} className={`click ${picked === f.bottom ? "sel" : ""}`} onClick={() => onSelect(f)}>
            <td>#{k + 1}</td>
            <td>{fmt(f.score, 3)}</td>
            <td>{f.d}</td>
            <td>{f.rows}</td>
            <td><span className="inbar" style={{ width: `${(60 * f.hits) / Math.max(totalHits, 1) / maxShare}px` }} /> <span className="muted">{fmt((100 * f.hits) / Math.max(totalHits, 1), 1)}%</span></td>
          </tr>
        ))}
        {funnels.length === 0 && <tr><td className="muted">No edges yet.</td><td /><td /><td /><td /></tr>}
      </tbody>
    </table>
  );
}

/** Score distributions of random layouts vs local optima (heights normalized per series). */
export function Landscape({ random, optima, refs }: { random: ArrayLike<number>; optima: ArrayLike<number>; refs: { label: string; score: number }[] }) {
  const [box, { w, h }] = useSize<HTMLDivElement>();
  const all = [...Array.from(random), ...Array.from(optima)].filter(Number.isFinite);
  if (all.length === 0) return <div ref={box} className="empty">No samples.</div>;
  const lo = Math.min(...all), hi = Math.max(...all);
  const bins = 70;
  const hr = histogram(random, lo, hi, bins), ho = histogram(optima, lo, hi, bins);
  const mr = Math.max(...hr, 1), mo = Math.max(...ho, 1);
  const m = { l: 12, r: 12, t: 28, b: 30 };
  const bw = (w - m.l - m.r) / bins;
  const sx = (v: number) => m.l + ((v - lo) / (hi - lo || 1)) * (w - m.l - m.r);
  const base = h - m.b;
  const bar = (c: number, max: number) => (c / max) * (h - m.t - m.b);
  return (
    <div ref={box} style={{ height: "100%" }}>
      <svg width={w} height={h}>
        <g className="legendrow"><rect x={m.l} y={4} width={10} height={10} rx={2} fill={INK} /><text x={m.l + 14} y={13}>local optima ({optima.length})</text>
          <rect x={m.l + 150} y={4} width={10} height={10} rx={2} fill={HAIR} /><text x={m.l + 164} y={13}>random layouts ({random.length})</text>
          <text x={w - m.r} y={13} textAnchor="end">heights normalized per series · lower score is better</text></g>
        {hr.map((c, i) => c > 0 && <rect key={`r${i}`} x={m.l + i * bw + 1} y={base - bar(c, mr)} width={Math.max(1, bw - 2)} height={bar(c, mr)} fill={HAIR} />)}
        {ho.map((c, i) => c > 0 && <rect key={`o${i}`} x={m.l + i * bw + 1} y={base - bar(c, mo)} width={Math.max(1, bw - 2)} height={bar(c, mo)} fill={INK} />)}
        <line x1={m.l} x2={w - m.r} y1={base + 0.5} y2={base + 0.5} stroke={MUTED} />
        {niceTicks(lo, hi, 8).map((t) => <text key={t} x={sx(t)} y={h - 14} textAnchor="middle">{fmt(t)}</text>)}
        {refs.filter((r) => r.score >= lo && r.score <= hi).sort((a, b) => a.score - b.score).map((r, k) => (
          <g key={r.label}><line x1={sx(r.score)} x2={sx(r.score)} y1={m.t + (k % 3) * 12} y2={base} stroke={MUTED} strokeDasharray="2 2" />
            <text x={sx(r.score) + 3} y={m.t + 9 + (k % 3) * 12} className="ink">{r.label}</text></g>
        ))}
      </svg>
    </div>
  );
}

/** Autocorrelation of score along random swap walks, with the correlation length. */
export function Correlogram({ rho, ell }: { rho: number[]; ell: number }) {
  const [box, { w, h }] = useSize<HTMLDivElement>();
  const m = { l: 34, r: 10, t: 22, b: 26 };
  const n = Math.max(rho.length - 1, 1);
  const sx = (k: number) => m.l + (k / n) * (w - m.l - m.r);
  const sy = (v: number) => m.t + (1 - Math.max(-0.2, v)) / 1.2 * (h - m.t - m.b);
  return (
    <div ref={box} style={{ height: "100%" }}>
      <svg width={w} height={h}>
        <text x={m.l} y={12} className="ink">ρ(k) of score after k random swaps · correlation length ℓ = {fmt(ell, 1)} swaps</text>
        {[0, 0.5, 1].map((t) => <g key={t}><line x1={m.l} x2={w - m.r} y1={sy(t)} y2={sy(t)} stroke={HAIR} /><text x={m.l - 6} y={sy(t) + 4} textAnchor="end">{t}</text></g>)}
        {niceTicks(0, n, 5).map((t) => <text key={t} x={sx(t)} y={h - 10} textAnchor="middle">{t}</text>)}
        {Number.isFinite(ell) && ell <= n && <line x1={sx(ell)} x2={sx(ell)} y1={m.t} y2={h - m.b} stroke={MUTED} strokeDasharray="4 3" />}
        <path d={rho.map((v, k) => `${k ? "L" : "M"}${sx(k)},${sy(v)}`).join("")} fill="none" stroke={INK} strokeWidth={2} strokeLinejoin="round" />
      </svg>
    </div>
  );
}

/** Distinct optima found over time, and how many are near the best: flattening = coverage. */
export function Discovery({ firstMs, near, elapsedMs }: { firstMs: ArrayLike<number>; near: boolean[]; elapsedMs: number }) {
  const [box, { w, h }] = useSize<HTMLDivElement>();
  const [hover, setHover] = useState<number | null>(null);
  const pts = useMemo(() => {
    const idx = Array.from({ length: firstMs.length }, (_, i) => i).sort((a, b) => firstMs[a] - firstMs[b]);
    const all: [number, number][] = [], close: [number, number][] = [];
    let c = 0;
    idx.forEach((i, k) => { all.push([firstMs[i], k + 1]); if (near[i]) close.push([firstMs[i], ++c]); });
    return { all, close };
  }, [firstMs, near]);
  const m = { l: 46, r: 12, t: 22, b: 26 };
  const xMax = Math.max(elapsedMs, ...pts.all.map((p) => p[0]), 1);
  const yMax = Math.max(pts.all.length, 1);
  const sx = (x: number) => m.l + (x / xMax) * (w - m.l - m.r);
  const sy = (y: number) => m.t + (1 - y / yMax) * (h - m.t - m.b);
  const step = (ps: [number, number][]) => {
    if (!ps.length) return "";
    const stride = Math.max(1, Math.floor(ps.length / 800));
    let d = `M${sx(0)},${sy(0)}`;
    ps.forEach(([x, y], k) => { if (k % stride === 0 || k === ps.length - 1) d += `H${sx(x)}V${sy(y)}`; });
    return d + `H${sx(xMax)}`;
  };
  const sy2 = (y: number) => m.t + (1 - y / Math.max(pts.close.length, 1)) * (h - m.t - m.b);
  return (
    <div ref={box} style={{ height: "100%" }} onPointerLeave={() => setHover(null)}
      onPointerMove={(e) => { const r = e.currentTarget.getBoundingClientRect(); setHover(Math.max(0, ((e.clientX - r.left - m.l) / (w - m.l - m.r)) * xMax)); }}>
      <svg width={w} height={h}>
        <g className="legendrow"><line x1={m.l} x2={m.l + 14} y1={9} y2={9} stroke={INK} strokeWidth={2} /><text x={m.l + 18} y={13}>distinct optima ({pts.all.length})</text>
          <line x1={m.l + 160} x2={m.l + 174} y1={9} y2={9} stroke={MUTED} strokeWidth={2} strokeDasharray="4 2" /><text x={m.l + 178} y={13}>near the best ({pts.close.length}, own scale)</text></g>
        {niceTicks(0, yMax, 4).map((t) => <g key={t}><line x1={m.l} x2={w - m.r} y1={sy(t)} y2={sy(t)} stroke={HAIR} /><text x={m.l - 6} y={sy(t) + 4} textAnchor="end">{t}</text></g>)}
        {niceTicks(0, xMax / 1000, 6).map((t) => <text key={t} x={sx(t * 1000)} y={h - 10} textAnchor="middle">{t}s</text>)}
        <path d={step(pts.all)} fill="none" stroke={INK} strokeWidth={2} />
        <path d={(() => { const ps = pts.close; if (!ps.length) return ""; let d = `M${sx(0)},${sy2(0)}`; ps.forEach(([x, y]) => { d += `H${sx(x)}V${sy2(y)}`; }); return d + `H${sx(xMax)}`; })()}
          fill="none" stroke={MUTED} strokeWidth={2} strokeDasharray="4 2" />
        {hover !== null && (() => {
          const a = pts.all.filter((p) => p[0] <= hover).length, c = pts.close.filter((p) => p[0] <= hover).length;
          return <g><line x1={sx(hover)} x2={sx(hover)} y1={m.t} y2={h - m.b} stroke={MUTED} />
            <text x={Math.min(sx(hover) + 6, w - 150)} y={m.t + 12} className="ink">{fmtTime(hover)}: {a} optima · {c} near best</text></g>;
        })()}
      </svg>
    </div>
  );
}

export interface StabilityRow { k: number; def: MetricDef; lo: number; hi: number; below: number | null; above: number | null }

/** For each weighted metric: the range of weight multipliers over which the best layout stays best. */
export function Stability({ rows, factors, describe }: { rows: StabilityRow[]; factors: number[]; describe: (row: number | null) => string }) {
  const [box, { w }] = useSize<HTMLDivElement>();
  const lw = 120, rw = 92;
  const [f0, f1] = [factors[0], factors[factors.length - 1]];
  const sx = (f: number) => lw + ((Math.log(f) - Math.log(f0)) / (Math.log(f1) - Math.log(f0))) * (w - lw - rw);
  const rh = 20;
  return (
    <div ref={box}>
      <svg width={w} height={rows.length * rh + 34}>
        <text x={lw} y={12}>bar: weight × factor (log) that keeps the best on top</text>
        {[0.25, 0.5, 1, 2, 4].filter((f) => f >= f0 && f <= f1).map((f) => (
          <g key={f}><line x1={sx(f)} x2={sx(f)} y1={18} y2={22 + rows.length * rh} stroke={f === 1 ? MUTED : HAIR} />
            <text x={sx(f)} y={rows.length * rh + 32} textAnchor="middle">×{f}</text></g>
        ))}
        {rows.map((r, i) => {
          const y = 22 + i * rh;
          const full = r.below === null && r.above === null;
          return (
            <g key={r.k}>
              <title>{`${r.def.name}: best for ×${fmt(r.lo)}–×${fmt(r.hi)}\nbelow: ${describe(r.below)}\nabove: ${describe(r.above)}`}</title>
              <text className="ink" x={0} y={y + 13}>{r.def.name}</text>
              <rect x={lw} y={y} width={w - lw - rw} height={rh} fill="transparent" />
              <rect x={sx(r.lo)} y={y + 5} width={Math.max(2, sx(r.hi) - sx(r.lo))} height={9} rx={2} fill={full ? FAINT : INK} />
              <text x={w - rw + 8} y={y + 13}>{full ? "always" : `×${fmt(r.lo)}–×${fmt(r.hi)}`}</text>
            </g>
          );
        })}
      </svg>
    </div>
  );
}

export interface Axis { key: string; label: string; values: ArrayLike<number>; /** true: lower is better (drawn at the bottom). */ lowerBetter: boolean }

/** Parallel coordinates over focused layouts; drag on an axis to filter, double-click to clear. */
export function ParallelCoords({ axes, focus, context, picked, onBrush }: {
  axes: Axis[]; focus: ArrayLike<number>; context: ArrayLike<number>; picked: number | null; onBrush: (rows: number[] | null) => void;
}) {
  const [box, { w, h }] = useSize<HTMLDivElement>();
  const canvas = useRef<HTMLCanvasElement>(null);
  const [ranges, setRanges] = useState<Record<string, [number, number]>>({});
  const [drag, setDrag] = useState<{ key: string; a: number; b: number } | null>(null);
  const m = { l: 24, r: 24, t: 30, b: 14 };
  const bounds = useMemo(() => axes.map((a) => {
    let lo = Infinity, hi = -Infinity;
    for (const set of [focus, context]) for (let k = 0; k < set.length; k++) { const v = a.values[set[k]]; if (v < lo) lo = v; if (v > hi) hi = v; }
    return hi > lo ? [lo, hi] : [lo - 1, lo + 1];
  }), [axes, focus, context]);
  const ax = (j: number) => m.l + (j / Math.max(axes.length - 1, 1)) * (w - m.l - m.r);
  // 0 = bottom (better), 1 = top.
  const norm = (j: number, v: number) => { const [lo, hi] = bounds[j]; const t = (v - lo) / (hi - lo); return axes[j].lowerBetter ? t : 1 - t; };
  const py = (t: number) => m.t + (1 - t) * (h - m.t - m.b);
  const inRanges = (i: number) => axes.every((a, j) => { const r = ranges[a.key]; if (!r) return true; const t = norm(j, a.values[i]); return t >= r[0] && t <= r[1]; });

  useEffect(() => {
    const c = canvas.current;
    if (!c) return;
    const dpr = window.devicePixelRatio || 1;
    c.width = Math.round(w * dpr); c.height = Math.round(h * dpr);
    const g = c.getContext("2d")!;
    g.setTransform(dpr, 0, 0, dpr, 0, 0);
    g.clearRect(0, 0, w, h);
    const line = (i: number) => { g.beginPath(); axes.forEach((a, j) => { const y = py(norm(j, a.values[i])); if (j) g.lineTo(ax(j), y); else g.moveTo(ax(j), y); }); g.stroke(); };
    g.lineWidth = 1;
    g.strokeStyle = "rgba(20,20,20,0.05)";
    for (let k = 0; k < context.length; k++) line(context[k]);
    const filtered = Object.keys(ranges).length > 0;
    for (let k = 0; k < focus.length; k++) {
      const i = focus[k];
      g.strokeStyle = filtered && !inRanges(i) ? "rgba(20,20,20,0.05)" : "rgba(20,20,20,0.22)";
      line(i);
    }
    if (picked !== null) { g.strokeStyle = ACCENT; g.lineWidth = 2; line(picked); }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [axes, focus, context, picked, ranges, w, h, bounds]);

  const toT = (y: number) => Math.max(0, Math.min(1, 1 - (y - m.t) / (h - m.t - m.b)));
  const apply = (next: Record<string, [number, number]>) => {
    setRanges(next);
    if (Object.keys(next).length === 0) { onBrush(null); return; }
    const rows: number[] = [];
    const all = [...Array.from(focus), ...Array.from(context)];
    for (const i of all) if (axes.every((a, j) => { const r = next[a.key]; if (!r) return true; const t = norm(j, a.values[i]); return t >= r[0] && t <= r[1]; })) rows.push(i);
    onBrush(rows.length ? rows : []);
  };
  return (
    <div ref={box} className="scatter" style={{ height: "100%" }} onDoubleClick={() => apply({})}>
      <canvas ref={canvas} style={{ width: w, height: h }} />
      <svg className="axes" width={w} height={h} style={{ pointerEvents: "none" }}>
        {axes.map((a, j) => (
          <g key={a.key}>
            <line x1={ax(j)} x2={ax(j)} y1={m.t} y2={h - m.b} stroke={MUTED} />
            <text x={ax(j)} y={12} textAnchor={j === 0 ? "start" : j === axes.length - 1 ? "end" : "middle"} className="ink">{a.label}</text>
            <text x={ax(j)} y={24} textAnchor={j === 0 ? "start" : j === axes.length - 1 ? "end" : "middle"}>{fmt(a.lowerBetter ? bounds[j][1] : bounds[j][0])}</text>
            {ranges[a.key] && <rect x={ax(j) - 5} y={py(ranges[a.key][1])} width={10} height={py(ranges[a.key][0]) - py(ranges[a.key][1])} fill="rgba(0,102,255,0.18)" stroke={ACCENT} />}
          </g>
        ))}
        {drag && (() => { const j = axes.findIndex((a) => a.key === drag.key); return <rect x={ax(j) - 5} y={py(Math.max(drag.a, drag.b))} width={10} height={Math.abs(py(drag.a) - py(drag.b))} fill="rgba(0,102,255,0.18)" />; })()}
      </svg>
      {axes.map((a, j) => (
        <div key={a.key} className="pcaxis" style={{ left: ax(j) - 9, top: m.t, height: h - m.t - m.b }} title={`${a.label}: drag to filter (better is down)`}
          onPointerDown={(e) => { (e.target as HTMLElement).setPointerCapture(e.pointerId); const t = toT(e.clientY - box.current!.getBoundingClientRect().top); setDrag({ key: a.key, a: t, b: t }); }}
          onPointerMove={(e) => { if (drag?.key === a.key) setDrag({ ...drag, b: toT(e.clientY - box.current!.getBoundingClientRect().top) }); }}
          onPointerUp={() => {
            if (!drag) return;
            const next = { ...ranges };
            if (Math.abs(drag.a - drag.b) < 0.01) delete next[a.key];
            else next[a.key] = [Math.min(drag.a, drag.b), Math.max(drag.a, drag.b)];
            setDrag(null);
            apply(next);
          }} />
      ))}
    </div>
  );
}

/** Stability rows from winners per (metric, factor). */
export function stabilityRows(winners: Uint32Array, ks: number[], defs: MetricDef[], factors: number[], best: number): StabilityRow[] {
  return ks.map((k, a) => ({ k, def: defs[a], ...stabilityRange(winners.subarray(a * factors.length, (a + 1) * factors.length), factors, best) }));
}

