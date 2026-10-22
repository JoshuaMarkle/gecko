// Search tab (web-screens.md "Search"): search settings | a small animation of the chosen
// algorithm (or the 2D layout editor) | search-space size and time-to-optimum estimates.
// With "Board + layout" (a parametric board): board shape groups and settings | a preview of
// the board with one parameter swept | the free parameters and their bounds (`geom/GeomSearch`).

import { useEffect, useRef, useState } from "react";
import { api, type Analysis, type BoundGap, type CorpusInfo, type Geometry, type GeomVar, type LayoutInfo, type Meta, type RunSummary } from "../../api";
import { ACCENT, HAIR, INK, MUTED } from "../../colors";
import { fmtPow10, rediscovery, searchSpace, timeToOptimum } from "../../estimate";
import { fmt, fmtCount, fmtTime, slotCount } from "../../format";
import { GEOM_DEFAULTS, type GeomSettings } from "../../geom";
import type { SearchSettings } from "../../inputs";
import { useSize } from "../Charts";
import { Frame, Num, Sec, Seg, Slider, Toggle } from "../Form";
import { GeomParams, GeomPreview, GeomSide } from "../geom/GeomSearch";
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
  corpusName: string;
  profileName: string;
  search: SearchSettings;
  setSearch: (s: SearchSettings) => void;
  running: boolean;
  runs: RunSummary[];
  /** The layout runs start from on Auto, and the editor edits. */
  chars: string;
  label: string;
  a: Analysis | null;
  pins: string;
  setPins: (p: string) => void;
  onSwap: (i: number, j: number) => void;
  canUndo: boolean;
  onUndo: () => void;
  onLoad: (chars: string, label: string) => void;
  /** Free parameters of the board for the selected groups (`useGeomVars`). */
  geomVars: { vars: GeomVar[]; error: string | null };
  /** The profile weights a hand-model metric (G01 / G03 / G05): without one, board shape barely matters. */
  handWeighted: boolean;
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

// ---------- estimates ----------

function Estimates(p: Props & { onEstimate: () => void; bound: BoundGap | null | "loading" }) {
  const slots = slotCount(p.geometry, p.chars);
  const sym = new Set([...p.chars]);
  const pinned = [...new Set(p.pins.replace(/\s/g, ""))].filter((c) => sym.has(c)).length;
  const sp = searchSpace(slots, pinned);
  const same = p.runs.filter((r) => r.geometry === p.geometry.name && r.corpus === p.corpusName && r.profile === p.profileName && r.algo === p.search.algo);
  const c = rediscovery(same);
  const tto = timeToOptimum(c);
  return (
    <div className="bottomgrid">
      <div>
        <h4>Search space</h4>
        <div className="kv"><span>Slots</span><span>{slots}{pinned ? ` · ${pinned} pinned` : ""}</span></div>
        <div className="kv"><span>Arrangements</span><span title="slots!">{fmtPow10(sp.total)}</span></div>
        <div className="kv"><span>After pins</span><span title="(slots − pinned)!">{fmtPow10(sp.effective)}</span></div>
        <div className="hint">Every arrangement of the symbols over the keys; pins remove their symbols and keys.</div>
      </div>
      <div>
        <h4>From past runs <span className="hint">{algoName(p.search.algo)} · same board, language, profile</span></h4>
        <div className="kv"><span>Runs</span><span>{c.runs}{c.runs ? ` · mean ${fmtTime(c.meanMs)}` : ""}</span></div>
        <div className="kv"><span>Best score</span><span>{c.best !== null ? `${fmt(c.best, 3)} · reached ${c.hits}×` : "–"}</span></div>
        <div className="kv"><span>Found per run</span><span>{c.runs ? `${(100 * c.perRun).toFixed(0)}%` : "–"}</span></div>
        <div className="kv"><span>Chance it is optimal</span><span>{c.runs ? `${(100 * c.chance).toFixed(0)}%` : "–"}</span></div>
        <div className="kv"><span>Time to optimum</span><span>{tto !== null ? `~${fmtTime(tto)} (guess)` : c.runs ? "more runs needed" : "–"}</span></div>
        <div className="hint">Heuristic: how often independent runs rediscover the same best layout.</div>
      </div>
      <div className="estimate-cell">
        <h4>Lower bound <span className="hint">Gilmore–Lawler</span></h4>
        {p.bound === null && <div className="hint">Reads the bound saved with the best matching run (needs a run on these inputs).</div>}
        {p.bound === "loading" && <div className="hint">Loading…</div>}
        {p.bound && p.bound !== "loading" && <>
          <div className="kv"><span>No layout below</span><span>{fmt(p.bound.lower, 3)}</span></div>
          <div className="kv"><span>Best found</span><span>{fmt(p.bound.value, 3)}</span></div>
          <div className="kv"><span>Gap</span><span>within {fmt(p.bound.gap_pct, 1)}% of optimal</span></div>
        </>}
        <button className="btn outline small estimate-btn" disabled={!same.length} onClick={p.onEstimate} title={same.length ? "Load the bound from the best matching run" : "No matching runs yet"}>Estimate</button>
      </div>
    </div>
  );
}

export function SearchView(p: Props) {
  const s = p.search;
  const set = (patch: Partial<SearchSettings>) => p.setSearch({ ...s, ...patch });
  const [editing, setEditing] = useState(false);
  const [bound, setBound] = useState<BoundGap | null | "loading">(null);
  const desc = ALGOS.find(([a]) => a === s.algo)?.[2] ?? "";
  const algos = ALGOS.filter(([a]) => p.meta.algos.includes(a));
  const start = s.start ?? "auto";
  const budget = s.budget ?? "time";
  const board = s.target === "board";
  const g = s.geom ?? GEOM_DEFAULTS;
  const setGeom = (patch: Partial<GeomSettings>) => set({ geom: { ...g, ...patch } });
  // Board preview: the selected free parameter and its shown value.
  const [sel, setSel] = useState<number | null>(null);
  const [pv, setPv] = useState(0);
  const vars = p.geomVars.vars;
  useEffect(() => { setSel(null); }, [p.geometry.id, vars.length]);
  const pick = (i: number) => { setSel(i); setPv(vars[i].start); };

  const estimate = async () => {
    const same = p.runs.filter((r) => r.geometry === p.geometry.name && r.corpus === p.corpusName && r.profile === p.profileName && r.algo === s.algo && r.best_score !== null);
    const best = same.sort((a, b) => a.best_score! - b.best_score!)[0];
    if (!best) return;
    setBound("loading");
    try {
      const d = await api.run(best.id);
      setBound(d.result?.bound?.total ?? null);
    } catch {
      setBound(null);
    }
  };

  return (
    <Frame
      title="Search"
      control={<span className="hint">{p.running ? "locked while running" : ""}</span>}
      side={
        <fieldset className="plain" disabled={p.running}>
          <Sec id="search-target" title="Optimize" note="what Play changes">
            <div className="row">
              <label title="Layout: search key assignments on the chosen board. Board + layout: also search the board's shape (stagger, splay, curvature, tenting…), each candidate board with its own layout search. Needs a parametric board.">Optimize</label>
              <Seg value={board ? "board" : "layout"} onChange={(v) => set({ target: v, geom: g })} options={[["layout", "Layout"], ["board", "Board + layout"]]} />
              <span />
            </div>
            {board && !p.geometry.param && <div className="hint">{p.geometry.name} has no parameter file: pick a parametric board to run.</div>}
            {board && !p.handWeighted && <div className="hint">The profile weights no hand-model metric, so the board's shape only moves distance metrics. Turn on "Use hand in optimization" on the Hand tab.</div>}
          </Sec>
          {board && <GeomSide s={g} set={setGeom} />}
          {!board && <Sec id="search-algo" title="Algorithm" note="how the search moves">
            <div className="row">
              <label title="Search algorithm. Its own settings appear below.">Algorithm</label>
              <select value={s.algo} onChange={(e) => set({ algo: e.target.value })}>
                {algos.map(([a, n]) => <option key={a} value={a}>{n}</option>)}
              </select>
              <span />
            </div>
          </Sec>}
          <Sec id="search-start" title="Starting layout" note="where runs begin">
            <div className="row">
              <label title="Auto starts from whatever layout is loaded (the working model or your edits). Pick a named layout to always start from it.">Start from</label>
              <select value={start} onChange={(e) => {
                const v = e.target.value;
                set({ start: v });
                const l = p.layouts.find((x) => x.id === v);
                if (l) p.onLoad(l.chars, l.name);
              }}>
                <option value="auto">Auto · {p.label || "loaded layout"}</option>
                {p.layouts.map((l) => <option key={l.id} value={l.id}>{l.name}</option>)}
              </select>
              <span />
            </div>
            <div className="btns"><button className="btn outline small" onClick={() => setEditing(true)}>Edit layout</button>
              {p.pins && <span className="hint">pins: <span className="mono">{p.pins}</span></span>}</div>
          </Sec>
          <Sec id="search" title="Budget" note="how long, how wide">
            <div className="row">
              <label title={board ? "Stop after a wall-clock time or after a number of candidate boards (exactly reproducible)." : "Stop after a wall-clock time or after a number of evaluations (exactly reproducible)."}>Budget</label>
              <Seg value={budget} onChange={(b) => set({ budget: b })} options={[["time", "Time"], ["evals", board ? "Boards" : "Evaluations"]]} />
              <span />
            </div>
            {budget === "time"
              ? <Num label="Time" help={board ? "Wall-clock budget for the run, final layout polish of both boards included (a minute or more is useful)." : "Wall-clock budget for the run."} value={s.timeS} min={1} step={1} unit="s" onChange={(v) => set({ timeS: Math.max(1, v) })} />
              : board
                ? <Num label="Boards" help="Candidate boards to score; each gets its own layout search." value={g.evals} min={10} step={50} onChange={(v) => setGeom({ evals: Math.max(10, Math.round(v)) })} />
                : <Num label="Evaluations" help="Total layout evaluations across threads." value={s.evals ?? 50_000_000} min={1000} step={1_000_000} unit={fmtCount(s.evals ?? 50_000_000)} onChange={(v) => set({ evals: Math.max(1000, Math.round(v)) })} />}
            <Num label="Threads" help="Independent workers (restarts / islands) in parallel." value={s.threads} min={1} max={p.meta.threads} unit={`/${p.meta.threads}`} onChange={(v) => set({ threads: Math.min(p.meta.threads, Math.max(1, Math.round(v))) })} />
            <Num label="Seed" help="Runs are reproducible from the seed (exactly so with an evaluation budget)." value={s.seed} min={0} onChange={(v) => set({ seed: Math.max(0, Math.round(v)) })} />
          </Sec>
          {!board && <>
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
          <Sec id="search-stop" title="Stop confidence" note="stop early">
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
          </>}
        </fieldset>
      }
      main={
        editing ? (
          <LayoutEditor geometry={p.geometry} chars={p.chars} label={p.label} a={p.a} pins={p.pins} setPins={p.setPins}
            onSwap={(i, j) => { if (start !== "auto") set({ start: "auto" }); p.onSwap(i, j); }} canUndo={p.canUndo} onUndo={p.onUndo} onClose={() => setEditing(false)} />
        ) : board ? (
          <GeomPreview geometry={p.geometry} vars={vars} sel={sel} value={pv} setValue={setPv} chars={p.chars} />
        ) : (
          <div className="algoview">
            <div className="algoview-text"><h3>{algoName(s.algo)}</h3><p>{desc}</p></div>
            <div className="algoview-anim"><AlgoVisual algo={s.algo} /></div>
          </div>
        )
      }
      bottom={board
        ? <GeomParams s={g} set={setGeom} board={p.geometry.id} vars={vars} error={p.geomVars.error} sel={sel} onSel={pick} />
        : <div className="bottompad"><Estimates {...p} onEstimate={estimate} bound={bound} /></div>}
    />
  );
}
