// Home (web-screens.md "Home"): read-only summary of the inputs | 3D working model |
// convergence, chance of optimal, score and main metrics.

import { useMemo, useState } from "react";
import type { Analysis, Geometry, PairKind, Progress, RunDetail, RunSummary } from "../../api";
import { fingerLegend, PAIR_KINDS } from "../../colors";
import { rediscovery } from "../../estimate";
import { fmt, fmtTime, scoredMetrics, undirected, verdict } from "../../format";
import type { InputTabName } from "../../tabs";
import { Keyboard3D, type Arc, type Overlay, type ViewMode } from "../../three/Keyboard3D";
import { Convergence } from "../Charts";
import { Frame } from "../Form";

export interface Block { tab: InputTabName; title: string; rows: [string, string][]; flags?: [string, boolean][] }

interface Props {
  blocks: Block[];
  onGo: (tab: InputTabName) => void;
  geometry: Geometry;
  /** The shown layer of the working model: symbols, heat and muted keys per key. */
  view: { chars: string; heat: number[]; muted: boolean[] };
  label: string;
  hasModel: boolean;
  running: boolean;
  a: Analysis | null;
  refA: Analysis | null;
  refName: string;
  progress: Progress[];
  run: RunDetail | null;
  runs: RunSummary[];
  corpusName: string;
  profileName: string;
}

const HIDDEN: PairKind[] = ["same_hand", "alternate"];

export function HomeView(p: Props) {
  const [mode, setMode] = useState<ViewMode>("3d");
  const [overlay, setOverlay] = useState<Overlay>("none");
  const a = p.a;
  const arcs = useMemo((): Arc[] => {
    const shown = undirected(a?.bigrams ?? []).filter((x) => !HIDDEN.includes(x.kind));
    const max = Math.max(...shown.map((x) => x.freq), 1e-9);
    return shown.map((x) => ({ ngram: x.ngram, w: Math.sqrt(x.freq / max), color: PAIR_KINDS[x.kind][1] }));
  }, [a]);
  const same = p.runs.filter((r) => r.geometry === p.geometry.name && r.corpus === p.corpusName && r.profile === p.profileName);
  const conf = rediscovery(same);
  const bound = p.run?.result?.bound?.total;
  const delta = a && p.refA ? a.breakdown.score - p.refA.breakdown.score : null;
  const top = a ? scoredMetrics(a.breakdown.metrics).slice(0, 7) : [];

  return (
    <Frame
      title="Home"
      control={<span className="hint">read-only summary</span>}
      side={
        <div className="summary">
          {p.blocks.map((b) => (
            <button key={b.tab} className="sumblock" onClick={() => p.onGo(b.tab)} title={`Change on the ${b.title} tab`}>
              <div className="sumhead"><b>{b.title}</b><span className="hint">edit ›</span></div>
              {b.rows.map(([k, v]) => <div key={k} className="kv"><span>{k}</span><span>{v}</span></div>)}
              {b.flags && <div className="flags">{b.flags.map(([k, on]) => <span key={k} className={`flag ${on ? "on" : ""}`}>{k}: {on ? "on" : "off"}</span>)}</div>}
            </button>
          ))}
        </div>
      }
      main={
        <div className="viewport">
          <Keyboard3D geometry={p.geometry} chars={p.view.chars} heat={p.view.heat} muted={p.view.muted} arcs={arcs} overlay={overlay} mode={mode} selected={null} onKey={() => {}} />
          <div className="viewlabel">
            <span>{p.running ? "Live best" : p.hasModel ? p.label : `No model · showing ${p.label || "the start layout"}`}</span>
            {a && <span className="muted">{fmt(a.breakdown.score, 3)}</span>}
          </div>
          <div className="viewctl">
            <div className="seg inline"><button className={mode === "3d" ? "on" : ""} onClick={() => setMode(mode === "3d" ? "top" : "3d")} title="3D view (off: flat top-down)">3D</button></div>
            <div className="seg inline">
              {([["none", "Default"], ["heat", "Heat"], ["fingers", "Fingers"], ["ngrams", "N-grams"]] as [Overlay, string][]).map(([o, n]) => <button key={o} className={overlay === o ? "on" : ""} onClick={() => setOverlay(o)}>{n}</button>)}
            </div>
          </div>
          {overlay === "fingers" && <div className="legend">{fingerLegend.map(([n, c]) => <span key={n}><i style={{ background: c }} />{n}</span>)}</div>}
          {overlay === "heat" && <div className="legend"><span>darker = more keystrokes</span></div>}
          {overlay === "ngrams" && <div className="legend">{(["sfb", "lsb", "scissor"] as PairKind[]).map((k) => <span key={k}><i style={{ background: PAIR_KINDS[k][1] }} />{PAIR_KINDS[k][0]}</span>)}</div>}
        </div>
      }
      bottom={
        <div className="homegrid">
          <div className="homeconv">
            <h4>Convergence <span className="hint">{p.run?.result ? `${p.run.config.params.algo} · ${fmtTime(p.run.result.elapsed_ms)}` : p.running ? "live" : ""}</span></h4>
            <div className="chartfill"><Convergence progress={p.progress} reference={p.refA?.breakdown.score} refName={p.refName} /></div>
          </div>
          <div>
            <h4>Chance we've found the optimal layout</h4>
            <div className="score"><b>{conf.runs ? `${(100 * conf.chance).toFixed(0)}%` : "–"}</b></div>
            <div className="hint">
              {conf.runs
                ? `${conf.runs} run${conf.runs === 1 ? "" : "s"} on these inputs; the best (${fmt(conf.best ?? 0, 3)}) was reached ${conf.hits}×. Estimated from how often independent runs rediscover it.`
                : "No runs on these inputs yet. Press play."}
            </div>
            {bound && <div className="kv"><span>Lower bound</span><span>{fmt(bound.lower, 3)} · within {fmt(bound.gap_pct, 1)}%</span></div>}
          </div>
          <div>
            <h4>Score <span className="hint">lower is better</span></h4>
            <div className="score">
              <b>{a ? fmt(a.breakdown.score, 3) : "–"}</b>
              {delta !== null && <span className={`badge ${delta < -1e-9 ? "success" : delta > 1e-9 ? "error" : ""}`}>{delta <= 0 ? "" : "+"}{fmt(delta, 3)} vs {p.refName}</span>}
            </div>
            <table className="tbl">
              <tbody>
                {top.map((m) => {
                  const r = p.refA?.breakdown.metrics.find((x) => x.id === m.id);
                  const v = r ? verdict(m.dir, m.value, r.value) : "same";
                  return <tr key={m.id}><td>{m.name}</td><td>{fmt(m.value)}{m.unit === "%" ? "%" : ""}</td><td>{v !== "same" && <span className={`dot ${v}`} />}</td></tr>;
                })}
              </tbody>
            </table>
          </div>
        </div>
      }
    />
  );
}
