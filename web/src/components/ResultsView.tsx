// Saved view: every saved run on the left; the selected run's convergence, ranked layouts
// (2D thumbnails) and reproducibility details on the right.

import type { Geometry, Progress, RunDetail, RunSummary } from "../api";
import { fitChars, fmt, fmtCount, fmtTime } from "../format";
import { Thumb } from "./Board2D";
import { Convergence } from "./Charts";

interface Props {
  runs: RunSummary[];
  run: RunDetail | null;
  progress: Progress[];
  rank: number | null;
  refScore?: number;
  refName: string;
  geometries: Geometry[];
  onOpenRun: (id: string) => void;
  onPickRank: (i: number) => void;
  onShowHome: () => void;
}

export function ResultsView(p: Props) {
  const res = p.run?.result;
  const cfg = p.run?.config;
  const geo = cfg ? p.geometries.find((g) => g.name === cfg.geometry.name) : undefined;
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
              {res && <button className="btn soft small" style={{ marginLeft: "auto" }} onClick={p.onShowHome}>Open in Home</button>}
            </div>
            <div className="scroll">
              <div className="pad">
                <div className="chartbox"><Convergence progress={p.progress} reference={p.refScore} refName={p.refName} /></div>
              </div>
              {res && (
                <details className="sec" open>
                  <summary>Best layouts<span className="note">click one to make it the working model</span></summary>
                  <div className="body">
                    <table className="tbl">
                      <thead><tr><th>#</th><th style={{ textAlign: "left" }}>layout</th><th>score</th><th>vs {p.refName}</th>{res.holdout && <th title={`Score on ${res.holdout.corpus}, which the search never saw`}>{res.holdout.corpus}</th>}</tr></thead>
                      <tbody>
                        {res.best.map((b, i) => (
                          <tr key={b.hash} className={`click ${p.rank === i ? "sel" : ""}`} onClick={() => p.onPickRank(i)}>
                            <td>{b.rank}</td>
                            <td className="l">{geo ? <Thumb geometry={geo} chars={fitChars(b.chars, geo)} u={15} /> : <span className="mono">{b.rows.join(" ")}</span>}</td>
                            <td>{fmt(b.score, 3)}</td>
                            <td>{p.refScore !== undefined ? fmt(b.score - p.refScore, 3) : "–"}</td>
                            {res.holdout && <td>{b.holdout !== undefined ? fmt(b.holdout, 3) : "–"}</td>}
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
                  {res?.holdout && <>
                    <div className="kv"><span>Holdout corpus</span><span>{res.holdout.corpus}</span></div>
                    <div className="kv" title="Start → best on the holdout corpus, and on the search corpus"><span>Holdout gain</span><span>{fmt(res.holdout.start, 3)} → {fmt(res.holdout.best, 3)} (search {fmt(res.holdout.start_search, 3)} → {fmt(res.holdout.best_search, 3)})</span></div>
                    <div className="kv" title="Share of the search corpus's gain over the start layout that holds on the holdout corpus; 100% = fully generalizes"><span>Gain kept</span><span>{(100 * res.holdout.retained).toFixed(1)}%{res.holdout.rank_corr !== undefined ? ` · rank ρ ${fmt(res.holdout.rank_corr, 2)}` : ""}</span></div>
                  </>}
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
