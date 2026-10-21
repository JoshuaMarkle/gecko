// Results view: every saved run on the left; the selected run's convergence, ranked layouts
// and reproducibility details on the right.

import type { Progress, RunDetail, RunSummary } from "../api";
import { fmt, fmtCount, fmtTime } from "../format";
import { Convergence } from "./Charts";

interface Props {
  runs: RunSummary[];
  run: RunDetail | null;
  progress: Progress[];
  rank: number | null;
  refScore?: number;
  refName: string;
  onOpenRun: (id: string) => void;
  onPickRank: (i: number) => void;
  onShowOptimize: () => void;
}

export function ResultsView(p: Props) {
  const res = p.run?.result;
  const cfg = p.run?.config;
  return (
    <div className="results">
      <section className="pane">
        <div className="panehead"><b>Runs</b><span className="hint">{p.runs.length} saved in runs/</span></div>
        <div className="scroll">
          <div className="pad">
            <table className="tbl">
              <thead><tr><th>created</th><th>algo</th><th>profile</th><th>time</th><th>score</th></tr></thead>
              <tbody>
                {p.runs.map((r) => (
                  <tr key={r.id} className={`click ${cfg?.id === r.id ? "sel" : ""}`} onClick={() => p.onOpenRun(r.id)} title={r.id}>
                    <td className="l">{r.created.replace("T", " ").replace("Z", "")}<div className="hint mono">{r.best ?? r.status}</div></td>
                    <td>{r.algo}</td>
                    <td>{r.profile}</td>
                    <td>{r.elapsed_ms !== null ? fmtTime(r.elapsed_ms) : "–"}</td>
                    <td>{r.status === "running" ? <span className="badge warn">running</span> : r.best_score !== null ? fmt(r.best_score, 3) : "–"}</td>
                  </tr>
                ))}
                {p.runs.length === 0 && <tr><td className="muted">No runs yet. Press play to start one.</td><td /><td /><td /><td /></tr>}
              </tbody>
            </table>
          </div>
        </div>
      </section>

      <section className="pane">
        {!cfg ? <div className="empty">Select a run.</div> : (
          <>
            <div className="panehead">
              <b>{cfg.params.algo} run</b>
              <span className="hint mono">{cfg.id}</span>
              {res && <button className="btn soft small" style={{ marginLeft: "auto" }} onClick={p.onShowOptimize}>Open in Optimize</button>}
            </div>
            <div className="scroll">
              <div className="pad">
                <div className="chartbox"><Convergence progress={p.progress} reference={p.refScore} refName={p.refName} /></div>
              </div>
              {res && (
                <details className="sec" open>
                  <summary>Best layouts<span className="note">click to load into the editor</span></summary>
                  <div className="body">
                    <table className="tbl">
                      <thead><tr><th>#</th><th style={{ textAlign: "left" }}>layout</th><th>score</th><th>vs {p.refName}</th></tr></thead>
                      <tbody>
                        {res.best.map((b, i) => (
                          <tr key={b.hash} className={`click ${p.rank === i ? "sel" : ""}`} onClick={() => p.onPickRank(i)}>
                            <td>{b.rank}</td>
                            <td className="l mono">{b.rows.join(" ")}</td>
                            <td>{fmt(b.score, 3)}</td>
                            <td>{p.refScore !== undefined ? fmt(b.score - p.refScore, 3) : "–"}</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </details>
              )}
              <details className="sec" open>
                <summary>Run<span className="note">everything needed to reproduce it</span></summary>
                <div className="body">
                  <div className="kv"><span>Status</span><span>{p.run?.running ? (p.run.paused ? "paused" : "running") : res ? "done" : "no result"}</span></div>
                  {res && <div className="kv"><span>Evaluations</span><span>{fmtCount(res.evals)} · {(res.evals_per_sec / 1e6).toFixed(2)} M/s</span></div>}
                  {res && <div className="kv"><span>Elapsed</span><span>{fmtTime(res.elapsed_ms)}</span></div>}
                  <div className="kv"><span>Seed · threads</span><span>{cfg.params.seed} · {cfg.params.threads}</span></div>
                  <div className="kv"><span>Budget</span><span>{cfg.params.time_ms !== null ? fmtTime(cfg.params.time_ms) : `${fmtCount(cfg.params.evals ?? 0)} evals`}</span></div>
                  <div className="kv"><span>Profile</span><span>{cfg.profile.name}</span></div>
                  <div className="kv"><span>Corpus</span><span>{cfg.corpus.name}</span></div>
                  <div className="kv"><span>Board</span><span>{cfg.geometry.name}</span></div>
                  <div className="kv"><span>Pins</span><span className="mono">{cfg.pins || "none"}</span></div>
                  <div className="kv"><span>Start</span><span className="mono">{cfg.start}</span></div>
                </div>
              </details>
            </div>
          </>
        )}
      </section>
    </div>
  );
}
