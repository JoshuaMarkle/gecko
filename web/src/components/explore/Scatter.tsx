// Canvas scatter for dense point clouds (10⁴–10⁵ optima): hover tooltip, click to pick,
// drag to brush a rectangle, double-click to clear. Used by the map, fitness–distance and
// trade-off panels. SVG would not scale to this many points.

import { useEffect, useRef, useState, type ReactNode } from "react";
import { ACCENT, HAIR, INK, MUTED } from "../../colors";
import { fmt } from "../../format";
import { niceTicks, useSize } from "../Charts";

export interface Mark { x: number; y: number; label: string }

interface Props {
  n: number;
  xs: ArrayLike<number>;
  ys: ArrayLike<number>;
  /** Fill per row; null hides the row. */
  tone: (i: number) => string | null;
  /** Draw order (later rows on top). */
  order: ArrayLike<number>;
  radius?: (i: number) => number;
  picked: number | null;
  best: number | null;
  /** Rows to connect with a line (e.g. a Pareto front), in order. */
  line?: number[];
  marks?: Mark[];
  xLabel?: string;
  yLabel?: string;
  /** Map mode: equal scale on both axes, no tick labels (PCA axes have no units). */
  map?: boolean;
  onPick: (i: number) => void;
  onBrush: (rows: number[] | null) => void;
  tooltip: (i: number) => ReactNode;
}

interface View { x0: number; x1: number; y0: number; y1: number; sx: (x: number) => number; sy: (y: number) => number; l: number; t: number; w: number; h: number }

export function Scatter(p: Props) {
  const [box, size] = useSize<HTMLDivElement>();
  const canvas = useRef<HTMLCanvasElement>(null);
  const view = useRef<View | null>(null);
  const [hover, setHover] = useState<{ i: number; px: number; py: number } | null>(null);
  const [rect, setRect] = useState<{ ax: number; ay: number; bx: number; by: number } | null>(null);
  const w = size.w, h = size.h;
  const m = p.map ? { l: 8, r: 8, t: 8, b: 8 } : { l: 46, r: 12, t: 10, b: 30 };

  // Bounds over visible rows (marks outside are drawn at the edge, not allowed to squash the cloud).
  let x0 = Infinity, x1 = -Infinity, y0 = Infinity, y1 = -Infinity;
  for (let k = 0; k < p.order.length; k++) {
    const i = p.order[k];
    const x = p.xs[i], y = p.ys[i];
    if (!Number.isFinite(x) || !Number.isFinite(y)) continue;
    if (x < x0) x0 = x; if (x > x1) x1 = x; if (y < y0) y0 = y; if (y > y1) y1 = y;
  }
  if (!(x1 >= x0)) { x0 = 0; x1 = 1; y0 = 0; y1 = 1; }
  const padX = (x1 - x0 || 1) * 0.04, padY = (y1 - y0 || 1) * 0.06;
  x0 -= padX; x1 += padX; y0 -= padY; y1 += padY;
  const pw = w - m.l - m.r, ph = h - m.t - m.b;
  let kx = pw / (x1 - x0), ky = ph / (y1 - y0);
  if (p.map) {
    const k = Math.min(kx, ky);
    kx = ky = k;
  }
  const ox = m.l + (pw - kx * (x1 - x0)) / 2, oy = m.t + (ph - ky * (y1 - y0)) / 2;
  const sx = (x: number) => ox + (x - x0) * kx;
  const sy = (y: number) => oy + ky * (y1 - y0) - (y - y0) * ky;

  useEffect(() => {
    const c = canvas.current;
    if (!c) return;
    const dpr = window.devicePixelRatio || 1;
    c.width = Math.round(w * dpr);
    c.height = Math.round(h * dpr);
    const g = c.getContext("2d")!;
    g.setTransform(dpr, 0, 0, dpr, 0, 0);
    g.clearRect(0, 0, w, h);
    view.current = { x0, x1, y0, y1, sx, sy, l: m.l, t: m.t, w, h };
    if (!p.map) {
      g.strokeStyle = HAIR;
      g.lineWidth = 1;
      for (const t of niceTicks(y0, y1)) { g.beginPath(); g.moveTo(m.l, Math.round(sy(t)) + 0.5); g.lineTo(w - m.r, Math.round(sy(t)) + 0.5); g.stroke(); }
    }
    let last = "";
    for (let k = 0; k < p.order.length; k++) {
      const i = p.order[k];
      const col = p.tone(i);
      const x = p.xs[i], y = p.ys[i];
      if (!col || !Number.isFinite(x) || !Number.isFinite(y)) continue;
      if (col !== last) { g.fillStyle = col; last = col; }
      const r = p.radius ? p.radius(i) : 2;
      if (r <= 1.5) g.fillRect(sx(x) - r, sy(y) - r, 2 * r, 2 * r);
      else { g.beginPath(); g.arc(sx(x), sy(y), r, 0, 2 * Math.PI); g.fill(); }
    }
    if (p.line && p.line.length > 1) {
      g.strokeStyle = INK;
      g.lineWidth = 1.5;
      g.beginPath();
      p.line.forEach((i, k) => (k === 0 ? g.moveTo(sx(p.xs[i]), sy(p.ys[i])) : g.lineTo(sx(p.xs[i]), sy(p.ys[i]))));
      g.stroke();
    }
    // Best and picked: accent with a 2px surface ring.
    for (const i of [p.best, p.picked]) {
      if (i === null || i === undefined || !Number.isFinite(p.xs[i])) continue;
      g.beginPath(); g.arc(sx(p.xs[i]), sy(p.ys[i]), i === p.picked ? 6 : 4.5, 0, 2 * Math.PI);
      g.fillStyle = "#ffffff"; g.fill();
      g.beginPath(); g.arc(sx(p.xs[i]), sy(p.ys[i]), i === p.picked ? 4 : 3, 0, 2 * Math.PI);
      g.fillStyle = i === p.picked ? ACCENT : INK; g.fill();
      if (i === p.picked) { g.strokeStyle = ACCENT; g.lineWidth = 1.5; g.beginPath(); g.arc(sx(p.xs[i]), sy(p.ys[i]), 8, 0, 2 * Math.PI); g.stroke(); }
    }
    g.font = "450 11px Inter, -apple-system, sans-serif";
    // Marks inside the plot get a cross and a label; marks off the plot are listed in the
    // top-left corner (under the y label) instead of piling up on the edge.
    const off: Mark[] = [];
    for (const mk of p.marks ?? []) {
      const cx = sx(mk.x), cy = sy(mk.y);
      if (cx < m.l || cx > w - m.r || cy < m.t || cy > h - m.b) { off.push(mk); continue; }
      g.strokeStyle = MUTED; g.lineWidth = 1.5;
      g.beginPath(); g.moveTo(cx - 4, cy - 4); g.lineTo(cx + 4, cy + 4); g.moveTo(cx + 4, cy - 4); g.lineTo(cx - 4, cy + 4); g.stroke();
      g.fillStyle = INK;
      const tw = g.measureText(mk.label).width;
      g.fillText(mk.label, cx + 7 + tw > w - m.r ? cx - 7 - tw : cx + 7, cy + 4);
    }
    g.fillStyle = MUTED;
    off.sort((a, b) => a.y - b.y).forEach((mk, k) => g.fillText(`${mk.label} ${fmt(mk.y)} (off chart)`, m.l + 6, m.t + 30 + 14 * k));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [p.n, p.xs, p.ys, p.tone, p.order, p.radius, p.picked, p.best, p.line, p.marks, w, h, p.map]);

  const nearest = (px: number, py: number, maxD = 10): number | null => {
    let best: number | null = null, bd = maxD * maxD;
    for (let k = 0; k < p.order.length; k++) {
      const i = p.order[k];
      if (!p.tone(i)) continue;
      const dx = sx(p.xs[i]) - px, dy = sy(p.ys[i]) - py;
      const d = dx * dx + dy * dy;
      if (d <= bd) { bd = d; best = i; }
    }
    return best;
  };
  const local = (e: React.PointerEvent) => {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    return [e.clientX - r.left, e.clientY - r.top];
  };

  return (
    <div ref={box} className="scatter" style={{ height: "100%" }}>
      <canvas ref={canvas} style={{ width: w, height: h }}
        onPointerDown={(e) => { const [x, y] = local(e); (e.target as HTMLElement).setPointerCapture(e.pointerId); setRect({ ax: x, ay: y, bx: x, by: y }); }}
        onPointerMove={(e) => {
          const [x, y] = local(e);
          if (rect && e.buttons) { setRect({ ...rect, bx: x, by: y }); setHover(null); return; }
          const i = nearest(x, y);
          setHover(i === null ? null : { i, px: x, py: y });
        }}
        onPointerLeave={() => setHover(null)}
        onPointerUp={(e) => {
          const [x, y] = local(e);
          if (rect && (Math.abs(rect.ax - x) > 4 || Math.abs(rect.ay - y) > 4)) {
            const [ax, bx] = [Math.min(rect.ax, x), Math.max(rect.ax, x)], [ay, by] = [Math.min(rect.ay, y), Math.max(rect.ay, y)];
            const rows: number[] = [];
            for (let k = 0; k < p.order.length; k++) {
              const i = p.order[k];
              if (!p.tone(i)) continue;
              const px = sx(p.xs[i]), py = sy(p.ys[i]);
              if (px >= ax && px <= bx && py >= ay && py <= by) rows.push(i);
            }
            p.onBrush(rows.length ? rows : null);
          } else {
            const i = nearest(x, y);
            if (i !== null) p.onPick(i);
          }
          setRect(null);
        }}
        onDoubleClick={() => p.onBrush(null)} />
      {!p.map && (
        <svg className="axes" width={w} height={h}>
          {niceTicks(y0, y1).map((t) => <text key={`y${t}`} x={m.l - 6} y={sy(t) + 4} textAnchor="end">{fmt(t)}</text>)}
          {niceTicks(x0, x1, 6).map((t) => <text key={`x${t}`} x={sx(t)} y={h - 14} textAnchor="middle">{fmt(t)}</text>)}
          {p.xLabel && <text x={w - m.r} y={h - 2} textAnchor="end">{p.xLabel}</text>}
          {p.yLabel && <text x={m.l} y={m.t + 2} textAnchor="start" className="ink">{p.yLabel}</text>}
        </svg>
      )}
      {rect && <div className="brushrect" style={{ left: Math.min(rect.ax, rect.bx), top: Math.min(rect.ay, rect.by), width: Math.abs(rect.bx - rect.ax), height: Math.abs(rect.by - rect.ay) }} />}
      {hover && <div className="tip" style={{ left: Math.min(hover.px + 12, w - 230), top: Math.max(0, hover.py - 10) }}>{p.tooltip(hover.i)}</div>}
    </div>
  );
}
