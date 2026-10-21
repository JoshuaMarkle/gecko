// Analysis view: every chart for the current layout at once, against the reference.

import type { Analysis, Geometry } from "../api";
import { fmt, verdict } from "../format";
import { FingerLoad, Heatmap, MetricBars, WorstNgrams } from "./Charts";

/** Per-metric value vs reference and weighted contribution, with a better/worse dot. */
export function MetricTable({ a, refA, refName }: { a: Analysis; refA: Analysis | null; refName: string }) {
  return (
    <table className="tbl">
      <thead><tr><th>metric</th><th>value</th><th>{refName}</th><th>score</th><th /></tr></thead>
      <tbody>
        {a.breakdown.metrics.map((m) => {
          const r = refA?.breakdown.metrics.find((x) => x.id === m.id);
          const v = r ? verdict(m.dir, m.value, r.value) : "same";
          return (
            <tr key={m.id} className={m.weight === 0 ? "off" : ""} title={`${m.id}${m.dir === "max" ? " (higher is better)" : m.dir === "info" ? " (info only)" : " (lower is better)"}`}>
              <td>{m.name}</td>
              <td>{fmt(m.value)}{m.unit === "%" ? "%" : ""}</td>
              <td>{r ? `${fmt(r.value)}${m.unit === "%" ? "%" : ""}` : ""}</td>
              <td>{m.weight === 0 ? "–" : fmt(m.weighted, 3)}</td>
              <td>{v !== "same" && m.weight > 0 && <span className={`dot ${v}`} />}</td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}

interface Props {
  geometry: Geometry;
  chars: string;
  label: string;
  a: Analysis | null;
  refA: Analysis | null;
  refName: string;
  selected: number | null;
  onKey: (i: number) => void;
}

export function AnalysisView(p: Props) {
  const a = p.a;
  if (!a) return <div className="empty">Scoring…</div>;
  const refA = p.refA ?? undefined;
  const delta = refA ? a.breakdown.score - refA.breakdown.score : null;
  return (
    <div className="analysis">
      <div className="analysis-head">
        <b>{p.label}</b>
        <span className="score"><b>{fmt(a.breakdown.score, 3)}</b></span>
        {delta !== null && (
          <span className={`badge ${delta < -1e-9 ? "success" : delta > 1e-9 ? "error" : ""}`}>{delta <= 0 ? "" : "+"}{fmt(delta, 3)} vs {p.refName}</span>
        )}
        <span className="hint">lower score is better · click two keys on the heatmap to swap them</span>
      </div>
      <div className="cards">
        <section className="card"><h3>Heatmap</h3><div className="chartbox"><Heatmap geometry={p.geometry} chars={p.chars} a={a} selected={p.selected} onKey={p.onKey} /></div></section>
        <section className="card"><h3>Metrics<span className="hint">value · {p.refName} · contribution</span></h3><div className="cardbody"><MetricTable a={a} refA={p.refA} refName={p.refName} /></div></section>
        <section className="card"><h3>Score contributions</h3><div className="chartbox"><MetricBars cur={a} ref={refA} refName={p.refName} /></div></section>
        <section className="card"><h3>Finger load</h3><div className="chartbox"><FingerLoad a={a} refA={refA} refName={p.refName} /></div></section>
        <section className="card wide"><h3>Worst n-grams</h3><div className="cardbody"><WorstNgrams a={a} /></div></section>
      </div>
    </div>
  );
}
