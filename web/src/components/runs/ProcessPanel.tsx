// Saved view: how a run went, not only its best score (research/studies/search-space-visualization.md
// §3.1, §3.3). Worker convergence (each worker's current score, the best in bold), the schedule
// (temperature, worsening moves accepted, improving share), the symbol-migration raster (slot ×
// time, each symbol in the color of the finger it ends on), settling order and the search
// trajectory network (where each worker went). Data: the `workers` trace in progress.jsonl
// (`process.ts`).

import { useEffect, useMemo, useRef, useState } from "react";
import type { Geometry, Progress } from "../../api";
import { ACCENT, fingerColor, HAIR, INK, MUTED, seriesColor } from "../../colors";
import { fitChars, fmt, fmtTime } from "../../format";
import { forceLayout } from "../../lon";
import { frames, schedule, settling, stn, workerLines, type Frame } from "../../process";
import { Thumb } from "../Board2D";
import { niceTicks, useSize } from "../Charts";

const FAINT = "#adadad";

export function ProcessPanel({ progress, geometry }: { progress: Progress[]; geometry?: Geometry }) {
  const workers = useMemo(() => Math.max(0, ...progress.map((p) => p.workers?.length ?? 0)), [progress]);
  const [src, setSrc] = useState<number | null>(null);
  const fs = useMemo(() => frames(progress, src !== null && src < workers ? src : null), [progress, src, workers]);
  const settle = useMemo(() => settling(frames(progress, null)), [progress]);
  if (progress.length < 2) return null;
  const hasSchedule = progress.some((p) => p.workers?.some((w) => w.temp !== undefined || w.proposed > 0));
  return (
    <details className="sec" open>
      <summary>Process<span className="note">how the search went · {workers ? `${workers} worker${workers === 1 ? "" : "s"}` : "no per-worker trace (older run or population algorithm)"}</span></summary>
      <div className="body">
        <div className="cards">
          {workers > 0 && (
            <section className="card">
              <h3>Workers<span className="hint">thin: each worker's current layout · bold: best so far</span></h3>
              <div className="chartbox"><WorkerChart progress={progress} /></div>
            </section>
          )}
          {hasSchedule && (
            <section className="card">
              <h3>Schedule<span className="hint">temperature, and the share of moves accepted</span></h3>
              <div className="chartbox"><ScheduleChart progress={progress} /></div>
            </section>
          )}
          <section className="card wide">
            <h3>Symbol migration<span className="hint">slot × time · color: the finger the symbol ends on · long bands = settled</span>
              {workers > 0 && (
                <div className="seg inline right">
                  <button className={src === null ? "on" : ""} onClick={() => setSrc(null)} title="The run's best layout so far">Best</button>
                  {Array.from({ length: Math.min(workers, 8) }, (_, k) => (
                    <button key={k} className={src === k ? "on" : ""} onClick={() => setSrc(k)} title={`Worker ${k + 1}'s current layout`}>W{k + 1}</button>
                  ))}
                </div>
              )}
            </h3>
            <div className="chartbox"><Raster frames={fs} geometry={geometry} /></div>
          </section>
          {workers > 0 && (
            <section className="card wide">
              <h3>Trajectory network<span className="hint">locations each worker visited (layouts within the radius merged) · color: worker · ring: best</span></h3>
              <div className="chartbox tall"><StnChart progress={progress} geometry={geometry} /></div>
            </section>
          )}
          <section className="card wide">
            <h3>Settling order<span className="hint">when each symbol last moved in the best layout · late = the objective is flat there</span></h3>
            <div className="cardbody"><SettleBars rows={settle} end={Math.max(...settle.map((r) => r.t), 1)} /></div>
          </section>
        </div>
      </div>
    </details>
  );
}

const M = { l: 48, r: 12, t: 22, b: 24 };

function timeAxis(xMax: number, sx: (t: number) => number, y: number) {
  return niceTicks(0, xMax / 1000, 6).map((t) => <text key={t} x={sx(t * 1000)} y={y} textAnchor="middle">{t}s</text>);
}

function WorkerChart({ progress }: { progress: Progress[] }) {
  const [box, { w, h }] = useSize<HTMLDivElement>();
  const lines = useMemo(() => workerLines(progress), [progress]);
  const xMax = Math.max(progress[progress.length - 1].elapsed_ms, 1);
  const best = progress.map((p) => p.best);
  const cur = lines.flat().map((p) => p[1]).sort((a, b) => a - b);
  const lo = Math.min(...best);
  // Clip the random-start spike: the top is the 95th percentile of current scores.
  const hi = Math.max(cur[Math.floor(cur.length * 0.95)] ?? lo, best[Math.floor(best.length * 0.1)] ?? lo, lo + 1e-6);
  const sx = (t: number) => M.l + (t / xMax) * (w - M.l - M.r);
  const sy = (v: number) => M.t + (1 - (Math.min(v, hi) - lo) / (hi - lo)) * (h - M.t - M.b);
  const path = (pts: [number, number][]) => pts.map(([t, v], i) => `${i ? "L" : "M"}${sx(t)},${sy(v)}`).join("");
  let bp = "";
  progress.forEach((p, i) => { bp += i ? `H${sx(p.elapsed_ms)}V${sy(p.best)}` : `M${sx(p.elapsed_ms)},${sy(p.best)}`; });
  return (
    <div ref={box} style={{ height: "100%" }}>
      <svg width={w} height={h}>
        <text x={M.l} y={12} className="ink">best {fmt(best[best.length - 1], 3)} · {lines.length} worker{lines.length === 1 ? "" : "s"}</text>
        {niceTicks(lo, hi, 4).map((t) => <g key={t}><line x1={M.l} x2={w - M.r} y1={sy(t)} y2={sy(t)} stroke={HAIR} /><text x={M.l - 6} y={sy(t) + 4} textAnchor="end">{fmt(t)}</text></g>)}
        {timeAxis(xMax, sx, h - 6)}
        {lines.map((l, k) => <path key={k} d={path(l)} fill="none" stroke={FAINT} strokeWidth={1} opacity={0.8} />)}
        <path d={bp} fill="none" stroke={INK} strokeWidth={2} />
      </svg>
    </div>
  );
}

function ScheduleChart({ progress }: { progress: Progress[] }) {
  const [box, { w, h }] = useSize<HTMLDivElement>();
  const rates = useMemo(() => schedule(progress), [progress]);
  const xMax = Math.max(progress[progress.length - 1].elapsed_ms, 1);
  const sx = (t: number) => M.l + (t / xMax) * (w - M.l - M.r);
  const mid = M.t + (h - M.t - M.b) * 0.45;
  const temps = rates.flatMap((r) => (r.temp !== null && r.temp > 0 ? [r.temp] : []));
  const [tlo, thi] = temps.length ? [Math.log10(Math.min(...temps)), Math.log10(Math.max(...temps))] : [0, 1];
  const syT = (v: number) => M.t + (1 - (Math.log10(v) - tlo) / (thi - tlo || 1)) * (mid - M.t - 8);
  const syR = (v: number) => mid + 8 + (1 - v) * (h - M.b - mid - 8);
  const line = (f: (r: (typeof rates)[number]) => number | null, sy: (v: number) => number) => {
    let d = "", pen = false;
    for (const r of rates) {
      const v = f(r);
      if (v === null || !(v > 0 || sy === syR)) { pen = false; continue; }
      d += `${pen ? "L" : "M"}${sx(r.t)},${sy(v)}`;
      pen = true;
    }
    return d;
  };
  return (
    <div ref={box} style={{ height: "100%" }}>
      <svg width={w} height={h}>
        <g className="legendrow">
          <line x1={M.l} x2={M.l + 14} y1={9} y2={9} stroke={INK} strokeWidth={2} /><text x={M.l + 18} y={13}>temperature (log)</text>
          <line x1={M.l + 130} x2={M.l + 144} y1={9} y2={9} stroke={ACCENT} strokeWidth={2} /><text x={M.l + 148} y={13}>worse moves accepted</text>
          <line x1={M.l + 290} x2={M.l + 304} y1={9} y2={9} stroke={MUTED} strokeWidth={2} strokeDasharray="4 2" /><text x={M.l + 308} y={13}>improving</text>
        </g>
        {temps.length > 0 && <>
          <text x={M.l - 6} y={syT(10 ** thi) + 4} textAnchor="end">{fmt(10 ** thi, 2)}</text>
          <text x={M.l - 6} y={syT(10 ** tlo) + 4} textAnchor="end">{fmt(10 ** tlo, 3)}</text>
          <path d={line((r) => r.temp, syT)} fill="none" stroke={INK} strokeWidth={1.5} />
        </>}
        {[0, 0.5, 1].map((t) => <g key={t}><line x1={M.l} x2={w - M.r} y1={syR(t)} y2={syR(t)} stroke={HAIR} /><text x={M.l - 6} y={syR(t) + 4} textAnchor="end">{t * 100}%</text></g>)}
        <path d={line((r) => r.accept, syR)} fill="none" stroke={ACCENT} strokeWidth={1.5} />
        <path d={line((r) => r.improving, syR)} fill="none" stroke={MUTED} strokeWidth={1.5} strokeDasharray="4 2" />
        {timeAxis(xMax, sx, h - 6)}
      </svg>
    </div>
  );
}

/** Slot × time matrix of symbols (canvas); hover names the symbol. */
function Raster({ frames: fs, geometry }: { frames: Frame[]; geometry?: Geometry }) {
  const [box, { w, h }] = useSize<HTMLDivElement>();
  const canvas = useRef<HTMLCanvasElement>(null);
  const [hover, setHover] = useState<{ x: number; y: number; f: number; s: number } | null>(null);
  const m = { l: 28, r: 8, t: 6, b: 22 };
  const slots = fs.length ? fs[0].chars.length : 0;
  const t0 = fs[0]?.t ?? 0, tEnd = Math.max(fs[fs.length - 1]?.t ?? 1, t0 + 1);
  const color = useMemo(() => {
    const final = fs[fs.length - 1]?.chars ?? [];
    const c = new Map<string, string>();
    final.forEach((ch, slot) => { const k = geometry?.key[slot]; c.set(ch, k ? fingerColor(k.finger) : FAINT); });
    return c;
  }, [fs, geometry]);
  const sx = (t: number) => m.l + ((t - t0) / (tEnd - t0)) * (w - m.l - m.r);
  const rh = (h - m.t - m.b) / Math.max(slots, 1);
  useEffect(() => {
    const c = canvas.current;
    if (!c) return;
    const dpr = window.devicePixelRatio || 1;
    c.width = Math.round(w * dpr); c.height = Math.round(h * dpr);
    const g = c.getContext("2d")!;
    g.setTransform(dpr, 0, 0, dpr, 0, 0);
    g.clearRect(0, 0, w, h);
    fs.forEach((f, k) => {
      const x0 = sx(f.t), x1 = k + 1 < fs.length ? sx(fs[k + 1].t) : w - m.r;
      f.chars.forEach((ch, s) => {
        g.fillStyle = color.get(ch) ?? FAINT;
        g.fillRect(x0, m.t + s * rh, Math.max(1, x1 - x0), Math.max(1, rh - (rh > 4 ? 1 : 0)));
      });
      // Final layout's letters at the right edge when rows are tall enough.
      if (k === fs.length - 1 && rh >= 9) {
        g.fillStyle = INK; g.font = `${Math.min(11, rh - 1)}px sans-serif`; g.textBaseline = "middle";
        f.chars.forEach((ch, s) => g.fillText(ch, Math.max(x0 + 2, w - m.r - 10), m.t + (s + 0.5) * rh));
      }
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [fs, w, h, color]);
  if (!fs.length) return <div ref={box} className="empty">No layouts recorded.</div>;
  const hv = hover && fs[hover.f];
  return (
    <div ref={box} className="scatter" style={{ height: "100%" }} onPointerLeave={() => setHover(null)}
      onPointerMove={(e) => {
        const r = e.currentTarget.getBoundingClientRect();
        const x = e.clientX - r.left, y = e.clientY - r.top;
        const t = t0 + ((x - m.l) / (w - m.l - m.r)) * (tEnd - t0), s = Math.floor((y - m.t) / rh);
        if (x < m.l || s < 0 || s >= slots) { setHover(null); return; }
        let f = 0;
        while (f + 1 < fs.length && fs[f + 1].t <= t) f++;
        setHover({ x, y, f, s });
      }}>
      <canvas ref={canvas} style={{ width: w, height: h }} />
      <svg className="axes" width={w} height={h}>
        <text x={m.l - 6} y={m.t + 8} textAnchor="end">slot</text>
        {niceTicks(t0 / 1000, tEnd / 1000, 6).map((t) => <text key={t} x={sx(t * 1000)} y={h - 6} textAnchor="middle">{t}s</text>)}
      </svg>
      {hover && hv && (
        <div className="tip" style={{ left: Math.min(hover.x + 12, w - 180), top: Math.max(0, hover.y - 30) }}>
          <b>{hv.chars[hover.s] === " " ? "space" : hv.chars[hover.s]}</b> on slot {hover.s + 1} · {fmtTime(hv.t)}
          <div className="mono">{hv.chars.join("")}</div>
        </div>
      )}
    </div>
  );
}

function SettleBars({ rows, end }: { rows: ReturnType<typeof settling>; end: number }) {
  const [box, { w }] = useSize<HTMLDivElement>();
  if (!rows.length) return <div ref={box} className="empty">No layouts recorded.</div>;
  const n = rows.length, gap = 2;
  const bw = Math.max(6, (w - 40) / n - gap);
  const ht = 120;
  const sy = (t: number) => ht - (t / Math.max(end, 1)) * (ht - 4);
  return (
    <div ref={box}>
      <svg width={w} height={ht + 34}>
        {niceTicks(0, end / 1000, 3).map((t) => <g key={t}><line x1={36} x2={w} y1={sy(t * 1000)} y2={sy(t * 1000)} stroke={HAIR} /><text x={32} y={sy(t * 1000) + 4} textAnchor="end">{t}s</text></g>)}
        {rows.map((r, k) => {
          const x = 40 + k * (bw + gap);
          return (
            <g key={r.sym}>
              <title>{`${r.sym === " " ? "space" : r.sym}: last moved at ${fmtTime(r.t)}, ${r.moves} move${r.moves === 1 ? "" : "s"} in the best layout`}</title>
              <rect x={x} y={sy(r.t)} width={bw} height={ht - sy(r.t)} rx={1} fill={k === 0 ? INK : FAINT} />
              <text x={x + bw / 2} y={ht + 14} textAnchor="middle" className="ink">{r.sym === " " ? "␣" : r.sym}</text>
              {bw >= 14 && <text x={x + bw / 2} y={ht + 28} textAnchor="middle">{r.moves}</text>}
            </g>
          );
        })}
      </svg>
    </div>
  );
}

/** Search trajectory network: force layout of locations, one colored path per worker. */
function StnChart({ progress, geometry }: { progress: Progress[]; geometry?: Geometry }) {
  const [box, { w, h }] = useSize<HTMLDivElement>();
  const [radius, setRadius] = useState(4);
  const [hover, setHover] = useState<{ i: number; x: number; y: number } | null>(null);
  const g = useMemo(() => stn(progress, radius), [progress, radius]);
  const xy = useMemo(() => forceLayout(g.nodes.length, g.edges.map((e) => ({ a: e.a, b: e.b, w: e.w })), 250, 7), [g]);
  const m = 16, top = 30;
  const px = (i: number) => m + xy[2 * i] * (w - 2 * m), py = (i: number) => top + xy[2 * i + 1] * (h - top - m);
  const maxV = Math.max(1, ...g.nodes.map((n) => n.visits));
  const hv = hover && g.nodes[hover.i];
  return (
    <div ref={box} style={{ height: "100%", position: "relative" }} onPointerLeave={() => setHover(null)}>
      <svg width={w} height={h}>
        <text x={m} y={14} className="ink">{g.nodes.length} locations · {g.edges.length} moves between them</text>
        <foreignObject x={w - 230} y={0} width={230} height={26}>
          <div className="row" style={{ display: "flex", gap: 6, alignItems: "center", justifyContent: "flex-end", fontSize: 11 }}>
            <span className="hint" title="Layouts this many differing slots or fewer from a location join it">merge radius</span>
            <input type="range" min={0} max={12} value={radius} onChange={(e) => setRadius(+e.target.value)} style={{ width: 90 }} /><span>{radius}</span>
          </div>
        </foreignObject>
        {g.edges.map((e, k) => (
          <line key={k} x1={px(e.a)} y1={py(e.a)} x2={px(e.b)} y2={py(e.b)} stroke={seriesColor(e.worker)} strokeOpacity={0.55} strokeWidth={1 + Math.min(2, Math.log2(e.w))} />
        ))}
        {g.nodes.map((n, i) => (
          <circle key={i} cx={px(i)} cy={py(i)} r={2.5 + 4 * Math.sqrt(n.visits / maxV)} fill={n.workers.length > 1 ? INK : seriesColor(n.workers[0] ?? 0)}
            stroke={i === g.best ? ACCENT : "white"} strokeWidth={i === g.best ? 2.5 : 0.8}
            onPointerEnter={(e) => { const b = (e.currentTarget.ownerSVGElement as SVGSVGElement).getBoundingClientRect(); setHover({ i, x: e.clientX - b.left, y: e.clientY - b.top }); }} />
        ))}
      </svg>
      {hover && hv && (
        <div className="tip" style={{ left: Math.max(0, Math.min(hover.x + 12, w - 230)), top: Math.max(0, Math.min(hover.y + 8, h - 130)) }}>
          <div><b>{fmt(hv.score, 3)}</b> · {hv.visits} visit{hv.visits === 1 ? "" : "s"} · worker{hv.workers.length > 1 ? "s" : ""} {hv.workers.map((k) => k + 1).join(", ")}</div>
          {geometry ? <Thumb geometry={geometry} chars={fitChars(hv.chars, geometry)} u={14} width={212} /> : <div className="mono">{hv.chars}</div>}
        </div>
      )}
    </div>
  );
}
