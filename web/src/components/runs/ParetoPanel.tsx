// Saved view, NSGA-II / RWSA runs: the Pareto front as a scatter over any two metrics (the
// line joins the layouts still non-dominated in that plane). Click a point to make it the
// working model; the picked layout's keyboard shows below.

import { useMemo, useState } from "react";
import type { Geometry, Objective, ParetoRanked } from "../../api";
import { ACCENT, INK } from "../../colors";
import { paretoFront } from "../../explore";
import { fitChars, fmt } from "../../format";
import { metricColumn, plotMetrics } from "../../runViews";
import { Thumb } from "../Board2D";
import { Scatter } from "../explore/Scatter";

interface Props {
  front: ParetoRanked[];
  objectives: Objective[];
  geometry?: Geometry;
  picked: number | null;
  onPick: (i: number) => void;
}

export function ParetoPanel({ front, objectives, geometry, picked, onPick }: Props) {
  const metrics = useMemo(() => plotMetrics(front), [front]);
  const [tx, setTx] = useState(objectives[0]?.id ?? metrics[0]?.id ?? "");
  const [ty, setTy] = useState(objectives[1]?.id ?? metrics[1]?.id ?? "");
  const dx = metrics.find((m) => m.id === tx), dy = metrics.find((m) => m.id === ty);
  const xs = useMemo(() => metricColumn(front, tx), [front, tx]);
  const ys = useMemo(() => metricColumn(front, ty), [front, ty]);
  const all = useMemo(() => Uint32Array.from(front, (_, i) => i), [front]);
  const line = useMemo(() => paretoFront(xs, ys, all, dx?.dir !== "max", dy?.dir !== "max"), [xs, ys, all, dx, dy]);
  const best = front.reduce((b, f, i) => (f.score < front[b].score ? i : b), 0);
  const label = (m: typeof dx, id: string) => `${m?.name ?? id} ${m?.dir === "max" ? "(higher is better)" : "(lower is better)"}`;
  const pick = picked !== null ? front[picked] : null;
  const select = (value: string, set: (v: string) => void) => (
    <select value={value} onChange={(e) => set(e.target.value)}>
      {metrics.map((m) => <option key={m.id} value={m.id}>{m.name}{objectives.some((o) => o.id === m.id) ? " ★" : ""}</option>)}
    </select>
  );
  return (
    <details className="sec" open>
      <summary>Pareto front<span className="note">{front.length} layouts no other beats on every objective (★) · click one to make it the working model</span></summary>
      <div className="body">
        <div className="row" style={{ display: "flex", gap: 6, alignItems: "center" }}>
          <label>x</label>{select(tx, setTx)}<label>y</label>{select(ty, setTy)}
          <span className="hint">line: non-dominated in this plane</span>
        </div>
        <div className="chartbox tall">
          <Scatter n={front.length} xs={xs} ys={ys} tone={(i) => (i === picked ? ACCENT : INK)} order={all} radius={() => 3} picked={picked} best={best}
            line={line} xLabel={label(dx, tx)} yLabel={label(dy, ty)} onPick={onPick} onBrush={() => {}}
            tooltip={(i) => (
              <div>
                <b>#{front[i].rank}</b> score {fmt(front[i].score, 3)} · {dx?.name ?? tx} {fmt(xs[i])} · {dy?.name ?? ty} {fmt(ys[i])}
                {geometry && <Thumb geometry={geometry} chars={fitChars(front[i].chars, geometry)} u={13} />}
              </div>
            )} />
        </div>
        {pick && (
          <div style={{ marginTop: 8 }}>
            <div className="kv"><span>Picked</span><span>#{pick.rank} · score {fmt(pick.score, 3)} · {objectives.map((o, k) => `${o.id} ${fmt(pick.raw[k])}`).join(" · ")}</span></div>
            {geometry ? <Thumb geometry={geometry} chars={fitChars(pick.chars, geometry)} u={17} /> : <span className="mono">{pick.rows.join(" ")}</span>}
          </div>
        )}
      </div>
    </details>
  );
}
