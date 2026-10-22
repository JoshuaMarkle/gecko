// Saved tab, a board + layout run (web-screens.md "Analysis · Saved"): the start board's outline
// over the board at any point of the run (scrub or play through its progress, 2D or 3D), the
// convergence, how each free parameter and weighted metric moved, and the run's facts.

import { useEffect, useMemo, useState } from "react";
import type { GeomRunDetail } from "../../api";
import { fitChars, fmt, fmtCount, fmtTime } from "../../format";
import { asProgress, metricChanges, varLabel } from "../../geom";
import { Keyboard3D } from "../../three/Keyboard3D";
import { Convergence, useSize } from "../Charts";
import { Seg } from "../Form";
import { BoardOverlay, collisionColors, stableFocus, useParamBoard } from "./GeomBoard";
import { RangeBar } from "./GeomSearch";

interface Props {
  run: GeomRunDetail;
  refScore?: number;
  refName: string;
  /** Make the run's best board (`run:<id>`) the board and its layout the working model. */
  onUseBoard: () => void;
  /** The run's best board is in the board list (it is once the run has finished). */
  canUse: boolean;
}

export function GeomRunPanel({ run, refScore, refName, onUseBoard, canUse }: Props) {
  const cfg = run.config, res = run.result, prog = run.progress;
  const names = useMemo(() => cfg.vars.map((v) => v.name), [cfg.vars]);
  const starts = useMemo(() => cfg.vars.map((v) => v.start), [cfg.vars]);
  // Steps: the start, every progress event, then the final best board (after its layout polish).
  const steps = prog.length + (res ? 2 : 1);
  const [step, setStep] = useState(steps - 1);
  const [playing, setPlaying] = useState(false);
  const [view, setView] = useState<"2d" | "3d">("2d");
  useEffect(() => { setStep(steps - 1); setPlaying(false); }, [cfg.id, steps]);
  useEffect(() => {
    if (!playing) return;
    const t = setInterval(() => setStep((s) => { if (s >= steps - 1) { setPlaying(false); return s; } return s + 1; }), Math.max(30, 4000 / steps));
    return () => clearInterval(t);
  }, [playing, steps]);

  const at = step === 0 ? null : step <= prog.length ? prog[step - 1] : null;
  const values = step === 0 ? starts : at ? at.values : res?.best.values ?? starts;
  const chars = step === 0 ? res?.start.chars ?? cfg.start : at ? at.chars : res?.best.chars ?? cfg.start;
  const start = useParamBoard(run.param, names, starts, `run:${cfg.id}:start`);
  const shown = useParamBoard(run.param, names, values, `run:${cfg.id}`);
  const focus = useMemo(() => (start ? stableFocus(start.geometry.key) : undefined), [start]);
  const [box, { w, h }] = useSize<HTMLDivElement>();
  const caption = step === 0 ? "Start board" : at
    ? `${at.phase === "polish" ? "Polish" : `Generation ${at.generation}`} · ${fmtCount(at.candidates)} boards · best ${fmt(at.best, 3)}`
    : "Best board, after the final layout polish";
  const nColl = shown?.collisions.length ?? 0;
  const changes = res ? metricChanges(res.start.breakdown, res.best.breakdown).slice(0, 10) : [];
  const gain = res ? res.best.score - res.start.score : 0;

  return (
    <>
      <div className="panehead">
        <b>Board + layout run</b>
        <span className="hint mono">{cfg.id}</span>
        {res && <button className="btn soft small" style={{ marginLeft: "auto" }} disabled={!canUse} onClick={onUseBoard}
          title={canUse ? "Use the best board (and its layout) for scoring and the next runs" : "Reload the page to list this run's board"}>Use this board</button>}
      </div>
      <div className="scroll">
        <div className="pad">
          <div className="geomstage" ref={box}>
            {shown && !shown.error && view === "2d" && w > 0 && (
              <BoardOverlay board={shown.geometry} ghost={step > 0 ? start?.geometry : undefined} chars={fitChars(chars, shown.geometry)} collisions={shown.collisions} width={w} height={h} />
            )}
            {shown && !shown.error && view === "3d" && (
              <Keyboard3D geometry={shown.geometry} chars={fitChars(chars, shown.geometry)} heat={[]} arcs={[]} overlay="none" mode="3d" selected={null} onKey={() => {}}
                focus={focus} colors={collisionColors(shown.geometry.key.length, shown.collisions)} ghost={step > 0 ? start?.geometry.key : undefined} />
            )}
            {shown?.error && <div className="empty">{shown.error}</div>}
            <div className="viewlabel"><span>{caption}</span><span className={nColl ? "err" : "muted"}>{nColl ? `${nColl} collisions` : "no collisions"}</span></div>
            <div className="viewctl"><Seg inline value={view} onChange={setView} options={[["2d", "2D"], ["3d", "3D"]]} /></div>
          </div>
          <div className="geomscrub">
            <button className="btn soft small" onClick={() => { if (step >= steps - 1) setStep(0); setPlaying(!playing); }}>{playing ? "Pause" : "Play"}</button>
            <input type="range" min={0} max={steps - 1} step={1} value={step} onChange={(e) => { setPlaying(false); setStep(+e.target.value); }} title="Board over the run" />
            <span className="hint">{step === 0 ? "start" : step === steps - 1 && res ? "best" : `${step} / ${prog.length}`}</span>
            <span className="hint"><i className="dash" /> start board</span>
          </div>
          <div className="chartbox"><Convergence progress={prog.map(asProgress)} reference={refScore} refName={refName} /></div>
        </div>
        {res && (
          <details className="sec" open>
            <summary>Result<span className="note">both boards get the same final layout polish</span></summary>
            <div className="body">
              <div className="kv"><span>Start board</span><span>{fmt(res.start.score, 4)}</span></div>
              <div className="kv"><span>Best board</span><span>{fmt(res.best.score, 4)} <span className={`badge ${gain < -1e-9 ? "success" : ""}`}>{gain <= 0 ? "" : "+"}{fmt(gain, 4)} · {fmt(100 * gain / Math.max(Math.abs(res.start.score), 1e-9), 1)}%</span></span></div>
              <div className="kv"><span>Collisions</span><span>{res.feasible ? "none" : `${res.best.collisions} pairs, ${fmt(res.best.depth, 3)} u: no collision-free board found`}</span></div>
              {changes.length > 0 && (
                <table className="tbl">
                  <thead><tr><th>metric</th><th>start board</th><th>best board</th><th title="Change of the metric's weighted contribution to the score">Δ score</th></tr></thead>
                  <tbody>{changes.map((m) => (
                    <tr key={m.id}><td className="l" title={m.id}>{m.name}</td><td>{fmt(m.start)}{m.unit === "%" ? "%" : ""}</td><td>{fmt(m.best)}{m.unit === "%" ? "%" : ""}</td>
                      <td><span className={`dot ${m.delta < 0 ? "better" : "worse"}`} /> {m.delta > 0 ? "+" : ""}{fmt(m.delta, 3)}</td></tr>
                  ))}</tbody>
                </table>
              )}
            </div>
          </details>
        )}
        <details className="sec" open>
          <summary>Parameters<span className="note">{cfg.vars.length} free · start → {res ? "best" : "shown"}</span></summary>
          <div className="body">
            <table className="tbl">
              <thead><tr><th>parameter</th><th>start</th><th>{res ? "best" : "shown"}</th><th>bounds</th></tr></thead>
              <tbody>{cfg.vars.map((v, i) => {
                const [label, unit] = varLabel(v.name);
                const b = (res?.best.values ?? values)[i];
                const moved = Math.abs(b - v.start) > 1e-6;
                return (
                  <tr key={v.name} className={moved ? "" : "off"} title={v.name}>
                    <td className="l">{label}</td><td>{fmt(v.start, 2)}{unit}</td><td>{fmt(b, 2)}{unit}</td>
                    <td><RangeBar lo={v.lo} hi={v.hi} start={v.start} best={b} /></td>
                  </tr>
                );
              })}</tbody>
            </table>
          </div>
        </details>
        <details className="sec" open>
          <summary>Run<span className="note">everything needed to reproduce it</span></summary>
          <div className="body">
            <div className="kv"><span>Status</span><span>{run.running ? (run.paused ? "paused" : "running") : res ? "done" : "no result"}</span></div>
            {res && <div className="kv"><span>Boards</span><span>{fmtCount(res.candidates)} · {res.generations} generations · {(res.layout_evals / 1e6).toFixed(1)} M layout evals</span></div>}
            {res && <div className="kv"><span>Elapsed</span><span>{fmtTime(res.elapsed_ms)}</span></div>}
            <div className="kv"><span>Mode</span><span>{cfg.params.mode}{cfg.params.mode === "bilevel" ? ` · ${fmtCount(cfg.params.inner_evals)} swaps / board` : ` · ${cfg.params.rounds} rounds`}</span></div>
            <div className="kv"><span>Seed · threads</span><span>{cfg.params.seed} · {cfg.params.threads}</span></div>
            <div className="kv"><span>Budget</span><span>{cfg.params.time_ms !== null ? fmtTime(cfg.params.time_ms) : `${fmtCount(cfg.params.evals ?? 0)} boards`}</span></div>
            <div className="kv"><span>Free</span><span className="mono">{cfg.free}</span></div>
            <div className="kv"><span>Clearance · penalty</span><span>{fmt(cfg.params.margin, 2)} u · {fmt(cfg.params.penalty, 1)}</span></div>
            <div className="kv"><span>Board</span><span>{cfg.geometry.name}</span></div>
            <div className="kv"><span>Profile</span><span>{cfg.profile.name}</span></div>
            <div className="kv"><span>Corpus</span><span>{cfg.corpus.name}</span></div>
            <div className="kv"><span>Pins</span><span className="mono">{cfg.pins || "none"}</span></div>
          </div>
        </details>
      </div>
    </>
  );
}
