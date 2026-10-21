// App shell (context/design.md): toolbar (tabs | transport | progress) over one tab.
// Tabs follow the design process: inputs (Board, Hand, Corpus, Profile, Search), then
// Optimize (layout | 3D view over charts | result), Explore, Stats (current layout), Runs.

import { useCallback, useEffect, useMemo, useRef, useState, type CSSProperties, type ReactElement } from "react";
import { api, type Analysis, type PairKind, type CorpusInfo, type Geometry, type LayoutInfo, type Meta, type Profile, type Progress, type RunDetail, type RunSummary } from "./api";
import { Convergence, FingerLoad, Heatmap, MetricBars, WorstNgrams } from "./components/Charts";
import { AnalysisView } from "./components/AnalysisView";
import { ExploreView } from "./components/explore/ExploreView";
import { IconBoard, IconCorpus, IconExplore, IconHand, IconOptimize, IconPause, IconPlay, IconProfile, IconRuns, IconSearch, IconSkip, IconStats } from "./components/Icons";
import { InputPage, LayoutPanel, type InputProps, type InputTab } from "./components/Inputs";
import { ResultPanel } from "./components/ResultPanel";
import { ResultsView } from "./components/ResultsView";
import { fingerLegend, PAIR_KINDS } from "./colors";
import { EMPTY, fitChars, fmt, fmtCount, fmtTime, LAYER_KEYS, layerSlots, rowsOf, swapChars, undirected } from "./format";
import { Keyboard3D, type Arc, type Overlay, type ViewMode } from "./three/Keyboard3D";
import { useInputs, usePref } from "./inputs";
import { Scorer } from "./wasm";

type Tab = "convergence" | "metrics" | "fingers" | "heatmap" | "ngrams";
const TABS: [Tab, string][] = [["convergence", "Convergence"], ["metrics", "Metrics"], ["fingers", "Finger load"], ["heatmap", "Heatmap"], ["ngrams", "Worst n-grams"]];
type View = InputTab | "optimize" | "explore" | "stats" | "runs";
// Two tracks: the inputs, then what you do with them.
const INPUT_VIEWS: [View, string, () => ReactElement][] = [
  ["board", "Board", IconBoard], ["hand", "Hand", IconHand], ["corpus", "Corpus", IconCorpus], ["profile", "Profile", IconProfile], ["search", "Search", IconSearch],
];
const WORK_VIEWS: [View, string, () => ReactElement][] = [
  ["optimize", "Optimize", IconOptimize], ["explore", "Explore the search space", IconExplore], ["stats", "Stats", IconStats], ["runs", "Runs", IconRuns],
];
const OLD_VIEWS: Record<string, View> = { home: "optimize", results: "runs", analysis: "stats" };
const asView = (v: string): View => OLD_VIEWS[v] ?? ([...INPUT_VIEWS, ...WORK_VIEWS].some(([x]) => x === v) ? (v as View) : "optimize");

function Gutter({ dir, onDrag, onReset }: { dir: "v" | "h"; onDrag: (delta: number) => void; onReset: () => void }) {
  const [drag, setDrag] = useState(false);
  return (
    <div className={`gutter ${dir} ${drag ? "drag" : ""}`} title="Drag to resize. Double-click to reset." onDoubleClick={onReset}
      onPointerDown={(e) => {
        e.preventDefault();
        let last = dir === "v" ? e.clientX : e.clientY;
        setDrag(true);
        document.body.classList.add("resizing");
        const move = (ev: PointerEvent) => { const cur = dir === "v" ? ev.clientX : ev.clientY; onDrag(cur - last); last = cur; };
        const up = () => { setDrag(false); document.body.classList.remove("resizing"); window.removeEventListener("pointermove", move); window.removeEventListener("pointerup", up); };
        window.addEventListener("pointermove", move);
        window.addEventListener("pointerup", up);
      }} />
  );
}

function download(name: string, text: string, type = "application/json") {
  const url = URL.createObjectURL(new Blob([text], { type }));
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  a.click();
  URL.revokeObjectURL(url);
}

// Pause bookkeeping mirrors the server: paused time does not use up the budget.
interface Live { id: string; algo: string; progress: Progress[]; running: boolean; started: number; budgetMs: number; pausedAt: number | null; pausedMs: number }

export function App() {
  const [meta, setMeta] = useState<Meta | null>(null);
  const [layouts, setLayouts] = useState<LayoutInfo[]>([]);
  const [geometries, setGeometries] = useState<Geometry[]>([]);
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [corpora, setCorpora] = useState<CorpusInfo[]>([]);
  const [runs, setRuns] = useState<RunSummary[]>([]);
  const [err, setErr] = useState<string | null>(null);

  const inputs = useInputs();
  const {
    geomId, setGeomId, corpusId, setCorpusId, profileId, setProfileId: setProfileIdRaw, profile, setProfile, refId, setRefId,
    chars, setChars: setCharsRaw, label, setLabel, startChars, setStartChars, pins, setPins, search, setSearch,
  } = inputs;
  const [viewPref, setView] = usePref<string>("view", "optimize");
  const view = asView(viewPref);
  const [tab, setTab] = usePref<Tab>("tab", "convergence");
  const [overlayPref, setOverlay] = usePref<Overlay | "sfb">("overlay", "none");
  const overlay: Overlay = overlayPref === "sfb" ? "ngrams" : overlayPref; // "sfb" was the old n-gram overlay
  const [pairOrder, setPairOrder] = usePref<"bigrams" | "skipgrams">("pairOrder", "bigrams");
  const [hiddenKinds, setHiddenKinds] = usePref<PairKind[]>("pairsHidden", ["same_hand", "alternate"]);
  const [mode, setMode] = usePref<ViewMode>("mode", "3d");
  const [sizes, setSizes] = usePref("sizes", { side: 0, res: 0, plot: 0 });

  const [scorer, setScorer] = useState<Scorer | null>(null);
  const [scorerKey, setScorerKey] = useState("");
  const [analysis, setAnalysis] = useState<Analysis | null>(null);
  const [refAnalysis, setRefAnalysis] = useState<Analysis | null>(null);
  /** Selected slot (an index into the flat layered string, not a key). */
  const [selected, setSelected] = useState<number | null>(null);
  const [layer, setLayer] = useState(0);
  const [live, setLive] = useState<Live | null>(null);
  const [runDetail, setRunDetail] = useState<RunDetail | null>(null);
  const [rank, setRank] = useState<number | null>(null);
  const [copied, setCopied] = useState<string | null>(null);
  const esRef = useRef<EventSource | null>(null);

  // ---- initial load ----
  useEffect(() => {
    (async () => {
      try {
        const [m, l, g, p, c, r] = await Promise.all([api.meta(), api.layouts(), api.geometries(), api.profiles(), api.corpora(), api.runs()]);
        setMeta(m); setLayouts(l); setGeometries(g); setProfiles(p); setCorpora(c); setRuns(r);
        const d = m.defaults;
        if (!g.some((x) => x.id === geomId)) setGeomId(d.geometry);
        if (!c.some((x) => x.id === corpusId)) setCorpusId(c.find((x) => x.id === d.corpus)?.id ?? c[0]?.id ?? "");
        const pid = p.some((x) => x.id === profileId) ? profileId : d.profile;
        setProfileIdRaw(pid);
        setProfile(p.find((x) => x.id === pid) ?? p[0] ?? null);
        if (!l.some((x) => x.id === refId)) setRefId(d.reference);
        const ref = l.find((x) => x.id === d.reference) ?? l[0];
        if (!chars && ref) { setCharsRaw(ref.chars); setLabel(ref.name); }
        setStartChars(chars || ref?.chars || "");
        if (search.threads > m.threads) setSearch({ ...search, threads: m.threads });
      } catch (e) {
        setErr(`Could not reach the gecko server: ${(e as Error).message}. Start it with \`gecko serve\`.`);
      }
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const geometry = geometries.find((g) => g.id === geomId) ?? geometries[0];
  const corpus = corpora.find((c) => c.id === corpusId);
  const reference = layouts.find((l) => l.id === refId);

  // ---- WASM scorer per (geometry, corpus) ----
  useEffect(() => {
    if (!geometry || !corpus) return;
    let alive = true;
    let made: Scorer | null = null;
    (async () => {
      try {
        const bytes = await api.corpusBytes(corpus.id);
        made = await Scorer.create(geometry, bytes);
        if (alive) { setScorer(made); setScorerKey(`${geometry.id}|${corpus.id}`); }
      } catch (e) {
        if (alive) setErr(`WASM scorer failed to load (${(e as Error).message}); falling back to the server.`);
      }
    })();
    return () => { alive = false; setScorer(null); made?.free(); };
  }, [geometry, corpus]);

  const previewChars = live?.running && live.algo !== "explore" ? live.progress[live.progress.length - 1]?.best_chars : undefined;
  const viewChars = geometry ? fitChars(previewChars ?? chars, geometry) : previewChars ?? chars;
  const editChars = geometry ? fitChars(chars, geometry) : chars;
  // Layers: the 3D view shows one layer at a time; `slots[layer][key]` indexes `viewChars`.
  const slots = useMemo(() => (geometry ? layerSlots(geometry, viewChars) : [[]]), [geometry, viewChars]);
  const shownLayer = Math.min(layer, slots.length - 1);
  const layerView = useMemo(() => {
    const a = [...viewChars];
    const s = slots[shownLayer];
    return {
      chars: s.map((i) => (i === null ? EMPTY : a[i])).join(""),
      heat: s.map((i) => (i === null ? 0 : analysis?.heat[i] ?? 0)),
      muted: s.map((i) => i === null),
    };
  }, [slots, shownLayer, viewChars, analysis]);

  // ---- scoring (WASM first, server fallback) ----
  const scoreChars = useCallback(async (c: string): Promise<Analysis | null> => {
    if (!profile || !geometry || !corpus || !c) return null;
    if (scorer) return scorer.analyze(c, profile, 10);
    return (await api.score({ geometry: geometry.id, corpus: corpus.id, profile_override: profile, chars: c })).analysis;
  }, [scorer, profile, geometry, corpus]);

  useEffect(() => {
    let alive = true;
    scoreChars(viewChars).then((a) => { if (alive) { setAnalysis(a); if (a) setErr((e) => (e?.startsWith("Layout") ? null : e)); } })
      .catch((e) => alive && setErr(`Layout cannot be scored: ${(e as Error).message}`));
    return () => { alive = false; };
  }, [scoreChars, viewChars]);

  useEffect(() => {
    let alive = true;
    if (reference) scoreChars(reference.chars).then((a) => alive && setRefAnalysis(a)).catch(() => alive && setRefAnalysis(null));
    return () => { alive = false; };
  }, [scoreChars, reference]);

  // ---- actions ----
  const setChars = (c: string, l: string) => { setCharsRaw(c); setLabel(l); setStartChars(c); setSelected(null); setRank(null); };
  const setProfileId = (id: string) => { setProfileIdRaw(id); const p = profiles.find((x) => x.id === id); if (p) setProfile(p); };

  /** `k` is a key of the shown layer (or a slot when `asSlot`, from the 2D heatmap). */
  const onKey = (k: number, asSlot = false) => {
    if (live?.running) return;
    if (k < 0) { setSelected(null); return; }
    const i = asSlot ? k : slots[shownLayer][k];
    if (i === null || i === undefined) return;
    if (selected === null) { setSelected(i); return; }
    if (selected !== i) { setCharsRaw(swapChars(editChars, selected, i)); setLabel("edited"); setRank(null); }
    setSelected(null);
  };
  const selectedKey = selected === null ? null : slots[shownLayer].indexOf(selected);

  const refreshRuns = () => api.runs().then(setRuns).catch(() => {});

  const openRun = async (id: string, pickBest = true) => {
    try {
      const d = await api.run(id);
      setRunDetail(d);
      if (pickBest && d.result?.best[0]) {
        setCharsRaw(d.result.best[0].chars); setLabel(`${d.config.params.algo} run #1`); setStartChars(d.result.best[0].chars); setRank(0); setSelected(null);
      }
      setTab("convergence");
    } catch (e) {
      setErr((e as Error).message);
    }
  };

  const startRun = async (explore = false) => {
    if (!profile || !geometry || !corpus) return;
    setErr(null);
    const algo = explore ? "explore" : search.algo;
    try {
      const r = await api.startRun({
        geometry: geometry.id, corpus: corpus.id, profile_override: profile, chars, algo, time_ms: search.timeS * 1000,
        threads: search.threads, seed: search.seed, pins, sa_steps: search.saSteps, ga_pop: search.gaPop,
      });
      setLive({ id: r.id, algo, progress: [], running: true, started: Date.now(), budgetMs: search.timeS * 1000, pausedAt: null, pausedMs: 0 });
      if (!explore) setTab("convergence");
      refreshRuns();
      esRef.current?.close();
      const es = new EventSource(`/api/runs/${encodeURIComponent(r.id)}/events`);
      esRef.current = es;
      es.addEventListener("progress", (e) => {
        const p = JSON.parse((e as MessageEvent).data) as Progress;
        setLive((l) => (l && l.id === r.id ? { ...l, progress: [...l.progress, p] } : l));
      });
      es.addEventListener("done", () => {
        es.close();
        setLive((l) => (l ? { ...l, running: false } : l));
        if (!explore) openRun(r.id);
        refreshRuns();
      });
      es.addEventListener("failed", (e) => {
        es.close();
        setLive((l) => (l ? { ...l, running: false } : l));
        setErr(JSON.parse((e as MessageEvent).data).error);
        refreshRuns();
      });
      es.onerror = () => { if (es.readyState === EventSource.CLOSED) setLive((l) => (l ? { ...l, running: false } : l)); };
    } catch (e) {
      setErr((e as Error).message);
    }
  };

  const stopRun = () => { if (live) api.cancel(live.id).catch(() => {}); };
  const setPaused = (on: boolean) => {
    if (!live?.running || (live.pausedAt !== null) === on) return;
    api.pause(live.id, on).then(() => setLive((l) => {
      if (!l || l.id !== live.id) return l;
      return on ? { ...l, pausedAt: Date.now() } : { ...l, pausedAt: null, pausedMs: l.pausedMs + Date.now() - (l.pausedAt ?? Date.now()) };
    })).catch((e) => setErr((e as Error).message));
  };
  const play = () => { if (!live?.running) startRun(view === "explore"); else setPaused(false); };

  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    if (!live?.running) return;
    const t = setInterval(() => setNow(Date.now()), 200);
    return () => clearInterval(t);
  }, [live?.running]);

  const progress = live && (live.running || runDetail?.config.id !== live.id) ? live.progress : runDetail?.progress ?? live?.progress ?? [];
  const lastP = progress[progress.length - 1];
  const paused = !!live?.running && live.pausedAt !== null;
  const activeMs = live ? (live.pausedAt ?? now) - live.started - live.pausedMs : 0;
  const liveP = live?.progress[live.progress.length - 1];
  const status = live?.running
    ? liveP?.explore
      ? `${paused ? "paused" : liveP.explore.phase === "probe" ? "probing landscape" : "exploring"} · ${fmtCount(liveP.explore.optima)} optima · best ${fmt(liveP.best, 3)}`
      : `${paused ? "paused" : "running"} · ${fmtCount(lastP?.evals ?? 0)} evals · best ${lastP ? fmt(lastP.best, 3) : "–"}`
    : runDetail?.result ? `done · ${runDetail.config.params.algo} · ${fmtTime(runDetail.result.elapsed_ms)}` : "idle";
  const frac = live?.running ? Math.min(1, activeMs / live.budgetMs) : live ? 1 : 0;

  // N-gram overlay: every pair of the chosen order, undirected, one arc each.
  const pairs = useMemo(() => undirected(analysis?.[pairOrder] ?? []), [analysis, pairOrder]);
  const kindTotals = useMemo(() => {
    const t = new Map<PairKind, number>();
    for (const p of pairs) t.set(p.kind, (t.get(p.kind) ?? 0) + p.freq);
    return t;
  }, [pairs]);
  const arcs = useMemo((): Arc[] => {
    // Width and opacity are relative to the most frequent visible pair.
    const shown = pairs.filter((p) => !hiddenKinds.includes(p.kind));
    const max = Math.max(...shown.map((p) => p.freq), 1e-9);
    return shown.map((p) => ({ ngram: p.ngram, w: Math.sqrt(p.freq / max), color: PAIR_KINDS[p.kind][1] }));
  }, [pairs, hiddenKinds]);
  const pairKinds: PairKind[] = pairOrder === "bigrams" ? ["sfb", "lsb", "scissor", "same_hand", "alternate"] : ["sfs", "same_hand", "alternate"];

  const shellStyle = useMemo(() => {
    const s: Record<string, string> = {};
    if (sizes.side) s["--side"] = `${sizes.side}px`;
    if (sizes.res) s["--res"] = `${sizes.res}px`;
    if (sizes.plot) s["--plot"] = `${sizes.plot}px`;
    return s as CSSProperties;
  }, [sizes]);
  const px = (v: string) => parseFloat(getComputedStyle(document.querySelector(".shell")!).getPropertyValue(v));

  const copy = (text: string, which: string) => { navigator.clipboard?.writeText(text).then(() => { setCopied(which); setTimeout(() => setCopied(null), 1200); }).catch(() => {}); };

  if (!meta || !geometry || !profile) {
    return <div className="empty" style={{ height: "100%" }}>{err ? <div className="notice error" style={{ maxWidth: 520 }}>{err}</div> : "Loading…"}</div>;
  }
  const pickRank = (i: number) => {
    const b = runDetail?.result?.best[i];
    if (b) { setCharsRaw(b.chars); setStartChars(b.chars); setRank(i); setLabel(`${runDetail!.config.params.algo} run #${i + 1}`); setSelected(null); }
  };
  const isBest = rank === 0 && label.includes("run #1");
  const rankLabel = rank !== null && runDetail ? `${runDetail.config.params.algo} run #${rank + 1}` : label;
  const inputTab = INPUT_VIEWS.some(([v]) => v === view) ? (view as InputTab) : null;
  const inputProps: InputProps = {
    meta, layouts, geometries, corpora, profiles, geometry, corpus, chars, setChars, startChars, pins, setPins, refId, setRefId,
    setGeomId, setCorpusId, profileId, setProfileId, profile, setProfile, search, setSearch, running: !!live?.running,
  };

  return (
    <div className="shell" style={shellStyle}>
      <header id="top">
        <nav className="views" aria-label="Tabs">
          {[INPUT_VIEWS, WORK_VIEWS].map((track, i) => (
            <div key={i} className="toolgroup" aria-label={i === 0 ? "Inputs" : "Work"}>
              {track.map(([v, name, Icon]) => (
                <button key={v} className={`tool ${view === v ? "on" : ""}`} onClick={() => setView(v)} title={name} aria-label={name} aria-pressed={view === v}><Icon /></button>
              ))}
            </div>
          ))}
        </nav>
        <div className="transport" aria-label="Optimization">
          <button className="start" onClick={play} disabled={!corpus || (live?.running && !paused)}
            title={paused ? "Resume" : live?.running ? "Running" : view === "explore" ? `Explore the search space (${search.timeS}s)` : `Run optimization (${search.algo}, ${search.timeS}s)`}>
            <IconPlay />{paused ? "Resume" : live?.running ? (live.algo === "explore" ? "Exploring…" : "Optimizing…") : view === "explore" ? "Start exploration" : "Start optimization"}</button>
          <button className={`tool ${paused ? "on" : ""}`} onClick={() => setPaused(!paused)} disabled={!live?.running}
            title={paused ? "Resume" : "Pause"} aria-label="Pause" aria-pressed={paused}><IconPause /></button>
          <button className="tool" onClick={stopRun} disabled={!live?.running} title="End optimization now (results are kept)" aria-label="End optimization"><IconSkip /></button>
        </div>
        <div className="runbox">
          <div className="statusline"><span>{status}</span><span>{live?.running ? `${fmtTime(activeMs)} / ${fmtTime(live.budgetMs)}` : ""}</span></div>
          <progress value={frac} max={1} />
        </div>
      </header>

      {inputTab && (
        <div className="page">
          <InputPage tab={inputTab} {...inputProps} />
        </div>
      )}

      {view === "optimize" && <>
      <aside id="left" className="side">
        <LayoutPanel {...inputProps} onGo={setView} />
        <Gutter dir="v" onDrag={(d) => setSizes({ ...sizes, side: Math.max(260, Math.min(640, px("--side") + d)) })} onReset={() => setSizes({ ...sizes, side: 0 })} />
      </aside>

      <main id="view">
        <Keyboard3D geometry={geometry} chars={layerView.chars} heat={layerView.heat} muted={layerView.muted} arcs={arcs} overlay={overlay} mode={mode}
          selected={selectedKey !== null && selectedKey >= 0 ? selectedKey : null} onKey={(k) => onKey(k)} />
        <div className="viewlabel">
          <span>{live?.running ? "live best" : rankLabel}</span>
          {analysis && <span className="muted">{fmt(analysis.breakdown.score, 3)}</span>}
          {isBest && !live?.running && <span className="badge best">Best</span>}
        </div>
        <div className="viewctl">
          {slots.length > 1 && (
            <div className="seg inline" aria-label="Layer">
              {slots.map((_, l) => (
                <button key={l} className={shownLayer === l ? "on" : ""} aria-pressed={shownLayer === l}
                  title={l === 0 ? "Base layer" : `Layer ${l}: hold ${LAYER_KEYS[l - 1]} (keys on that finger are unusable, shown dimmed)`}
                  onClick={() => setLayer(l)}>{l === 0 ? "Base" : `${LAYER_KEYS[l - 1]} Layer ${l}`}</button>
              ))}
            </div>
          )}
          <div className="seg inline">
            <button className={mode === "3d" ? "on" : ""} aria-pressed={mode === "3d"} title="3D view (off: flat top-down)" onClick={() => setMode(mode === "3d" ? "top" : "3d")}>3D</button>
          </div>
          <div className="seg inline">
            {([["none", "Default"], ["heat", "Heat"], ["fingers", "Fingers"], ["ngrams", "N-grams"]] as [Overlay, string][]).map(([o, n]) => <button key={o} className={overlay === o ? "on" : ""} onClick={() => setOverlay(o)}>{n}</button>)}
          </div>
        </div>
        <div className="viewhint">{selected !== null ? `Swap ${([...editChars][selected] ?? "").toUpperCase()} with… (click another key${slots.length > 1 ? ", on any layer" : ""})` : live?.running ? "Showing the best layout so far." : mode === "3d" ? "Click two keys to swap them. Drag to orbit." : "Click two keys to swap them."}</div>
        {overlay === "fingers" && <div className="legend">{fingerLegend.map(([n, c]) => <span key={n}><i style={{ background: c }} />{n}</span>)}</div>}
        {overlay === "heat" && <div className="legend"><span>darker = more keystrokes</span></div>}
        {overlay === "ngrams" && (
          <div className="legend">
            <div className="seg inline">
              {([["bigrams", "Bigrams"], ["skipgrams", "Skipgrams"]] as const).map(([o, n]) => <button key={o} className={pairOrder === o ? "on" : ""} onClick={() => setPairOrder(o)}>{n}</button>)}
            </div>
            {pairKinds.map((k) => (
              <button key={k} className={hiddenKinds.includes(k) ? "off" : ""} title="Show / hide" onClick={() => setHiddenKinds(hiddenKinds.includes(k) ? hiddenKinds.filter((x) => x !== k) : [...hiddenKinds, k])}>
                <i style={{ background: PAIR_KINDS[k][1] }} />{PAIR_KINDS[k][0]} <span className="muted">{fmt(kindTotals.get(k) ?? 0, 1)}%</span>
              </button>
            ))}
          </div>
        )}
      </main>

      <section id="plotwrap">
        <Gutter dir="h" onDrag={(d) => setSizes({ ...sizes, plot: Math.max(160, Math.min(window.innerHeight - 220, px("--plot") - d)) })} onReset={() => setSizes({ ...sizes, plot: 0 })} />
        <div className="tabs">
          <div className="seg inline">
            {TABS.map(([t, n]) => <button key={t} className={tab === t ? "on" : ""} onClick={() => setTab(t)}>{n}</button>)}
          </div>
        </div>
        <div className="plot">
          {tab === "convergence" && <Convergence progress={progress} reference={refAnalysis?.breakdown.score} refName={reference?.name ?? "reference"} />}
          {tab === "metrics" && analysis && <MetricBars cur={analysis} ref={refAnalysis ?? undefined} refName={reference?.name ?? "reference"} />}
          {tab === "fingers" && analysis && <FingerLoad a={analysis} refA={refAnalysis ?? undefined} refName={reference?.name ?? "reference"} />}
          {tab === "heatmap" && analysis && <Heatmap geometry={geometry} chars={viewChars} a={analysis} selected={selected} onKey={(i) => onKey(i, true)} />}
          {tab === "ngrams" && analysis && <WorstNgrams a={analysis} />}
        </div>
      </section>

      <aside id="right" className="side">
        <ResultPanel err={err} geometry={geometry} chars={viewChars} label={live?.running ? "Live best" : rankLabel} isBest={isBest && !live?.running} a={analysis} refA={refAnalysis}
          refName={reference?.name ?? "reference"} run={runDetail} rank={rank}
          onPickRank={pickRank}
          runs={runs} onOpenRun={(id) => openRun(id)}
          onExportJson={() => analysis && download(`gecko-${viewChars.length}.json`, JSON.stringify({ schema: 1, chars: viewChars, rows: rowsOf(viewChars, geometry), profile, analysis }, null, 2))}
          onExportToml={() => download("layout.toml", `name = "${rankLabel.replace(/"/g, "'")}"\ngeometry = "${geometry.name}"\nsource = "gecko web"\nrows = [${rowsOf(viewChars, geometry).map((r) => (r.includes("'") ? `"${r}"` : `'${r}'`)).join(", ")}]\n`, "application/toml")}
          onCopy={(t) => copy(t, t.startsWith("gecko") ? "cli" : "rows")} copied={copied} />
        <Gutter dir="v" onDrag={(d) => setSizes({ ...sizes, res: Math.max(280, Math.min(640, px("--res") - d)) })} onReset={() => setSizes({ ...sizes, res: 0 })} />
      </aside>
      </>}

      {view === "runs" && (
        <div className="page">
          <ResultsView runs={runs} run={runDetail} progress={progress} rank={rank} refScore={refAnalysis?.breakdown.score} refName={reference?.name ?? "reference"}
            onOpenRun={(id) => openRun(id)} onPickRank={pickRank} onShowOptimize={() => setView("optimize")} />
        </div>
      )}

      {view === "explore" && (
        <div className="page">
          <ExploreView meta={meta} runs={runs} geometries={geometries} corpora={corpora} layouts={layouts} profile={profile} setProfile={setProfile}
            scorer={scorer} scorerKey={scorerKey} search={search} setSearch={setSearch} onStart={() => startRun(true)}
            live={live && live.algo === "explore" ? { id: live.id, version: liveP?.explore?.version ?? 0, running: live.running } : null}
            onOpen={(c, l) => { setChars(c, l); setView("optimize"); }} />
        </div>
      )}

      {view === "stats" && (
        <div className="page">
          <AnalysisView geometry={geometry} chars={viewChars} label={live?.running ? "Live best" : rankLabel} a={analysis} refA={refAnalysis}
            refName={reference?.name ?? "reference"} selected={selected} onKey={(i) => onKey(i, true)} />
        </div>
      )}
    </div>
  );
}
