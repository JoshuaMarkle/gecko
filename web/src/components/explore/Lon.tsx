// Explore: local optima network (research/studies/search-space-visualization.md §3.2, 15). Nodes
// are the focused optima (size: times hit ≈ basin size), edges the kicks between them (width:
// how often), colored by funnel (the funnels' bottoms ringed). Layout: force-directed, or
// "Score" puts the score on y so funnels read as trees draining downward. Click picks a layout.

import { useMemo, useState } from "react";
import { ACCENT, HAIR, INK, seriesColor } from "../../colors";
import { fmt } from "../../format";
import { forceLayout, subgraph } from "../../lon";
import { useSize } from "../Charts";

const OTHER = "#c6c6c6";

export function LonGraph({ order, edges, scores, hits, sinks, funnelRank, picked, best, onPick, tooltip }: {
  /** Rows to show, best first. */
  order: ArrayLike<number>; edges: ArrayLike<number>; scores: ArrayLike<number>; hits: ArrayLike<number>;
  sinks: ArrayLike<number>; funnelRank: Map<number, number>; picked: number | null; best: number;
  onPick: (row: number) => void; tooltip: (row: number) => React.ReactNode;
}) {
  const [box, { w, h }] = useSize<HTMLDivElement>();
  const [mode, setMode] = useState<"force" | "score">("force");
  const [hover, setHover] = useState<{ i: number; x: number; y: number } | null>(null);
  const g = useMemo(() => subgraph(order, order.length, edges), [order, edges]);
  const xy = useMemo(() => forceLayout(g.rows.length, g.edges, g.rows.length > 400 ? 150 : 300), [g]);
  const n = g.rows.length;
  const [lo, hi] = useMemo(() => {
    let a = Infinity, b = -Infinity;
    for (const r of g.rows) { a = Math.min(a, scores[r]); b = Math.max(b, scores[r]); }
    return [a, b > a ? b : a + 1e-9];
  }, [g, scores]);
  const m = 14;
  const px = (i: number) => m + xy[2 * i] * (w - 2 * m);
  const py = (i: number) => (mode === "force" ? m + 18 + xy[2 * i + 1] * (h - 2 * m - 18) : m + 18 + (1 - (scores[g.rows[i]] - lo) / (hi - lo)) * (h - 2 * m - 18));
  const color = (r: number) => { const k = funnelRank.get(sinks[r]); return k !== undefined && k < 7 ? seriesColor(k + 1) : OTHER; };
  const maxW = Math.max(1, ...g.edges.map((e) => e.w));
  const funnelsShown = new Set(g.rows.map((r) => funnelRank.get(sinks[r]))).size;
  if (!n) return <div ref={box} className="empty">No optima in focus.</div>;
  return (
    <div ref={box} style={{ height: "100%", position: "relative" }} onPointerLeave={() => setHover(null)}>
      <svg width={w} height={h}>
        <text x={m} y={12} className="ink">{n} optima · {g.edges.length} kick links · {funnelsShown} funnel{funnelsShown === 1 ? "" : "s"}{mode === "score" ? " · y = score, better is down" : ""}</text>
        <foreignObject x={w - 136} y={0} width={136} height={24}>
          <div className="seg inline">{(["force", "score"] as const).map((k) => <button key={k} className={mode === k ? "on" : ""} onClick={() => setMode(k)}
            title={k === "force" ? "Force-directed: linked optima pull together" : "y = score: each funnel drains down to its bottom"}>{k === "force" ? "Force" : "Score"}</button>)}</div>
        </foreignObject>
        {g.edges.map((e, k) => <line key={k} x1={px(e.a)} y1={py(e.a)} x2={px(e.b)} y2={py(e.b)} stroke={INK} strokeOpacity={0.12 + 0.3 * (e.w / maxW)} strokeWidth={0.6 + 1.6 * Math.sqrt(e.w / maxW)} />)}
        {[...g.rows.keys()].reverse().map((i) => {
          const r = g.rows[i], isSink = sinks[r] === r;
          const rad = 2 + Math.min(5, Math.log2(1 + hits[r]));
          return (
            <circle key={r} cx={px(i)} cy={py(i)} r={rad} fill={color(r)} stroke={r === picked ? ACCENT : isSink ? INK : "white"} strokeWidth={r === picked ? 2.5 : isSink ? 1.5 : 0.8}
              style={{ cursor: "pointer" }} onClick={() => onPick(r)}
              onPointerEnter={(e) => { const b = (e.currentTarget.ownerSVGElement as SVGSVGElement).getBoundingClientRect(); setHover({ i: r, x: e.clientX - b.left, y: e.clientY - b.top }); }} />
          );
        })}
        {best >= 0 && g.rows.includes(best) && (() => { const i = g.rows.indexOf(best); return <text x={px(i) + 8} y={py(i) + 4} className="ink">best {fmt(scores[best], 3)}</text>; })()}
        <line x1={0} x2={w} y1={h - 0.5} y2={h - 0.5} stroke={HAIR} />
      </svg>
      {hover && <div className="tip" style={{ left: Math.max(0, Math.min(hover.x + 12, w - 230)), top: Math.max(0, Math.min(hover.y + 8, h - 130)) }}>{tooltip(hover.i)}</div>}
    </div>
  );
}
