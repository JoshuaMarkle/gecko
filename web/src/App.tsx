// App shell (context/web.md, frontend.md): navbar (input tabs | result tabs · play / pause /
// stop centred, run % beside · working model) over one view. Inputs: Board, Hand, Language,
// Profile, Search (three-panel layout). Results: Home, Explore, Analysis, Saved.

import { useCallback, useEffect, useMemo, useRef, useState, type ReactElement } from "react";
import { api, type Analysis, type CorpusInfo, type Geometry, type LayoutInfo, type Meta, type Profile, type Progress, type RunDetail, type RunSummary } from "./api";
import { AnalysisView } from "./components/AnalysisView";
import { ExploreView } from "./components/explore/ExploreView";
import { IconBoard, IconExplore, IconHand, IconHome, IconLanguage, IconPause, IconPlay, IconProfile, IconRuns, IconSearch, IconStats, IconStop } from "./components/Icons";
import { ResultsView } from "./components/ResultsView";
import { BoardView } from "./components/views/BoardView";
import { HandView } from "./components/views/HandView";
import { HomeView, type Block } from "./components/views/HomeView";
import { LanguageView } from "./components/views/LanguageView";
import { ProfileView } from "./components/views/ProfileView";
import { algoName, SearchView } from "./components/views/SearchView";
import { EMPTY, fitChars, fmt, fmtCount, fmtTime, layerSlots, swapChars } from "./format";
import { handDims, handLength } from "./hand";
import { useInputs, usePref } from "./inputs";
import { useLanguages } from "./languages";
import { asView, INPUT_TABS, RESULT_TABS, type InputTabName, type View } from "./tabs";
import { Scorer, wasmReady } from "./wasm";

const ICONS: Record<View, () => ReactElement> = {
  board: IconBoard, hand: IconHand, language: IconLanguage, profile: IconProfile, search: IconSearch,
  home: IconHome, explore: IconExplore, analysis: IconStats, saved: IconRuns,
};

// Pause bookkeeping mirrors the server: paused time does not use up the budget.
interface Live { id: string; algo: string; progress: Progress[]; running: boolean; started: number; budgetMs: number | null; budgetEvals: number | null; pausedAt: number | null; pausedMs: number }

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
    customBoards, setCustomBoards, constraints, setConstraints, hand, setHand, model, setModel,
  } = inputs;
  const [viewPref, setView] = usePref<string>("view", "home");
  const view = asView(viewPref);
  const [boardCustom, setBoardCustom] = usePref("boardCustom", false);
  const langs = useLanguages(corpora);

  const [scorer, setScorer] = useState<Scorer | null>(null);
  const [scorerKey, setScorerKey] = useState("");
  const [analysis, setAnalysis] = useState<Analysis | null>(null);
  const [refAnalysis, setRefAnalysis] = useState<Analysis | null>(null);
  /** Selected slot on the Analysis heatmap (an index into the flat layered string). */
  const [selected, setSelected] = useState<number | null>(null);
  const [live, setLive] = useState<Live | null>(null);
  const [runDetail, setRunDetail] = useState<RunDetail | null>(null);
  const [rank, setRank] = useState<number | null>(null);
  const esRef = useRef<EventSource | null>(null);

  // ---- initial load ----
  useEffect(() => {
    wasmReady().catch(() => {});
    (async () => {
      try {
        const [m, l, g, p, c, r] = await Promise.all([api.meta(), api.layouts(), api.geometries(), api.profiles(), api.corpora(), api.runs()]);
        setMeta(m); setLayouts(l); setGeometries(g); setProfiles(p); setCorpora(c); setRuns(r);
        const d = m.defaults;
        if (!g.some((x) => x.id === geomId) && !customBoards.some((x) => x.id === geomId)) setGeomId(d.geometry);
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

  const allBoards = useMemo(() => [...geometries, ...customBoards], [geometries, customBoards]);
  const geometry = allBoards.find((g) => g.id === geomId) ?? geometries[0];
  const isCustomBoard = customBoards.some((g) => g.id === geometry?.id);
  const corpus = corpora.find((c) => c.id === corpusId);
  const reference = layouts.find((l) => l.id === refId);
  const refName = reference?.name ?? "reference";

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
  // The 3D views show the base layer; `slots[0][key]` indexes `viewChars`.
  const slots = useMemo(() => (geometry ? layerSlots(geometry, viewChars) : [[]]), [geometry, viewChars]);
  const baseView = useMemo(() => {
    const a = [...viewChars];
    return {
      chars: slots[0].map((i) => (i === null ? EMPTY : a[i])).join(""),
      heat: slots[0].map((i) => (i === null ? 0 : analysis?.heat[i] ?? 0)),
      muted: slots[0].map((i) => i === null),
    };
  }, [slots, viewChars, analysis]);

  // ---- scoring (WASM first, server fallback; custom boards are WASM only) ----
  const scoreChars = useCallback(async (c: string): Promise<Analysis | null> => {
    if (!profile || !geometry || !corpus || !c) return null;
    if (scorer) return scorer.analyze(c, profile, 10);
    if (isCustomBoard) return null;
    return (await api.score({ geometry: geometry.id, corpus: corpus.id, profile_override: profile, chars: c })).analysis;
  }, [scorer, profile, geometry, corpus, isCustomBoard]);

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
  const swap = (i: number, j: number) => { setCharsRaw(swapChars(editChars, i, j)); setLabel("edited"); setRank(null); };
  /** Make a layout the working model (and the loaded layout runs start from on Auto). */
  const adopt = (c: string, l: string, score: number | null, run?: string) => {
    setCharsRaw(c); setLabel(l); setStartChars(c); setSelected(null);
    setModel({ chars: c, label: l, score, run });
  };

  /** Analysis heatmap: click two keys (slots) to swap them. */
  const onSlot = (i: number) => {
    if (live?.running) return;
    if (i < 0) { setSelected(null); return; }
    if (selected === null) { setSelected(i); return; }
    if (selected !== i) swap(selected, i);
    setSelected(null);
  };

  const refreshRuns = () => api.runs().then(setRuns).catch(() => {});

  const openRun = async (id: string, pickBest = true) => {
    try {
      const d = await api.run(id);
      setRunDetail(d);
      const b = d.result?.best[0];
      if (pickBest && b) { adopt(b.chars, `${d.config.params.algo} run #1`, b.score, id); setRank(0); }
    } catch (e) {
      setErr((e as Error).message);
    }
  };

  const startRun = async (explore = false) => {
    if (!profile || !geometry || !corpus) return;
    if (isCustomBoard) { setErr("Custom boards cannot be optimized yet: the server needs a geometry upload route. Pick a bundled board to run."); return; }
    setErr(null);
    const algo = explore ? "explore" : search.algo;
    const start = search.start && search.start !== "auto" ? layouts.find((l) => l.id === search.start)?.chars ?? chars : chars;
    const byEvals = !explore && search.budget === "evals";
    const evals = search.evals ?? 50_000_000;
    try {
      const r = await api.startRun({
        geometry: geometry.id, corpus: corpus.id, profile_override: profile, chars: start, algo,
        ...(byEvals ? { evals } : { time_ms: search.timeS * 1000 }),
        threads: search.threads, seed: search.seed, pins, sa_steps: search.saSteps, ga_pop: search.gaPop,
        memetic: search.algo === "memetic" ? {
          ga: { pop_size: search.memeticPop ?? 24 }, local_search: search.localSearch ?? "hc",
          local_steps: search.localSteps ?? 12, islands: search.islands ?? 0,
          migration_interval: search.migrationInterval ?? 5, migrants: search.migrants ?? 2,
        } : undefined,
      });
      setLive({ id: r.id, algo, progress: [], running: true, started: Date.now(), budgetMs: byEvals ? null : search.timeS * 1000, budgetEvals: byEvals ? evals : null, pausedAt: null, pausedMs: 0 });
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
  const paused = !!live?.running && live.pausedAt !== null;
  const activeMs = live ? (live.pausedAt ?? now) - live.started - live.pausedMs : 0;
  const liveP = live?.progress[live.progress.length - 1];
  const frac = !live ? null : !live.running ? 1
    : live.budgetEvals ? Math.min(1, (liveP?.evals ?? 0) / live.budgetEvals) : Math.min(1, activeMs / (live.budgetMs ?? 1));

  if (!meta || !geometry || !profile) {
    return <div className="empty" style={{ height: "100%" }}>{err ? <div className="notice error" style={{ maxWidth: 520 }}>{err}</div> : "Loading…"}</div>;
  }

  const pickRank = (i: number) => {
    const b = runDetail?.result?.best[i];
    if (b) { adopt(b.chars, `${runDetail!.config.params.algo} run #${i + 1}`, b.score, runDetail!.config.id); setRank(i); }
  };
  const hasModel = !!model && model.chars === chars;
  const shownLabel = live?.running ? "Live best" : hasModel ? model!.label : label;
  const baseProfile = profiles.find((x) => x.id === profileId);
  const profileEdited = !!baseProfile && JSON.stringify({ ...baseProfile, id: undefined }) !== JSON.stringify({ ...profile, id: undefined });

  // ---- navbar: working model / live status ----
  const status = live?.running
    ? liveP?.explore
      ? `${paused ? "Paused · " : ""}Explore · ${fmtCount(liveP.explore.optima)} optima · best ${fmt(liveP.best, 3)}`
      : `${paused ? "Paused · " : ""}${live.algo.toUpperCase()} · ${fmtCount(liveP?.evals ?? 0)} evals · best ${liveP ? fmt(liveP.best, 3) : "–"}`
    : null;
  const modelScore = model ? (model.chars === chars && analysis ? analysis.breakdown.score : model.score) : null;

  const blocks: Block[] = [
    { tab: "board", title: "Board", rows: [["Board", geometry.name], ["Keys", `${geometry.key.length}`]],
      flags: [["Custom board", isCustomBoard], ["Custom constraints", Object.values(constraints).some((c) => c.on)]] },
    { tab: "hand", title: "Hand", rows: [["Preset", hand.preset[0].toUpperCase() + hand.preset.slice(1)], ["Hand length", `${handLength(handDims(hand)).toFixed(0)} mm`]],
      flags: [["Hand", hand.use]] },
    { tab: "language", title: "Language", rows: [["Language", corpus?.name ?? "–"], ["Characters", fmtCount(corpus?.chars ?? 0)]] },
    { tab: "profile", title: "Profile", rows: [["Profile", `${profile.name}${profileEdited ? " (edited)" : ""}`],
      ["Weighted metrics", `${Object.values(profile.weights).filter((w) => w > 0).length}`]] },
    { tab: "search", title: "Search", rows: [["Algorithm", algoName(search.algo)],
      ["Budget", search.budget === "evals" ? `${fmtCount(search.evals ?? 50_000_000)} evals` : `${search.timeS}s`],
      ["Threads · seed", `${search.threads} · ${search.seed}`],
      ["Start", search.start && search.start !== "auto" ? layouts.find((l) => l.id === search.start)?.name ?? search.start : `Auto · ${label}`],
      ["Pins", pins || "none"]],
      flags: [["Stop early", search.stopConf != null]] },
  ];

  const sel = langs.selected;
  const canSummarize = !!sel && sel.local && sel.status !== "Summarized" && !!sel.sources?.length && langs.busy === null;

  return (
    <div className="shell">
      <header id="top">
        <nav className="views" aria-label="Tabs">
          {([["Inputs", INPUT_TABS], ["Results", RESULT_TABS]] as const).map(([group, tabs], gi) => (
            <div key={group} className="tabgroup">
              {gi > 0 && <span className="tabdivider" aria-hidden="true" />}
              <span className="grouplabel">{group}</span>
              <div className="toolgroup" aria-label={group}>
                {tabs.map(([v, name]) => {
                  const Icon = ICONS[v];
                  return <button key={v} className={`tool ${view === v ? "on" : ""}`} onClick={() => setView(v)} title={name} aria-label={name} aria-pressed={view === v}><Icon /></button>;
                })}
              </div>
            </div>
          ))}
        </nav>
        <div className="transportwrap">
          {!INPUT_TABS.some(([tab]) => tab === view) && <>
            <div className="transport" aria-label="Optimization">
              <button className="play" onClick={play} disabled={!corpus || (live?.running && !paused)} aria-label={paused ? "Resume" : "Play"}
                title={paused ? "Resume" : live?.running ? "Running" : view === "explore" ? `Explore the search space (${search.timeS}s)` : `Optimize: ${algoName(search.algo)}, ${search.budget === "evals" ? `${fmtCount(search.evals ?? 50_000_000)} evals` : `${search.timeS}s`}`}>
                <IconPlay />
              </button>
              <button className={`tool ${paused ? "on" : ""}`} onClick={() => setPaused(!paused)} disabled={!live?.running}
                title={paused ? "Resume" : "Pause"} aria-label="Pause" aria-pressed={paused}><IconPause /></button>
              <button className="tool" onClick={stopRun} disabled={!live?.running} title="Stop now (results are kept)" aria-label="Stop"><IconStop /></button>
            </div>
            {frac !== null && <span className="runpct" title={live?.running ? `${fmtTime(activeMs)} elapsed` : "done"}>{Math.round(100 * frac)}%</span>}
          </>}
        </div>
        <div className="navright">
          {view === "language" && sel?.local && (
            <button className="btn outline small" disabled={!canSummarize} onClick={() => sel && langs.summarize(sel.id)}
              title={!sel.sources?.length ? "Add sources first" : sel.status === "Summarized" ? "Statistics are current" : "Build statistics from the sources"}>
              {langs.busy ? "Summarizing…" : "Summarize"}
            </button>
          )}
          {view === "language" && sel && !sel.local && <button className="btn outline small" disabled title="Bundled languages are already summarized">Summarize</button>}
          <div className="model" title={model ? `Working model: ${model.label}` : "No working model yet: run an optimization or pick a saved layout"}>
            {status ? <span className="mono">{status}</span>
              : model ? <><b>{model.label}</b>{modelScore !== null && <span className="muted">{fmt(modelScore, 3)}</span>}</>
              : <span className="muted">No model</span>}
          </div>
        </div>
      </header>

      {err && <div className="errbar"><div className="notice error">{err}</div><button className="btn soft small" onClick={() => setErr(null)}>Dismiss</button></div>}

      <main className="page">
        {view === "board" && (
          <BoardView geometries={geometries} customBoards={customBoards} setCustomBoards={setCustomBoards} geometry={geometry} setGeomId={setGeomId}
            constraints={constraints} setConstraints={setConstraints} custom={boardCustom} setCustom={setBoardCustom} />
        )}
        {view === "hand" && <HandView geometry={geometry} chars={baseView.chars} hand={hand} setHand={setHand} />}
        {view === "language" && <LanguageView langs={langs} corpusId={corpusId} setCorpusId={setCorpusId} />}
        {view === "profile" && (
          <ProfileView meta={meta} profiles={profiles} profileId={profileId} setProfileId={setProfileId} profile={profile} setProfile={setProfile}
            geometry={geometry} chars={viewChars} layouts={layouts} a={analysis} />
        )}
        {view === "search" && (
          <SearchView meta={meta} layouts={layouts} geometry={geometry} corpusName={corpus?.name ?? ""} profileName={profile.name} search={search} setSearch={setSearch}
            running={!!live?.running} runs={runs} chars={editChars} label={hasModel ? model!.label : label} a={analysis} pins={pins} setPins={setPins}
            onSwap={swap} canUndo={chars !== startChars} onUndo={() => setChars(startChars, "start")} onLoad={(c, l) => setChars(c, l)} />
        )}
        {view === "home" && (
          <HomeView blocks={blocks} onGo={(t: InputTabName) => setView(t)} geometry={geometry} view={baseView} label={shownLabel} hasModel={hasModel}
            running={!!live?.running && live.algo !== "explore"} a={analysis} refA={refAnalysis} refName={refName} progress={progress} run={runDetail}
            runs={runs} corpusName={corpus?.name ?? ""} profileName={profile.name} />
        )}
        {view === "explore" && (
          <ExploreView meta={meta} runs={runs} geometries={geometries} corpora={corpora} layouts={layouts} profile={profile} setProfile={setProfile}
            scorer={scorer} scorerKey={scorerKey} search={search} setSearch={setSearch} onStart={() => startRun(true)}
            live={live && live.algo === "explore" ? { id: live.id, version: liveP?.explore?.version ?? 0, running: live.running } : null}
            onOpen={(c, l) => { adopt(c, l, null); setView("home"); }} />
        )}
        {view === "analysis" && (
          <AnalysisView geometry={geometry} chars={viewChars} label={shownLabel} a={analysis} refA={refAnalysis}
            refName={refName} selected={selected} onKey={onSlot} />
        )}
        {view === "saved" && (
          <ResultsView runs={runs} run={runDetail} progress={progress} rank={rank} refScore={refAnalysis?.breakdown.score} refName={refName} geometries={allBoards}
            onOpenRun={(id) => openRun(id, false)} onPickRank={pickRank} onShowHome={() => setView("home")} />
        )}
      </main>
    </div>
  );
}
