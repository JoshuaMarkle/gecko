// Explore right pane: the picked layout — rank, score vs best, provenance in the atlas, metric
// values vs the best layout, and its swap neighbourhood (which hand edits are nearly free).

import type { Geometry, MetricDef } from "../../api";
import { HAIR, INK, MUTED } from "../../colors";
import type { Swap } from "../../explore";
import { fmt, fmtTime, rowsOf } from "../../format";
import { useSize } from "../Charts";

export interface PickedInfo {
  row: number; rank: number; total: number; chars: string; bestChars: string; score: number; best: number;
  hits: number; firstMs: number; constrained: boolean; dBest: number; funnel: number | null; family: number | null;
  /** Active metrics: raw value here and on the best layout, and the weighted contribution. */
  metrics: { def: MetricDef; value: number; bestValue: number; contrib: number }[];
}

function NeighbourChart({ swaps, score }: { swaps: Swap[]; score: number }) {
  const [box, { w }] = useSize<HTMLDivElement>();
  const h = 120;
  const m = { l: 40, r: 8, t: 8, b: 18 };
  const ds = swaps.map((s) => s.delta);
  // Clip the long costly tail so the near-zero region is readable.
  const hi = Math.max(1e-9, ds[Math.floor(ds.length * 0.6)] ?? 0), lo = Math.min(0, ds[0] ?? 0);
  const sx = (k: number) => m.l + (k / Math.max(ds.length - 1, 1)) * (w - m.l - m.r);
  const sy = (v: number) => m.t + (1 - (Math.min(v, hi) - lo) / (hi - lo)) * (h - m.t - m.b);
  const eps = 0.005 * Math.abs(score);
  return (
    <div ref={box}>
      <svg width={w} height={h}>
        <rect x={m.l} y={sy(eps)} width={w - m.l - m.r} height={Math.max(0, sy(lo) - sy(eps))} fill="#f3f3f3" />
        <line x1={m.l} x2={w - m.r} y1={sy(0)} y2={sy(0)} stroke={MUTED} />
        <text x={m.l - 4} y={sy(0) + 4} textAnchor="end">0</text>
        <text x={m.l - 4} y={sy(hi) + 8} textAnchor="end">{fmt(hi)}</text>
        <text x={m.l} y={h - 4}>{ds.length} swaps, cheapest first · shaded: within 0.5% of the score</text>
        <path d={ds.map((d, k) => `${k ? "L" : "M"}${sx(k)},${sy(d)}`).join("")} fill="none" stroke={INK} strokeWidth={2} />
        <line x1={m.l} x2={m.l} y1={m.t} y2={h - m.b} stroke={HAIR} />
      </svg>
    </div>
  );
}

export function Picked({ p, geometry, swaps, onOpen, onCopy, copied }: {
  p: PickedInfo | null; geometry: Geometry; swaps: Swap[] | null; onOpen: () => void; onCopy: () => void; copied: boolean;
}) {
  if (!p) return <div className="empty">Click a point to inspect a layout.</div>;
  const delta = p.score - p.best;
  const eps = 0.005 * Math.abs(p.score);
  const chars = [...p.chars];
  const nearFree = swaps ? swaps.filter((s) => Math.abs(s.delta) <= eps).length : 0;
  const improving = swaps ? swaps.filter((s) => s.delta < -1e-9).length : 0;
  return (
    <>
      <div className="pad">
        <div className="cardhead">
          <b>Layout #{p.rank + 1}</b><span className="muted">of {p.total}</span>
          {p.rank === 0 && <span className="badge best">Best</span>}
          {p.constrained && <span className="badge" title="End of an exclusion descent: the best found while kept ≥ d slots from better layouts">constrained</span>}
        </div>
        <div className="score">
          <b>{fmt(p.score, 3)}</b>
          {p.rank > 0 && <span className="badge error" title="Score difference vs the best layout in the atlas">+{fmt(delta, 3)} vs best</span>}
        </div>
        <div className="layoutrows">{rowsOf(p.chars, geometry).map((r) => [...r].join(" ")).join("\n")}</div>
        <div className="btns">
          <button className="btn primary" onClick={onOpen}>Open in Layout</button>
          <button className="btn soft small" onClick={onCopy}>{copied ? "Copied" : "Copy layout"}</button>
        </div>
      </div>
      <details className="sec" open>
        <summary>In the atlas</summary>
        <div className="body">
          <div className="kv"><span>Distance to best</span><span>{p.dBest} of {chars.length} keys differ</span></div>
          <div className="kv"><span>Descents ending here</span><span>{p.hits}</span></div>
          <div className="kv"><span>First found</span><span>{fmtTime(p.firstMs)}</span></div>
          <div className="kv"><span>Funnel · family</span><span>{p.funnel !== null ? `#${p.funnel + 1}` : "–"} · {p.family !== null ? `F${p.family + 1}` : "not in focus"}</span></div>
        </div>
      </details>
      <details className="sec" open>
        <summary>Metrics<span className="note">value · best · contribution</span></summary>
        <div className="body">
          <table className="tbl">
            <thead><tr><th>metric</th><th>value</th><th>best</th><th>score</th></tr></thead>
            <tbody>
              {p.metrics.map((m) => (
                <tr key={m.def.id} title={`${m.def.id}: ${m.def.desc}`}>
                  <td>{m.def.name}</td>
                  <td>{fmt(m.value)}{m.def.unit === "%" ? "%" : ""}</td>
                  <td className="muted">{fmt(m.bestValue)}{m.def.unit === "%" ? "%" : ""}</td>
                  <td>{fmt(m.contrib, 3)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </details>
      <details className="sec" open>
        <summary>Neighbourhood<span className="note">every single swap</span></summary>
        <div className="body">
          {!swaps ? <div className="hint">Measuring…</div> : (
            <>
              <NeighbourChart swaps={swaps} score={p.score} />
              <div className="hint">{improving > 0 ? `${improving} improving swaps (not a local optimum under these weights) · ` : ""}{nearFree} near-free swaps (|Δ| ≤ 0.5%)</div>
              <table className="tbl">
                <thead><tr><th>swap</th><th>Δ score</th></tr></thead>
                <tbody>
                  {swaps.slice(0, 8).map((s) => (
                    <tr key={`${s.i}-${s.j}`}><td className="mono">{chars[s.i].toUpperCase()} ↔ {chars[s.j].toUpperCase()}</td><td>{s.delta > 0 ? "+" : ""}{fmt(s.delta, 3)}</td></tr>
                  ))}
                </tbody>
              </table>
            </>
          )}
        </div>
      </details>
    </>
  );
}
