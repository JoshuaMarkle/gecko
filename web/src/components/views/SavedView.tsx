// Saved view: every saved run (with a mini keyboard of its best layout) on the left; the selected
// run's convergence (hover: best layout at that time), process views (`runs/ProcessPanel`: workers,
// schedule, symbol migration, settling order), ranked layouts and reproducibility details on the right. A board + layout run shows its board instead (`geom/GeomRunPanel`). NSGA-II /
// RWSA runs add their Pareto front, qd runs their MAP-Elites grid (`runs/`).

import type { Geometry, GeomRunDetail, Progress, RunDetail, RunSummary } from "../../api";
import { fitChars, fmt, fmtCount, fmtTime } from "../../format";
import { Thumb } from "../Board2D";
import { Convergence } from "../Charts";
import { GeomRunPanel } from "../geom/GeomRunPanel";
import { EliteGrid } from "../runs/EliteGrid";
import { ParetoPanel } from "../runs/ParetoPanel";
import { ProcessPanel } from "../runs/ProcessPanel";

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
  onShowLayout: () => void;
  /** The selected run when it is a board + layout run (then `run` is null). */
  geomRun: GeomRunDetail | null;
  onUseBoard: (id: string) => void;
  /** Chars of the working model. */
  current: string;
  /** Make a Pareto-front layout or MAP-Elites elite the working model. */
  onAdopt: (chars: string, label: string, score: number) => void;
}

export function SavedView(p: Props) {
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
                {p.runs.map((r) => {
                  const g = p.geometries.find((x) => x.name === r.geometry);
                  const best = r.best?.replace(/ /g, "");
                  return (
                  <tr key={r.id} className={`click ${(cfg?.id ?? p.geomRun?.config.id) === r.id ? "sel" : ""}`} onClick={() => p.onOpenRun(r.id)} title={r.id}>
                    <td className="l">{r.created.replace("T", " ").replace("Z", "")}
                      <div>{g && best ? <Thumb geometry={g} chars={fitChars(best, g)} u={15} width={240} /> : <span className="hint mono">{r.best ?? r.status}</span>}</div></td>
                    <td>{r.kind === "geometry" ? "board" : r.algo}</td>
                    <td>{r.profile}</td>
                    <td>{r.elapsed_ms !== null ? fmtTime(r.elapsed_ms) : "–"}</td>
                    <td>{r.status === "running" ? <span className="badge warn">running</span> : r.best_score !== null ? fmt(r.best_score, 3) : "–"}</td>
                  </tr>
                  );
                })}
                {p.runs.length === 0 && <tr><td className="muted">No runs yet. Press play to start one.</td><td /><td /><td /><td /></tr>}
              </tbody>
            </table>
          </div>
        </div>
      </section>

      <section className="pane">
        {p.geomRun ? (
          <GeomRunPanel run={p.geomRun} refScore={p.refScore} refName={p.refName} onUseBoard={() => p.onUseBoard(p.geomRun!.config.id)}
            canUse={p.geometries.some((g) => g.run === p.geomRun!.config.id)} />
        ) : !cfg ? <div className="empty">Select a run.</div> : (
          <>
            <div className="panehead">
              <b>{cfg.params.algo} run</b>
              <span className="hint mono">{cfg.id}</span>
              {res && <button className="btn soft small" style={{ marginLeft: "auto" }} onClick={p.onShowLayout}>Open in Layout</button>}
            </div>
            <div className="scroll">
              <div className="pad">
                <div className="chartbox"><Convergence progress={p.progress} reference={p.refScore} refName={p.refName} geometry={geo} /></div>
              </div>
              <ProcessPanel progress={p.progress} geometry={geo} />
              {res?.front && res.front.length > 0 && (
                <ParetoPanel front={res.front} objectives={res.objectives ?? []} geometry={geo}
                  picked={res.front.some((f) => f.chars === p.current) ? res.front.findIndex((f) => f.chars === p.current) : null}
                  onPick={(i) => p.onAdopt(res.front![i].chars, `${cfg.params.algo} front #${res.front![i].rank}`, res.front![i].score)} />
              )}
              {cfg.params.algo === "qd" && (
                <EliteGrid runId={cfg.id} geometry={geo} current={p.current}
                  onPick={(e) => p.onAdopt(e.chars, `qd elite ${e.bins.join("·")}`, e.score)} />
              )}
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
