// Saved view, MAP-Elites (qd) runs: the archive as a grid over two behaviour axes, each cell
// shaded by its elite's score (darker = better; with more than two axes a cell shows the best
// elite projecting onto it). Hover shows the elite, click makes it the working model.

import { useEffect, useMemo, useState } from "react";
import { api, type Geometry, type QdElite, type QdSummary } from "../../api";
import { ACCENT, heatColor } from "../../colors";
import { fitChars, fmt, fmtCount } from "../../format";
import { binEdge, eliteGrid } from "../../runViews";
import { Thumb } from "../Board2D";
import { useSize } from "../Charts";

interface Props {
  runId: string;
  geometry?: Geometry;
  /** Chars of the working model, outlined when it is an elite. */
  current?: string;
  onPick: (e: QdElite) => void;
}

export function EliteGrid({ runId, geometry, current, onPick }: Props) {
  const [qd, setQd] = useState<QdSummary | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [ax, setAx] = useState(0);
  const [ay, setAy] = useState(1);
  const [hover, setHover] = useState<number | null>(null);
  const [box, size] = useSize<HTMLDivElement>();
  useEffect(() => {
    let live = true;
    setQd(null);
    setErr(null);
    api.qd(runId).then((q) => { if (live) { setQd(q); setAx(0); setAy(q.axes.length > 1 ? 1 : -1); } }, (e: Error) => live && setErr(e.message));
    return () => { live = false; };
  }, [runId]);
  const grid = useMemo(() => (qd ? eliteGrid(qd.elites, qd.axes, ax, ay) : null), [qd, ax, ay]);
  if (err) return <div className="notice error">{err}</div>;
  if (!qd || !grid) return <div className="empty">Loading the MAP-Elites grid…</div>;
  const m = { l: 56, r: 8, t: 8, b: 34 };
  const cell = Math.max(6, Math.min(28, Math.floor((size.w - m.l - m.r) / grid.w)));
  const w = m.l + m.r + cell * grid.w, h = m.t + m.b + cell * grid.h;
  const t = (s: number) => (grid.hi > grid.lo ? (grid.hi - s) / (grid.hi - grid.lo) : 1);
  const shown = hover ?? (current ? qd.elites.findIndex((e) => e.chars === current) : -1);
  const el = shown !== null && shown >= 0 ? qd.elites[shown] : null;
  const xAxis = qd.axes[ax], yAxis = ay >= 0 ? qd.axes[ay] : null;
  const axisSelect = (v: number, set: (i: number) => void, none: boolean) => (
    <select value={v} onChange={(e) => set(+e.target.value)}>
      {none && <option value={-1}>(none)</option>}
      {qd.axes.map((a, i) => <option key={a.name} value={i}>{a.name}</option>)}
    </select>
  );
  return (
    <details className="sec" open>
      <summary>MAP-Elites grid<span className="note">{fmtCount(qd.filled)} of {fmtCount(qd.cells)} cells ({(100 * qd.coverage).toFixed(0)}%) · darker = better score · click a cell to make it the working model</span></summary>
      <div className="body">
        <div className="row" style={{ display: "flex", gap: 6, alignItems: "center" }}>
          <label>x</label>{axisSelect(ax, setAx, false)}<label>y</label>{axisSelect(ay, setAy, true)}
          <span className="hint">scores {fmt(grid.lo, 3)} – {fmt(grid.hi, 3)} · QD-score {fmt(qd.qd_score, 1)}</span>
        </div>
        <div ref={box} style={{ width: "100%" }} onMouseLeave={() => setHover(null)}>
          <svg width={w} height={h}>
            {grid.cells.map((k, i) => {
              const x = i % grid.w, y = Math.floor(i / grid.w);
              const px = m.l + x * cell, py = m.t + (grid.h - 1 - y) * cell;
              if (k === null) return <rect key={i} x={px} y={py} width={cell - 1} height={cell - 1} fill="none" stroke="var(--hair)" />;
              const e = qd.elites[k];
              return <rect key={i} x={px} y={py} width={cell - 1} height={cell - 1} fill={heatColor(0.15 + 0.85 * t(e.score))}
                stroke={k === shown ? ACCENT : "none"} strokeWidth={2} style={{ cursor: "pointer" }}
                onMouseEnter={() => setHover(k)} onClick={() => onPick(e)} />;
            })}
            <text x={m.l} y={h - 18}>{fmt(binEdge(xAxis, 0))}</text>
            <text x={m.l + cell * grid.w} y={h - 18} textAnchor="end">{fmt(binEdge(xAxis, grid.w))}</text>
            <text x={m.l + (cell * grid.w) / 2} y={h - 4} textAnchor="middle" className="ink">{xAxis.name}</text>
            {yAxis && <>
              <text x={m.l - 4} y={m.t + cell * grid.h} textAnchor="end">{fmt(binEdge(yAxis, 0))}</text>
              <text x={m.l - 4} y={m.t + 10} textAnchor="end">{fmt(binEdge(yAxis, grid.h))}</text>
              <text x={12} y={m.t + (cell * grid.h) / 2} textAnchor="middle" className="ink" transform={`rotate(-90 12 ${m.t + (cell * grid.h) / 2})`}>{yAxis.name}</text>
            </>}
          </svg>
        </div>
        {el ? (
          <div style={{ marginTop: 8 }}>
            <div className="kv"><span>{hover !== null ? "Cell" : "Working model"}</span>
              <span>score {fmt(el.score, 3)} · {qd.axes.map((a, i) => `${a.name} ${fmt(el.desc[i])}`).join(" · ")} · replaced {el.updates}×</span></div>
            {geometry ? <Thumb geometry={geometry} chars={fitChars(el.chars, geometry)} u={17} /> : <span className="mono">{el.chars}</span>}
          </div>
        ) : <div className="hint" style={{ marginTop: 8 }}>Hover a cell to see its layout.</div>}
      </div>
    </details>
  );
}
