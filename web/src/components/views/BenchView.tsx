// Bench view (Saved › Bench): compare algorithms from saved `gecko bench` files (runs/bench/,
// `GET /api/bench`). Left: every saved bench. Right: the leaderboard, performance profiles
// (share of seeds within τ of the best solver), time-to-target ECDFs and each seed's best-so-far.
// research/studies/search-space-visualization.md §3.5; helpers in bench.ts.

import { useEffect, useMemo, useState } from "react";
import { api, type Bench } from "../../api";
import { HAIR, MUTED, seriesColor } from "../../colors";
import { ecdf, instanceBest, profileGaps, solverName, ttt } from "../../bench";
import { fmt, fmtTime } from "../../format";
import { usePref } from "../../inputs";
import { niceTicks, useSize } from "../Charts";

const TARGETS: [number, string][] = [[0, "best"], [0.005, "+0.5%"], [0.01, "+1%"], [0.02, "+2%"]];

export function BenchView() {
  const [list, setList] = useState<Bench[] | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [id, setId] = usePref<string>("bench.id", "");
  const [bench, setBench] = useState<Bench | null>(null);
  const [target, setTarget] = usePref("bench.target", 0.005);

  useEffect(() => { api.benches().then(setList).catch((e) => setErr((e as Error).message)); }, []);
  const sel = list?.some((b) => b.id === id) ? id : list?.[0]?.id ?? "";
  useEffect(() => {
    if (!sel) { setBench(null); return; }
    let alive = true;
    api.bench(sel).then((b) => alive && setBench(b)).catch((e) => alive && setErr((e as Error).message));
    return () => { alive = false; };
  }, [sel]);

  return (
    <div className="results">
      <section className="pane">
        <div className="panehead"><b>Benches</b><span className="hint">{list?.length ?? 0} saved in runs/bench/</span></div>
        <div className="scroll"><div className="pad">
          {err && <div className="notice error">{err}</div>}
          <table className="tbl">
            <thead><tr><th>created</th><th>solvers</th><th>seeds</th><th>time</th><th>best</th></tr></thead>
            <tbody>
              {(list ?? []).map((b) => (
                <tr key={b.id} className={`click ${b.id === sel ? "sel" : ""}`} onClick={() => setId(b.id)} title={b.id}>
                  <td className="l">{b.created.replace("T", " ").replace("Z", "")}<div className="hint">{b.geometry} · {b.corpus} · {b.profile}</div></td>
                  <td>{b.results.map(solverName).join(", ")}</td>
                  <td>{b.seeds}</td>
                  <td>{fmtTime(b.time_ms)}</td>
                  <td>{fmt(Math.min(...b.results.map((r) => r.best)), 3)}</td>
                </tr>
              ))}
              {list && list.length === 0 && <tr><td className="muted" colSpan={5}>No benches yet. Run <span className="mono">gecko bench --algos sa,bls,tabu --seeds 5 --time 10s</span>.</td></tr>}
            </tbody>
          </table>
        </div></div>
      </section>
      <section className="pane">
        {!bench ? <div className="empty">{list === null ? "Loading…" : "Select a bench."}</div> : <BenchDetail b={bench} target={target} setTarget={setTarget} />}
      </section>
    </div>
  );
}

function BenchDetail({ b, target, setTarget }: { b: Bench; target: number; setTarget: (t: number) => void }) {
  const best = Math.min(...b.results.map((r) => r.best));
  const order = b.results.map((_, i) => i).sort((x, y) => b.results[x].mean - b.results[y].mean);
  return (
    <>
      <div className="panehead"><b>Bench</b><span className="hint mono">{b.id}</span></div>
      <div className="scroll"><div className="pad">
        <div className="hint">{b.geometry} · {b.corpus} · {b.profile} · {b.seeds} seeds × {fmtTime(b.time_ms)} · {b.threads} thread{b.threads === 1 ? "" : "s"} · commit {b.commit}</div>
        <table className="tbl">
          <thead><tr><th style={{ textAlign: "left" }}>solver</th><th>mean</th><th>best</th><th>stdev</th><th title="Seeds that reached the best score of any solver">hit best</th><th>M evals/s</th></tr></thead>
          <tbody>
            {order.map((i) => {
              const r = b.results[i];
              const hits = r.scores.filter((s) => s <= best + 1e-9).length;
              return (
                <tr key={i}>
                  <td className="l"><span className="swatch" style={{ background: seriesColor(i) }} /> {solverName(r)}</td>
                  <td>{fmt(r.mean, 4)}</td><td>{fmt(r.best, 4)}</td><td>{fmt(r.stdev, 4)}</td>
                  <td>{hits}/{r.scores.length}</td><td>{(r.evals_per_sec / 1e6).toFixed(2)}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
        <div className="cards" style={{ marginTop: 12 }}>
          <section className="card">
            <h3>Performance profile<span className="hint">share of seeds within τ of the best solver on that seed</span></h3>
            <div className="chartbox"><ProfileChart b={b} /></div>
          </section>
          <section className="card">
            <h3>Time to target<span className="hint">share of seeds at or below the target by time t</span>
              <div className="seg inline right">{TARGETS.map(([t, l]) => <button key={l} className={target === t ? "on" : ""} onClick={() => setTarget(t)} title={`Target: overall best ${l === "best" ? "" : l}`}>{l}</button>)}</div></h3>
            <div className="chartbox"><TttChart b={b} target={best + target * Math.abs(best)} /></div>
          </section>
          <section className="card wide">
            <h3>Best so far<span className="hint">one line per seed · dashed: target</span></h3>
            <div className="chartbox"><TraceChart b={b} target={best + target * Math.abs(best)} /></div>
          </section>
        </div>
      </div></div>
    </>
  );
}

const M = { l: 46, r: 12, t: 22, b: 26 };

function Legend({ b, w }: { b: Bench; w: number }) {
  let x = M.l;
  return (
    <g className="legendrow">
      {b.results.map((r, i) => {
        const name = solverName(r), x0 = x;
        x += 26 + name.length * 6.2;
        return x0 + 20 > w ? null : <g key={i}><line x1={x0} x2={x0 + 14} y1={9} y2={9} stroke={seriesColor(i)} strokeWidth={2} /><text x={x0 + 18} y={13}>{name}</text></g>;
      })}
    </g>
  );
}

/** Step ECDF path of sorted values over [0, xMax] (x mapped by `sx`). */
function stepPath(sorted: number[], xMax: number, sx: (x: number) => number, sy: (y: number) => number) {
  let d = `M${sx(0)},${sy(ecdf(sorted, 0))}`;
  sorted.forEach((v) => { if (Number.isFinite(v) && v <= xMax) d += `H${sx(v)}V${sy(ecdf(sorted, v))}`; });
  return d + `H${sx(xMax)}`;
}

function ProfileChart({ b }: { b: Bench }) {
  const [box, { w, h }] = useSize<HTMLDivElement>();
  const gaps = useMemo(() => profileGaps(b), [b]);
  const finite = gaps.flat().filter(Number.isFinite);
  const xMax = Math.max(...finite, 1e-4) * 1.05;
  const sx = (x: number) => M.l + (x / xMax) * (w - M.l - M.r);
  const sy = (y: number) => M.t + (1 - y) * (h - M.t - M.b);
  return (
    <div ref={box} style={{ height: "100%" }}>
      <svg width={w} height={h}>
        <Legend b={b} w={w} />
        {[0, 0.5, 1].map((t) => <g key={t}><line x1={M.l} x2={w - M.r} y1={sy(t)} y2={sy(t)} stroke={HAIR} /><text x={M.l - 6} y={sy(t) + 4} textAnchor="end">{t * 100}%</text></g>)}
        {niceTicks(0, xMax * 100, 5).map((t) => <text key={t} x={sx(t / 100)} y={h - 8} textAnchor="middle">τ {t}%</text>)}
        {gaps.map((g, i) => <path key={i} d={stepPath(g, xMax, sx, sy)} fill="none" stroke={seriesColor(i)} strokeWidth={2} opacity={0.9} />)}
      </svg>
    </div>
  );
}

function TttChart({ b, target }: { b: Bench; target: number }) {
  const [box, { w, h }] = useSize<HTMLDivElement>();
  const times = useMemo(() => ttt(b, target), [b, target]);
  const xMax = b.time_ms;
  const sx = (x: number) => M.l + (x / xMax) * (w - M.l - M.r);
  const sy = (y: number) => M.t + (1 - y) * (h - M.t - M.b);
  if (!b.results.some((r) => r.runs?.length)) return <div ref={box} className="empty">This bench has no traces (saved before traces were recorded).</div>;
  return (
    <div ref={box} style={{ height: "100%" }}>
      <svg width={w} height={h}>
        <Legend b={b} w={w} />
        {[0, 0.5, 1].map((t) => <g key={t}><line x1={M.l} x2={w - M.r} y1={sy(t)} y2={sy(t)} stroke={HAIR} /><text x={M.l - 6} y={sy(t) + 4} textAnchor="end">{t * 100}%</text></g>)}
        {niceTicks(0, xMax / 1000, 6).map((t) => <text key={t} x={sx(t * 1000)} y={h - 8} textAnchor="middle">{t}s</text>)}
        {times.map((ts, i) => <path key={i} d={stepPath(ts, xMax, sx, sy)} fill="none" stroke={seriesColor(i)} strokeWidth={2} opacity={0.9} />)}
        <text x={w - M.r} y={M.t + 12} textAnchor="end">target {fmt(target, 4)}</text>
      </svg>
    </div>
  );
}

function TraceChart({ b, target }: { b: Bench; target: number }) {
  const [box, { w, h }] = useSize<HTMLDivElement>();
  const xMax = b.time_ms;
  const all = b.results.flatMap((r) => (r.runs ?? []).flatMap((x) => x.trace.map((p) => p[1])));
  if (!all.length) return <div ref={box} className="empty">This bench has no traces.</div>;
  const lo = Math.min(...all);
  const finals = b.results.flatMap((r) => r.scores).sort((x, y) => x - y);
  // The top: well above the worst final score, so the tails are readable.
  const hi = Math.max(finals[finals.length - 1] + (finals[finals.length - 1] - lo) * 2, lo + 1e-6);
  const sx = (x: number) => M.l + (x / xMax) * (w - M.l - M.r);
  const sy = (v: number) => M.t + (1 - (Math.min(v, hi) - lo) / (hi - lo)) * (h - M.t - M.b);
  return (
    <div ref={box} style={{ height: "100%" }}>
      <svg width={w} height={h}>
        <Legend b={b} w={w} />
        {niceTicks(lo, hi, 4).map((t) => <g key={t}><line x1={M.l} x2={w - M.r} y1={sy(t)} y2={sy(t)} stroke={HAIR} /><text x={M.l - 6} y={sy(t) + 4} textAnchor="end">{fmt(t)}</text></g>)}
        {niceTicks(0, xMax / 1000, 6).map((t) => <text key={t} x={sx(t * 1000)} y={h - 8} textAnchor="middle">{t}s</text>)}
        <line x1={M.l} x2={w - M.r} y1={sy(target)} y2={sy(target)} stroke={MUTED} strokeDasharray="4 3" />
        {b.results.map((r, i) => (r.runs ?? []).map((run) => {
          let d = "";
          run.trace.forEach(([t, s], k) => { d += k ? `H${sx(t)}V${sy(s)}` : `M${sx(t)},${sy(s)}`; });
          return <path key={`${i}-${run.seed}`} d={d + `H${sx(Math.min(run.elapsed_ms, xMax))}`} fill="none" stroke={seriesColor(i)} strokeWidth={1.2} opacity={0.75} />;
        }))}
        <text x={M.l + 4} y={h - M.b - 6}>lowest {fmt(instanceBest(b).reduce((a, x) => Math.min(a, x), Infinity), 4)}</text>
      </svg>
    </div>
  );
}
