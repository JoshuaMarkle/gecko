// Explore: path between two layouts (research/studies/search-space-visualization.md §3.3, 31).
// From the path start (the best, or any layout set as start) to the picked layout, each step the
// cheapest swap that puts one more symbol where the target has it (gecko-core `explore::path`).
// The score profile shows the barrier between the two designs; hover or drag a step to see the
// layout halfway (grey keys: not yet as in the target).

import { useMemo, useState } from "react";
import type { Geometry, PathResult } from "../../api";
import { ACCENT, HAIR, INK, MUTED } from "../../colors";
import { fmt } from "../../format";
import { niceTicks, useSize } from "../Charts";
import { Thumb } from "./Board";

/** The layout after each step: index 0 is the start. */
export function pathLayouts(from: string, path: PathResult): string[] {
  const cur = [...from];
  const out = [from];
  for (const s of path.steps) { [cur[s.a], cur[s.b]] = [cur[s.b], cur[s.a]]; out.push(cur.join("")); }
  return out;
}

export function PathPanel({ geometry, from, to, fromLabel, toLabel, path, error }: {
  geometry: Geometry; from: string; to: string; fromLabel: string; toLabel: string; path: PathResult | null; error: string | null;
}) {
  const [box, { w }] = useSize<HTMLDivElement>();
  const [step, setStep] = useState<number | null>(null);
  const layouts = useMemo(() => (path ? pathLayouts(from, path) : []), [from, path]);
  if (error) return <div className="notice error">{error}</div>;
  if (from === to) return <div className="empty">Pick a layout other than the path start.</div>;
  if (!path) return <div ref={box} className="empty">Computing the path…</div>;
  const scores = [path.start, ...path.steps.map((s) => s.score)];
  const n = scores.length - 1;
  const h = 150, m = { l: 46, r: 12, t: 18, b: 22 };
  const lo = Math.min(...scores), hi = Math.max(...scores, lo + 1e-9);
  const sx = (k: number) => m.l + (k / Math.max(n, 1)) * (w - m.l - m.r);
  const sy = (v: number) => m.t + (1 - (v - lo) / (hi - lo)) * (h - m.t - m.b);
  const peak = scores.indexOf(Math.max(...scores));
  const shown = step ?? peak;
  const pick = (e: React.PointerEvent<SVGSVGElement>) => {
    const r = e.currentTarget.getBoundingClientRect();
    setStep(Math.max(0, Math.min(n, Math.round(((e.clientX - r.left - m.l) / (w - m.l - m.r)) * n))));
  };
  const swap = shown > 0 ? path.steps[shown - 1] : null;
  return (
    <div ref={box} className="pathview">
      <svg width={w} height={h} onPointerMove={pick} onPointerDown={pick} style={{ touchAction: "none" }}>
        <text x={m.l} y={11} className="ink">{n} swaps · barrier {path.barrier > 1e-9 ? `+${fmt(path.barrier, 3)} above the worse end` : "none: downhill or flat all the way"}</text>
        {niceTicks(lo, hi, 3).map((t) => <g key={t}><line x1={m.l} x2={w - m.r} y1={sy(t)} y2={sy(t)} stroke={HAIR} /><text x={m.l - 6} y={sy(t) + 4} textAnchor="end">{fmt(t)}</text></g>)}
        <text x={m.l} y={h - 6}>{fromLabel}</text>
        <text x={w - m.r} y={h - 6} textAnchor="end">{toLabel}</text>
        <path d={scores.map((v, k) => `${k ? "L" : "M"}${sx(k)},${sy(v)}`).join("")} fill="none" stroke={INK} strokeWidth={2} strokeLinejoin="round" />
        {scores.map((v, k) => <circle key={k} cx={sx(k)} cy={sy(v)} r={k === shown ? 4 : 2} fill={k === shown ? ACCENT : INK} />)}
        <line x1={sx(shown)} x2={sx(shown)} y1={m.t} y2={h - m.b} stroke={MUTED} strokeDasharray="3 3" />
      </svg>
      <div className="hint">
        Step {shown}/{n} · {fmt(scores[shown], 3)} ({scores[shown] - scores[0] >= 0 ? "+" : ""}{fmt(scores[shown] - scores[0], 3)} vs start)
        {swap && <> · swapped <b className="mono">{layouts[shown][swap.a]}</b> ↔ <b className="mono">{layouts[shown][swap.b]}</b></>}
      </div>
      <Thumb geometry={geometry} chars={layouts[shown]} diff={to} width={Math.min(w, 420)} />
    </div>
  );
}
