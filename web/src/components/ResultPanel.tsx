// Right panel: errors, the current layout with score vs reference, metrics table, run results,
// recent runs and exports.

import type { Analysis, Geometry, RunDetail, RunSummary } from "../api";
import { fmt, fmtCount, fmtTime, rowsOf, verdict } from "../format";

interface Props {
  err: string | null;
  geometry: Geometry;
  chars: string;
  label: string;
  isBest: boolean;
  a: Analysis | null;
  refA: Analysis | null;
  refName: string;
  run: RunDetail | null;
  rank: number | null;
  onPickRank: (i: number) => void;
  runs: RunSummary[];
  onOpenRun: (id: string) => void;
  onExportJson: () => void;
  onExportToml: () => void;
  onCopy: (text: string) => void;
  copied: string | null;
}

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

export function ResultPanel(p: Props) {
  const a = p.a;
  const delta = a && p.refA ? a.breakdown.score - p.refA.breakdown.score : null;
  const rows = rowsOf(p.chars, p.geometry);
  const res = p.run?.result;
  return (
    <>
      <div className="panehead"><b>Result</b><span className="hint">lower score is better</span></div>
      <div className="scroll">
        {p.err && <div className="pad"><div className="notice error">{p.err}</div></div>}
        <div className="pad">
          <div className="cardhead">
            <b>{p.label}</b>
            {p.isBest && <span className="badge best">Best</span>}
          </div>
          <div className="score">
            <b>{a ? fmt(a.breakdown.score, 3) : "–"}</b>
            {delta !== null && (
              <span className={`badge ${delta < -1e-9 ? "success" : delta > 1e-9 ? "error" : ""}`} title={`Score difference vs ${p.refName}`}>
                {delta <= 0 ? "" : "+"}{fmt(delta, 3)} vs {p.refName}
              </span>
            )}
          </div>
          <div className="layoutrows">{rows.map((r) => [...r].join(" ")).join("\n")}</div>
          <div className="btns">
            <button className="btn soft small" onClick={() => p.onCopy(rows.join(" "))}>{p.copied === "rows" ? "Copied" : "Copy layout"}</button>
            <button className="btn soft small" onClick={() => p.onCopy(`gecko analyze raw:${p.chars}`)}>{p.copied === "cli" ? "Copied" : "Copy CLI command"}</button>
            <button className="btn soft small" onClick={p.onExportToml}>Layout TOML</button>
            <button className="btn soft small" onClick={p.onExportJson}>Analysis JSON</button>
          </div>
        </div>

        {a && (
          <details className="sec" open>
            <summary>Metrics<span className="note">value · {p.refName} · contribution</span></summary>
            <div className="body">
              <MetricTable a={a} refA={p.refA} refName={p.refName} />
            </div>
          </details>
        )}

        {res && p.run && (
          <details className="sec" open>
            <summary>Run results<span className="note">{p.run.config.params.algo} · {fmtCount(res.evals)} evals · {fmtTime(res.elapsed_ms)}</span></summary>
            <div className="body">
              <table className="tbl">
                <thead><tr><th>#</th><th style={{ textAlign: "left" }}>layout</th><th>score</th></tr></thead>
                <tbody>
                  {res.best.map((b, i) => (
                    <tr key={b.hash} className={`click ${p.rank === i ? "sel" : ""}`} onClick={() => p.onPickRank(i)}>
                      <td>{b.rank}</td>
                      <td className="l mono">{b.rows.join(" ")}</td>
                      <td>{fmt(b.score, 3)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
              <div className="hint">{p.run.config.id} · seed {p.run.config.params.seed} · {p.run.config.params.threads} threads · {(res.evals_per_sec / 1e6).toFixed(2)} M evals/s</div>
            </div>
          </details>
        )}

        <details className="sec" open>
          <summary>Runs<span className="note">{p.runs.length} saved in runs/</span></summary>
          <div className="body">
            <table className="tbl">
              <thead><tr><th>run</th><th>algo</th><th>score</th></tr></thead>
              <tbody>
                {p.runs.slice(0, 40).map((r) => (
                  <tr key={r.id} className={`click ${p.run?.config.id === r.id ? "sel" : ""}`} onClick={() => p.onOpenRun(r.id)} title={r.best ?? ""}>
                    <td className="l">{r.created.replace("T", " ").replace("Z", "")}<div className="hint mono">{r.best ?? r.status}</div></td>
                    <td>{r.algo}</td>
                    <td>{r.status === "running" ? <span className="badge warn">running</span> : r.best_score !== null ? fmt(r.best_score, 3) : "–"}</td>
                  </tr>
                ))}
                {p.runs.length === 0 && <tr><td className="muted">No runs yet.</td><td /><td /></tr>}
              </tbody>
            </table>
          </div>
        </details>
      </div>
    </>
  );
}
