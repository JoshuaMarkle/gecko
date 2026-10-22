// Layout tab (web-screens.md "Layout"): the layout for the chosen board. Search settings |
// the working model in 3D, the algorithm's animation, or the 2D layout editor | convergence,
// past runs on these inputs, score and main metrics.

import { useEffect, useMemo, useRef, useState } from "react";
import { api, type Analysis, type BoundGap, type CorpusInfo, type Geometry, type LayoutInfo, type Meta, type PairKind, type Progress, type RunDetail, type RunSummary } from "../../api";
import { ACCENT, fingerLegend, HAIR, INK, MUTED, PAIR_KINDS } from "../../colors";
import { fmtPow10, rediscovery, searchSpace, timeToOptimum } from "../../estimate";
import { fmt, fmtCount, fmtTime, scoredMetrics, slotCount, undirected, verdict } from "../../format";
import type { SearchSettings } from "../../inputs";
import { Keyboard3D, type Arc, type Overlay, type ViewMode } from "../../three/Keyboard3D";
import { Convergence, useSize } from "../Charts";
import { Frame, Num, Sec, Seg, Slider, Toggle } from "../Form";
import { LayoutEditor } from "../LayoutEditor";

export const ALGOS: [string, string, string][] = [
  ["sa", "Simulated annealing", "Swaps two keys at a time. Worse swaps are accepted with a probability that shrinks as the temperature cools, so it can climb out of local optima early and settles late. Restarts run in parallel threads."],
  ["hc", "Hill climbing", "Takes the best improving swap until none is left (a local optimum), then restarts from a random layout. Fast and simple; quality comes from many restarts."],
  ["bls", "Breakout local search", "Descends to a local optimum, then kicks the layout with a few random swaps (a breakout) whose strength adapts to how often it lands back in the same place."],
  ["tabu", "Tabu search", "Always takes the best swap, even a worse one, but forbids undoing recent swaps for a while so it walks through valleys instead of cycling."],
  ["ga", "Genetic algorithm", "A population of layouts. The best breed: children mix their parents' key positions and mutate; the worst are replaced each generation."],
  ["memetic", "Memetic islands", "A genetic algorithm on several islands; every child is refined by a short local search, and islands exchange their best layouts around a ring."],
  ["qd", "Quality diversity", "Keeps the best layout in each cell of a feature grid (e.g. hand balance × rolls), so it returns a map of good, different layouts."],
  ["nsga2", "NSGA-II (Pareto)", "Multi-objective: keeps the set of layouts no other layout beats on every metric (the Pareto front) instead of one weighted score."],
  ["rwsa", "Random-weight SA (Pareto)", "Annealing runs under random metric weightings; together they trace the Pareto front."],
];
export const algoName = (a: string) => ALGOS.find(([x]) => x === a)?.[1] ?? a;

interface Props {
  meta: Meta;
  layouts: LayoutInfo[];
  geometry: Geometry;
  corpora: CorpusInfo[];
  corpusId: string;
  search: SearchSettings;
  setSearch: (s: SearchSettings) => void;
  /** A run of any kind is live: settings are locked. */
  locked: boolean;
  /** A layout run is live: the main view shows its best layout. */
  live: boolean;
  /** Layout runs on exactly the current inputs (`sameInputs`). */
  runs: RunSummary[];
  /** The layout runs start from on Auto, and the editor edits. */
  chars: string;
  label: string;
  /** Score of the shown layout (the live best during a run). */
  a: Analysis | null;
  pins: string;
  setPins: (p: string) => void;
  onSwap: (i: number, j: number) => void;
  canUndo: boolean;
  onUndo: () => void;
  onLoad: (chars: string, label: string) => void;
  /** The shown layer of the shown layout: symbols, heat and muted keys per key. */
  view: { chars: string; heat: number[]; muted: boolean[] };
  shownLabel: string;
  hasModel: boolean;
  refA: Analysis | null;
  /** The shown layout is the reference (no "vs reference" comparison). */
  isRef: boolean;
  refName: string;
  progress: Progress[];
  /** The opened layout run (convergence title, lower bound). */
  run: RunDetail | null;
}

/** Sidebar section shared by every optimize tab: the layout runs start from (or keep). */
export function StartSec({ layouts, start, setStart, label, pins, onLoad, onEdit, title = "Starting layout", note = "where runs begin" }: {
  layouts: LayoutInfo[]; start: string; setStart: (v: string) => void; label: string; pins: string;
  onLoad: (chars: string, label: string) => void; onEdit?: () => void; title?: string; note?: string;
}) {
  return (
    <Sec id="search-start" title={title} note={note}>
      <div className="row">
        <label title="Auto uses whatever layout is loaded (the working model or your edits). Pick a named layout to always use it.">Layout</label>
        <select value={start} onChange={(e) => {
          const v = e.target.value;
          setStart(v);
          const l = layouts.find((x) => x.id === v);
          if (l) onLoad(l.chars, l.name);
        }}>
          <option value="auto">Auto · {label || "loaded layout"}</option>
          {layouts.map((l) => <option key={l.id} value={l.id}>{l.name}</option>)}
        </select>
        <span />
      </div>
      <div className="btns">{onEdit && <button className="btn outline small" onClick={onEdit}>Edit layout</button>}
        {pins && <span className="hint">pins: <span className="mono">{pins}</span></span>}</div>
    </Sec>
  );
}

// ---------- algorithm animation ----------

const N = 12;
type Sim = { perm: number[]; pop: number[][]; temp: number; trace: number[]; swap: [number, number] | null; stuck: number; gen: number };
const cost = (p: number[]) => p.reduce((t, v, i) => t + Math.abs(v - i), 0);
const shuffled = (r: () => number) => { const a = [...Array(N).keys()]; for (let i = N - 1; i > 0; i--) { const j = Math.floor(r() * (i + 1)); [a[i], a[j]] = [a[j], a[i]]; } return a; };

function step(algo: string, s: Sim, r: () => number): Sim {
  const pop = algo === "ga" || algo === "memetic" || algo === "qd" || algo === "nsga2";
  if (pop) {
    const sorted = [...s.pop].sort((a, b) => cost(a) - cost(b));
    const [a, b] = [sorted[Math.floor(r() * 3)], sorted[Math.floor(r() * 4)]];
    // Order crossover then one mutation; memetic refines the child with a few improving swaps.
    const cut = 1 + Math.floor(r() * (N - 2));
    let child = [...a.slice(0, cut), ...b.filter((x) => !a.slice(0, cut).includes(x))];
    const [i, j] = [Math.floor(r() * N), Math.floor(r() * N)];
    [child[i], child[j]] = [child[j], child[i]];
    if (algo === "memetic") for (let k = 0; k < 4; k++) child = improve(child, r) ?? child;
    sorted[sorted.length - 1] = child;
    const best = cost(sorted[0]);
    return { ...s, pop: sorted, perm: sorted[0], gen: s.gen + 1, trace: [...s.trace.slice(-80), best], swap: null };
  }
  const [i, j] = [Math.floor(r() * N), Math.floor(r() * N)];
  if (i === j) return s;
  const next = [...s.perm];
  [next[i], next[j]] = [next[j], next[i]];
  const d = cost(next) - cost(s.perm);
  const temp = algo === "sa" || algo === "rwsa" ? Math.max(0.05, s.temp * 0.985) : 0;
  let accept = d < 0 || (temp > 0 && r() < Math.exp(-d / (temp * 4)));
  if (algo === "tabu") accept = d <= 2;
  let perm = accept ? next : s.perm;
  let stuck = accept && d < 0 ? 0 : s.stuck + 1;
  // Hill climbing restarts, breakout kicks, annealing reheats once settled.
  if (stuck > 40) {
    if (algo === "bls") { perm = [...perm]; for (let k = 0; k < 3; k++) { const [a, b] = [Math.floor(r() * N), Math.floor(r() * N)]; [perm[a], perm[b]] = [perm[b], perm[a]]; } }
    else if (algo === "hc") perm = shuffled(r);
    stuck = 0;
  }
  const reheat = (algo === "sa" || algo === "rwsa") && temp <= 0.05 && cost(perm) === 0;
  return { ...s, perm, temp: reheat ? 1 : temp, stuck, trace: [...s.trace.slice(-80), cost(perm)], swap: accept ? [i, j] : null, gen: s.gen + 1 };
}

function improve(p: number[], r: () => number): number[] | null {
  const [i, j] = [Math.floor(r() * N), Math.floor(r() * N)];
  const q = [...p];
  [q[i], q[j]] = [q[j], q[i]];
  return cost(q) < cost(p) ? q : null;
}

function AlgoVisual({ algo }: { algo: string }) {
  const [box, { w, h }] = useSize<HTMLDivElement>();
  const rng = useRef(Math.random);
  const fresh = (): Sim => ({ perm: shuffled(rng.current), pop: Array.from({ length: 8 }, () => shuffled(rng.current)), temp: 1, trace: [], swap: null, stuck: 0, gen: 0 });
  const [sim, setSim] = useState<Sim>(fresh);
  useEffect(() => { setSim(fresh()); }, [algo]); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => {
    const t = setInterval(() => setSim((s) => step(algo, s, rng.current)), 140);
    return () => clearInterval(t);
  }, [algo]);
  const pop = algo === "ga" || algo === "memetic" || algo === "qd" || algo === "nsga2";
  const tw = Math.min(w - 32, 420), bw = tw / N, x0 = (w - tw) / 2;
  const color = (v: number) => `hsl(${(v / N) * 280}, 55%, ${pop ? 70 : 68}%)`;
  const maxC = N * N / 2;
  const traceY0 = h - 18, traceH = 50;
  return (
    <div ref={box} className="chartfill">
      <svg width={w} height={h}>
        {!pop && sim.perm.map((v, i) => {
          const bh = 18 + (v + 1) * 7;
          const on = sim.swap && (sim.swap[0] === i || sim.swap[1] === i);
          return <rect key={i} x={x0 + i * bw + 2} y={40 + 110 - bh} width={bw - 4} height={bh} rx={3} fill={color(v)} stroke={on ? ACCENT : HAIR} strokeWidth={on ? 2 : 1} />;
        })}
        {pop && sim.pop.map((p, r) => p.map((v, i) => {
          const islands = algo === "memetic" ? 1 : 0;
          const gap = islands && r >= 4 ? 10 : 0;
          return <rect key={`${r}-${i}`} x={x0 + i * bw + 1} y={24 + r * 16 + gap} width={bw - 2} height={13} rx={2} fill={color(v)} stroke={r === 0 ? INK : "none"} />;
        }))}
        {!pop && <text x={x0} y={24}>{algo === "sa" || algo === "rwsa" ? `temperature ${sim.temp.toFixed(2)}` : algo === "bls" ? "breakout after 40 failed swaps" : algo === "tabu" ? "best swap, recent moves forbidden" : "improving swaps only"}</text>}
        {pop && <text x={x0} y={16}>{algo === "memetic" ? "two islands · children refined by local search" : "population, best on top"} · generation {sim.gen}</text>}
        <path d={sim.trace.map((c, i) => `${i ? "L" : "M"}${x0 + (i / 80) * tw},${traceY0 - (1 - c / maxC) * traceH}`).join(" ")} fill="none" stroke={INK} strokeWidth={1.2} />
        <line x1={x0} x2={x0 + tw} y1={traceY0} y2={traceY0} stroke={HAIR} />
        <text x={x0 + tw} y={h - 4} textAnchor="end" fill={MUTED}>{pop ? "best fitness over time" : "fitness over time"} (sorted = best)</text>
      </svg>
    </div>
  );
}

// ---------- bottom: past runs ----------

/** Chance the best known layout is optimal, this algorithm's time-to-optimum guess, the search
 *  space and the lower bound (the opened run's, or loaded from the best matching run). */
function PastRuns(p: Props) {
  const [loaded, setLoaded] = useState<BoundGap | null | "loading">(null);
  const slots = slotCount(p.geometry, p.chars);
  const sym = new Set([...p.chars]);
  const pinned = [...new Set(p.pins.replace(/\s/g, ""))].filter((c) => sym.has(c)).length;
  const sp = searchSpace(slots, pinned);
  const all = rediscovery(p.runs);
  const algo = rediscovery(p.runs.filter((r) => r.algo === p.search.algo));
  const tto = timeToOptimum(algo);
  const bound = p.run?.result?.bound?.total ?? (loaded !== "loading" ? loaded : null);
  const load = async () => {
    const best = p.runs.filter((r) => r.best_score !== null).sort((a, b) => a.best_score! - b.best_score!)[0];
    if (!best) return;
    setLoaded("loading");
    try { setLoaded((await api.run(best.id)).result?.bound?.total ?? null); } catch { setLoaded(null); }
  };
  return (
    <div>
      <h4>Chance we've found the optimal layout</h4>
      <div className="score"><b>{all.runs ? `${(100 * all.chance).toFixed(0)}%` : "–"}</b></div>
      <div className="hint">
        {all.runs
          ? `${all.runs} run${all.runs === 1 ? "" : "s"} on these inputs; the best (${fmt(all.best ?? 0, 3)}) was reached ${all.hits}×. Estimated from how often independent runs rediscover it.`
          : "No runs on these inputs yet. Press play."}
      </div>
      <div className="kv"><span title={`${algoName(p.search.algo)}: mean run length / chance one run finds the best`}>Time to optimum</span>
        <span>{tto !== null ? `~${fmtTime(tto)} (guess)` : algo.runs ? "more runs needed" : "–"}</span></div>
      <div className="kv"><span title="Arrangements of the unpinned symbols over the unpinned keys">Search space</span>
        <span>{fmtPow10(sp.effective)}{pinned ? ` · ${pinned} pinned` : ""}</span></div>
      <div className="kv"><span title="Gilmore–Lawler: no layout scores below this">Lower bound</span>
        <span>{bound ? `${fmt(bound.lower, 3)} · within ${fmt(bound.gap_pct, 1)}%`
          : loaded === "loading" ? "loading…"
          : <button className="btn soft small" disabled={!p.runs.length} onClick={load} title={p.runs.length ? "Read the bound saved with the best matching run" : "No matching runs yet"}>Load</button>}</span></div>
    </div>
  );
}

const HIDDEN: PairKind[] = ["same_hand", "alternate"];

export function LayoutView(p: Props) {
  const s = p.search;
  const set = (patch: Partial<SearchSettings>) => p.setSearch({ ...s, ...patch });
  const [editing, setEditing] = useState(false);
  const [shown, setShown] = usePrefMain();
  const [mode, setMode] = useState<ViewMode>("3d");
  const [overlay, setOverlay] = useState<Overlay>("none");
  const desc = ALGOS.find(([a]) => a === s.algo)?.[2] ?? "";
  const algos = ALGOS.filter(([a]) => p.meta.algos.includes(a));
  const start = s.start ?? "auto";
  const budget = s.budget ?? "time";
  const a = p.a;
  const arcs = useMemo((): Arc[] => {
    const pairs = undirected(a?.bigrams ?? []).filter((x) => !HIDDEN.includes(x.kind));
    const max = Math.max(...pairs.map((x) => x.freq), 1e-9);
    return pairs.map((x) => ({ ngram: x.ngram, w: Math.sqrt(x.freq / max), color: PAIR_KINDS[x.kind][1] }));
  }, [a]);
  const delta = a && p.refA && !p.isRef ? a.breakdown.score - p.refA.breakdown.score : null;
  const top = a ? scoredMetrics(a.breakdown.metrics).slice(0, 7) : [];

  return (
    <Frame
      title="Optimize layout"
      control={<span className="hint">{p.locked ? "locked while running" : "for the chosen board"}</span>}
      side={
        <fieldset className="plain" disabled={p.locked}>
          <Sec id="search-algo" title="Algorithm" note="how the search moves">
            <div className="row">
              <label title={desc}>Algorithm</label>
              <select value={s.algo} onChange={(e) => set({ algo: e.target.value })}>
                {algos.map(([x, n]) => <option key={x} value={x}>{n}</option>)}
              </select>
              <span />
            </div>
          </Sec>
          <StartSec layouts={p.layouts} start={start} setStart={(v) => set({ start: v })} label={p.label} pins={p.pins} onLoad={p.onLoad} onEdit={() => setEditing(true)} />
          <Sec id="search" title="Budget" note="how long, how wide">
            <div className="row">
              <label title="Stop after a wall-clock time or after a number of evaluations (exactly reproducible).">Budget</label>
              <Seg value={budget} onChange={(b) => set({ budget: b })} options={[["time", "Time"], ["evals", "Evaluations"]]} />
              <span />
            </div>
            {budget === "time"
              ? <Num label="Time" help="Wall-clock budget for the run." value={s.timeS} min={1} step={1} unit="s" onChange={(v) => set({ timeS: Math.max(1, v) })} />
              : <Num label="Evaluations" help="Total layout evaluations across threads." value={s.evals ?? 50_000_000} min={1000} step={1_000_000} unit={fmtCount(s.evals ?? 50_000_000)} onChange={(v) => set({ evals: Math.max(1000, Math.round(v)) })} />}
            <ThreadsSeed s={s} set={set} cores={p.meta.threads} />
          </Sec>
          <Sec id="search-holdout" title="Holdout corpus" note="check for overfitting">
            <div className="row">
              <label title="Also score the results on a corpus the search never sees. If they keep most of their gain there, they did not just fit the search corpus.">Holdout</label>
              <select value={s.holdout && s.holdout !== p.corpusId ? s.holdout : ""} onChange={(e) => set({ holdout: e.target.value })}>
                <option value="">None</option>
                {p.corpora.filter((c) => c.id !== p.corpusId).map((c) => <option key={c.id} value={c.id}>{c.name}</option>)}
              </select>
              <span />
            </div>
          </Sec>
          <Sec id="search-stop" title="Stop confidence" note="saved, not applied yet">
            <Toggle label="Stop early" help="Stop once the estimated chance of having found the optimum passes the threshold (estimator planned; saved but not applied yet)." value={s.stopConf != null} onChange={(on) => set({ stopConf: on ? 0.95 : null })} />
            {s.stopConf != null && <Slider label="Threshold" help="Chance of having found the optimum." value={s.stopConf} min={0.5} max={0.999} step={0.005} unit={`${(100 * s.stopConf).toFixed(1)}%`} onChange={(v) => set({ stopConf: v })} />}
          </Sec>
          <Sec id="search-params" title={`${algoName(s.algo)} settings`} note="this algorithm only">
            {s.algo === "sa" && (
              <Num label="Steps / restart" help="Swap attempts in one cooling schedule. 0 = auto (about 4 restarts per thread within the time budget)." value={s.saSteps} step={100000} min={0} unit={s.saSteps === 0 ? "auto" : ""} onChange={(v) => set({ saSteps: Math.max(0, Math.round(v)) })} />
            )}
            {s.algo === "ga" && (
              <Num label="Population" help="Layouts per generation." value={s.gaPop} step={16} min={4} onChange={(v) => set({ gaPop: Math.max(4, Math.round(v)) })} />
            )}
            {s.algo === "memetic" && <>
              <Num label="Population / island" help="Distinct layouts per island; every child receives local refinement." value={s.memeticPop ?? 24} step={4} min={4} onChange={(v) => set({ memeticPop: Math.max(4, Math.round(v)) })} />
              <div className="row">
                <label title="Local search applied to every child.">Refinement</label>
                <select value={s.localSearch ?? "hc"} onChange={(e) => set({ localSearch: e.target.value as "hc" | "tabu" })}>
                  <option value="hc">Hill climbing</option><option value="tabu">Tabu search</option>
                </select><span />
              </div>
              <Num label="Local steps" help="Maximum improving hill-climbing moves or tabu iterations per child." value={s.localSteps ?? 12} min={0} onChange={(v) => set({ localSteps: Math.max(0, Math.round(v)) })} />
              <Num label="Islands" help="0 uses one island per thread. Islands exchange their best layouts around a ring." value={s.islands ?? 0} min={0} max={p.meta.threads} unit={(s.islands ?? 0) === 0 ? "auto" : ""} onChange={(v) => set({ islands: Math.min(p.meta.threads, Math.max(0, Math.round(v))) })} />
              <Num label="Migration interval" help="Generations between exchanges. 0 disables migration." value={s.migrationInterval ?? 5} min={0} onChange={(v) => set({ migrationInterval: Math.max(0, Math.round(v)) })} />
              <Num label="Migrants" help="Best distinct layouts sent to the next island each exchange." value={s.migrants ?? 2} min={0} onChange={(v) => set({ migrants: Math.max(0, Math.round(v)) })} />
            </>}
            {!["sa", "ga", "memetic"].includes(s.algo) && <div className="hint">No extra settings; defaults from the core.</div>}
          </Sec>
        </fieldset>
      }
      main={
        editing ? (
          <LayoutEditor geometry={p.geometry} chars={p.chars} label={p.label} a={p.a} pins={p.pins} setPins={p.setPins}
            onSwap={(i, j) => { if (start !== "auto") set({ start: "auto" }); p.onSwap(i, j); }} canUndo={p.canUndo} onUndo={p.onUndo} onClose={() => setEditing(false)} />
        ) : shown === "algorithm" ? (
          <div className="algoview">
            <div className="algoview-text"><h3>{algoName(s.algo)}</h3><p>{desc}</p></div>
            <div className="algoview-anim"><AlgoVisual algo={s.algo} /></div>
            <div className="viewctl"><MainSeg shown={shown} setShown={setShown} /></div>
          </div>
        ) : (
          <div className="viewport">
            <Keyboard3D geometry={p.geometry} chars={p.view.chars} heat={p.view.heat} muted={p.view.muted} arcs={arcs} overlay={overlay} mode={mode} selected={null} onKey={() => {}} />
            <div className="viewlabel">
              <span>{p.live ? "Live best" : p.hasModel ? p.shownLabel : `No model · showing ${p.shownLabel || "the start layout"}`}</span>
              {a && <span className="muted">{fmt(a.breakdown.score, 3)}</span>}
            </div>
            <div className="viewctl">
              <MainSeg shown={shown} setShown={setShown} />
              <div className="seg inline"><button className={mode === "3d" ? "on" : ""} onClick={() => setMode(mode === "3d" ? "top" : "3d")} title="3D view (off: flat top-down)">3D</button></div>
              <div className="seg inline">
                {([["none", "Default"], ["heat", "Heat"], ["fingers", "Fingers"], ["ngrams", "N-grams"]] as [Overlay, string][]).map(([o, n]) => <button key={o} className={overlay === o ? "on" : ""} onClick={() => setOverlay(o)}>{n}</button>)}
              </div>
            </div>
            {overlay === "fingers" && <div className="legend">{fingerLegend.map(([n, c]) => <span key={n}><i style={{ background: c }} />{n}</span>)}</div>}
            {overlay === "heat" && <div className="legend"><span>darker = more keystrokes</span></div>}
            {overlay === "ngrams" && <div className="legend">{(["sfb", "lsb", "scissor"] as PairKind[]).map((k) => <span key={k}><i style={{ background: PAIR_KINDS[k][1] }} />{PAIR_KINDS[k][0]}</span>)}</div>}
          </div>
        )
      }
      bottom={
        <div className="homegrid">
          <div className="homeconv">
            <h4>Convergence <span className="hint">{p.run?.result ? `${p.run.config.params.algo} · ${fmtTime(p.run.result.elapsed_ms)}` : p.live ? "live" : ""}</span></h4>
            <div className="chartfill"><Convergence progress={p.progress} reference={p.refA?.breakdown.score} refName={p.refName} geometry={p.geometry} /></div>
          </div>
          <PastRuns {...p} />
          <div>
            <h4>Score <span className="hint">lower is better</span></h4>
            <div className="score">
              <b>{a ? fmt(a.breakdown.score, 3) : "–"}</b>
              {delta !== null && <span className={`badge ${delta < -1e-9 ? "success" : delta > 1e-9 ? "error" : ""}`}>{delta <= 0 ? "" : "+"}{fmt(delta, 3)} vs {p.refName}</span>}
            </div>
            <table className="tbl">
              <tbody>
                {top.map((m) => {
                  const r = p.isRef ? undefined : p.refA?.breakdown.metrics.find((x) => x.id === m.id);
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

type Main = "result" | "algorithm";

/** Which main view is shown (remembered per browser). */
function usePrefMain(): [Main, (m: Main) => void] {
  const [m, setM] = useState<Main>(() => { try { return localStorage.getItem("gecko.layoutMain") === "algorithm" ? "algorithm" : "result"; } catch { return "result"; } });
  return [m, (x) => { setM(x); try { localStorage.setItem("gecko.layoutMain", x); } catch { /* storage unavailable */ } }];
}

function MainSeg({ shown, setShown }: { shown: Main; setShown: (m: Main) => void }) {
  return (
    <div className="seg inline">
      <button className={shown === "result" ? "on" : ""} onClick={() => setShown("result")} title="The working model (or the live best) in 3D">Result</button>
      <button className={shown === "algorithm" ? "on" : ""} onClick={() => setShown("algorithm")} title="How the chosen algorithm searches">Algorithm</button>
    </div>
  );
}

/** Threads and seed rows (every optimize tab). */
export function ThreadsSeed({ s, set, cores }: { s: SearchSettings; set: (patch: Partial<SearchSettings>) => void; cores: number }) {
  return <>
    <Num label="Threads" help="Independent workers (restarts / islands / candidate boards) in parallel." value={s.threads} min={1} max={cores} unit={`/${cores}`} onChange={(v) => set({ threads: Math.min(cores, Math.max(1, Math.round(v))) })} />
    <Num label="Seed" help="Runs are reproducible from the seed (exactly so with an evaluation budget)." value={s.seed} min={0} onChange={(v) => set({ seed: Math.max(0, Math.round(v)) })} />
  </>;
}
