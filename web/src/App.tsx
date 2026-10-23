// App shell (context/web.md): navbar (Home | Inputs | Optimize | Saved tab groups · play / pause /
// stop centred, run % beside · working model) over one view. Home is the optimization flowchart;
// Inputs: Board, Hand, Language, Profile, Summary; Optimize: Layout, Board, Both, Explore (Play
// runs the open one); Saved: Runs, Export.

import { useCallback, useEffect, useMemo, useState, type ReactElement } from "react";
import { api, type Analysis, type CorpusInfo, type Geometry, type GeomRunDetail, type LayoutInfo, type Meta, type Profile, type RunDetail, type RunSummary } from "./api";
import { ExploreView } from "./components/explore/ExploreView";
import { collisionColors, stableFocus, useParamBoard } from "./components/geom/GeomBoard";
import { useGeomVars } from "./components/geom/GeomSearch";
import { IconBoard, IconExplore, IconExport, IconHand, IconHome, IconLanguage, IconOptBoard, IconOptBoth, IconOptLayout, IconPause, IconPlay, IconProfile, IconRuns, IconStop, IconSummary } from "./components/Icons";
import { BoardSearchView, type LiveBoard } from "./components/views/BoardSearchView";
import { BoardView } from "./components/views/BoardView";
import { ExportView } from "./components/views/ExportView";
import { HandView } from "./components/views/HandView";
import { HomeView } from "./components/views/HomeView";
import { LanguageView } from "./components/views/LanguageView";
import { algoName, LayoutView } from "./components/views/LayoutView";
import { ProfileView } from "./components/views/ProfileView";
import { SavedView } from "./components/views/SavedView";
import { SummaryView } from "./components/views/SummaryView";
import { sameInputs } from "./estimate";
import type { ExportInfo } from "./export";
import { EMPTY, fitChars, fmt, fmtCount, fmtTime, layerSlots, swapChars } from "./format";
import { asProgress, freeSpec, GEOM_DEFAULTS } from "./geom";
import { withHand } from "./hand";
import { useInputs, usePref } from "./inputs";
import { useLanguages } from "./languages";
import { useLiveRun } from "./liveRun";
import { asView, isOptimizeTab, TAB_GROUPS, type OptimizeTabName, type View } from "./tabs";
import { profileHash, Scorer, wasmReady } from "./wasm";

const ICONS: Record<View, () => ReactElement> = {
  home: IconHome, board: IconBoard, hand: IconHand, language: IconLanguage, profile: IconProfile, summary: IconSummary,
  layout: IconOptLayout, shape: IconOptBoard, both: IconOptBoth, explore: IconExplore, runs: IconRuns, export: IconExport,
};

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
    geomId, setGeomId, corpusId, setCorpusId, profileId, setProfileId: setProfileIdRaw, profile, setProfile, loadProfile, refId, setRefId,
    chars, setChars: setCharsRaw, label, setLabel, startChars, setStartChars, pins, setPins, search, setSearch,
    customBoards, setCustomBoards, constraints, setConstraints, hand, setHand, model, setModel,
  } = inputs;
  const [viewPref, setViewPref] = usePref<string>("view", "home");
  const view = asView(viewPref);
  /** The optimize tab used last: Home highlights it. */
  const [lastOpt, setLastOpt] = usePref<OptimizeTabName>("lastOptimize", "layout");
  const setView = (v: View) => { setViewPref(v); if (isOptimizeTab(v)) setLastOpt(v); };
  const [boardCustom, setBoardCustom] = usePref("boardCustom", false);
  const langs = useLanguages(corpora);

  const [scorer, setScorer] = useState<Scorer | null>(null);
  const [scorerKey, setScorerKey] = useState("");
  const [analysis, setAnalysis] = useState<Analysis | null>(null);
  const [refAnalysis, setRefAnalysis] = useState<Analysis | null>(null);
  const [runDetail, setRunDetail] = useState<RunDetail | null>(null);
  /** The opened run when it is a board run (then `runDetail` is null). */
  const [geomDetail, setGeomDetail] = useState<GeomRunDetail | null>(null);
  const [rank, setRank] = useState<number | null>(null);
  // Handlers are declared below; the hook calls them only after a run ends.
  const run = useLiveRun({
    onDone: (id, algo) => {
      if (algo === "geometry") api.geometries().then((g) => { setGeometries(g); openRun(id); }).catch((e) => setErr((e as Error).message));
      else if (algo !== "explore") openRun(id);
    },
    onError: setErr,
    onRunsChanged: () => refreshRuns(),
  });
  const { live, last: liveP, paused, activeMs, frac, setPaused } = run;

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
        loadProfile(p.find((x) => x.id === pid) ?? p[0] ?? null);
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

  // ---- past layout runs on exactly these inputs (hashes; custom boards and local languages never run) ----
  const [profHash, setProfHash] = useState<string | null>(null);
  useEffect(() => {
    let alive = true;
    if (profile) profileHash(profile).then((h) => alive && setProfHash(h)).catch(() => alive && setProfHash(null));
    return () => { alive = false; };
  }, [profile]);
  const matching = useMemo(() => sameInputs(runs.filter((r) => r.kind !== "geometry"), geometry?.hash && !isCustomBoard && corpus && profHash
    ? { geometry_hash: geometry.hash, corpus_hash: corpus.hash, profile_hash: profHash } : null), [runs, geometry, isCustomBoard, corpus, profHash]);

  // ---- board searches: free parameters (Board / Both tabs), the live board ----
  const geomTab = view === "shape" || view === "both";
  const geomSettings = { ...GEOM_DEFAULTS, ...search.geom };
  const geomVars = useGeomVars(geometry ?? { id: "", name: "", description: "", key: [] }, geomTab ? geomSettings.groups : []);
  const lg = live?.geom;
  const lgNames = useMemo(() => lg?.vars.map((v) => v.name) ?? [], [lg?.vars]);
  const lgStarts = useMemo(() => lg?.vars.map((v) => v.start), [lg?.vars]);
  const lgLast = lg?.progress[lg.progress.length - 1];
  const liveStart = useParamBoard(lg?.param, lgNames, lgStarts, "live-start");
  const liveNow = useParamBoard(lg?.param, lgNames, lgLast?.values ?? lgStarts, "live");

  const layoutLive = !!live?.running && live.algo !== "explore" && live.algo !== "geometry";
  const previewChars = layoutLive ? live!.progress[live!.progress.length - 1]?.best_chars : undefined;
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
  const setChars = (c: string, l: string) => { setCharsRaw(c); setLabel(l); setStartChars(c); setRank(null); };
  const setProfileId = (id: string) => { setProfileIdRaw(id); const p = profiles.find((x) => x.id === id); if (p) loadProfile(p); };
  const swap = (i: number, j: number) => { setCharsRaw(swapChars(editChars, i, j)); setLabel("edited"); setRank(null); };
  /** Make a layout the working model (and the loaded layout runs start from on Auto). */
  const adopt = (c: string, l: string, score: number | null, run?: string) => {
    setCharsRaw(c); setLabel(l); setStartChars(c);
    setModel({ chars: c, label: l, score, run });
  };

  const refreshRuns = () => api.runs().then(setRuns).catch(() => {});

  /** Make a board run's best board the board, and its layout the working model. */
  const adoptBoard = async (id: string, d?: GeomRunDetail) => {
    try {
      const detail = d ?? await api.geomRun(id);
      const best = detail.result?.best;
      if (!best) return;
      if (!geometries.some((g) => g.run === id)) setGeometries(await api.geometries());
      setGeomId(`run:${id}`);
      adopt(best.chars, detail.config.params.mode === "board" ? "board run" : "board + layout run", best.score);
      setRank(null);
    } catch (e) {
      setErr((e as Error).message);
    }
  };

  const openRun = async (id: string, pickBest = true) => {
    try {
      const d = await api.anyRun(id);
      if (d.kind === "geometry") {
        setGeomDetail(d); setRunDetail(null);
        if (pickBest) await adoptBoard(id, d);
        return;
      }
      setGeomDetail(null);
      setRunDetail(d);
      const b = d.result?.best[0];
      if (pickBest && b) { adopt(b.chars, `${d.config.params.algo} run #1`, b.score, id); setRank(0); }
    } catch (e) {
      setErr((e as Error).message);
    }
  };

  /** Start the run of an optimize tab with the shared inputs. */
  const startRun = async (kind: OptimizeTabName) => {
    if (!profile || !geometry || !corpus) return;
    if (isCustomBoard) { setErr("Custom boards cannot be optimized yet: the server needs a geometry upload route. Pick a bundled board to run."); return; }
    setErr(null);
    const start = search.start && search.start !== "auto" ? layouts.find((l) => l.id === search.start)?.chars ?? chars : chars;
    try {
      if (kind === "shape" || kind === "both") {
        if (!geometry.param) { setErr(`${geometry.name} has no parameter file, so its shape cannot be optimized. Pick a parametric board on the Board tab.`); return; }
        if (!geomVars.vars.length) { setErr(geomVars.error ?? "No free parameters: turn on a group under Board shape."); return; }
        const g = geomSettings;
        await run.start({
          geometry: geometry.id, corpus: corpus.id, profile_override: profile, chars: start, algo: "geometry",
          ...(g.budget === "evals" ? { evals: g.evals } : { time_ms: (g.timeS ?? 60) * 1000 }), threads: search.threads, seed: search.seed, pins,
          geom: { free: freeSpec(g, geometry.id, geomVars.vars), mode: kind === "shape" ? "board" : g.mode, inner_evals: g.innerEvals, margin: g.margin, penalty: g.penalty, keep_effort: g.keepEffort },
        }, geometry.param);
        return;
      }
      const explore = kind === "explore";
      await run.start({
        geometry: geometry.id, corpus: corpus.id, profile_override: profile, chars: start, algo: explore ? "explore" : search.algo,
        ...(!explore && search.budget === "evals" ? { evals: search.evals ?? 50_000_000 } : { time_ms: search.timeS * 1000 }),
        threads: search.threads, seed: search.seed, pins, sa_steps: search.saSteps, ga_pop: search.gaPop,
        holdout: search.holdout && search.holdout !== corpus.id ? search.holdout : undefined,
        memetic: search.algo === "memetic" ? {
          ga: { pop_size: search.memeticPop ?? 24 }, local_search: search.localSearch ?? "hc",
          local_steps: search.localSteps ?? 12, islands: search.islands ?? 0,
          migration_interval: search.migrationInterval ?? 5, migrants: search.migrants ?? 2,
        } : undefined,
      });
    } catch (e) {
      setErr((e as Error).message);
    }
  };

  const optTab = isOptimizeTab(view) ? view : null;
  const play = () => { if (!live?.running) { if (optTab) startRun(optTab); } else setPaused(false); };

  const progress = live && (live.running || (runDetail ?? geomDetail)?.config.id !== live.id) ? live.progress
    : runDetail?.progress ?? geomDetail?.progress.map(asProgress) ?? live?.progress ?? [];

  if (!meta || !geometry || !profile) {
    return <div className="empty" style={{ height: "100%" }}>{err ? <div className="notice error" style={{ maxWidth: 520 }}>{err}</div> : "Loading…"}</div>;
  }

  const pickRank = (i: number) => {
    const b = runDetail?.result?.best[i];
    if (b) { adopt(b.chars, `${runDetail!.config.params.algo} run #${i + 1}`, b.score, runDetail!.config.id); setRank(i); }
  };
  const hasModel = !!model && model.chars === chars;
  const shownLabel = layoutLive ? "Live best" : hasModel ? model!.label : label;
  const baseProfile = profiles.find((x) => x.id === profileId);
  const profileEdited = !!baseProfile && JSON.stringify({ ...withHand(baseProfile, hand), id: undefined }) !== JSON.stringify({ ...profile, id: undefined });
  /** The shown layout is the reference itself: comparing it with itself says nothing. */
  const isRef = !!reference && fitChars(reference.chars, geometry) === viewChars;

  // ---- navbar: working model / live status ----
  const status = live?.running
    ? lgLast
      ? `${paused ? "Paused · " : ""}Board · gen ${lgLast.generation} · ${fmtCount(lgLast.candidates)} boards · best ${fmt(lgLast.best, 3)}`
    : live.algo === "geometry" ? `${paused ? "Paused · " : ""}Board · starting…`
    : liveP?.explore
      ? `${paused ? "Paused · " : ""}Explore · ${fmtCount(liveP.explore.optima)} optima · best ${fmt(liveP.best, 3)}`
      : `${paused ? "Paused · " : ""}${live.algo.toUpperCase()} · ${fmtCount(liveP?.evals ?? 0)} evals · best ${liveP ? fmt(liveP.best, 3) : "–"}`
    : null;
  const modelScore = model ? (model.chars === chars && analysis ? analysis.breakdown.score : model.score) : null;
  const playTitle = paused ? "Resume" : live?.running ? "Running"
    : optTab === "explore" ? `Explore the search space (${search.timeS}s)`
    : optTab === "shape" || optTab === "both" ? `Optimize ${optTab === "shape" ? "the board" : "board + layout"}, ${geomSettings.budget === "evals" ? `${fmtCount(geomSettings.evals)} boards` : `${geomSettings.timeS ?? 60}s`}`
    : `Optimize the layout: ${algoName(search.algo)}, ${search.budget === "evals" ? `${fmtCount(search.evals ?? 50_000_000)} evals` : `${search.timeS}s`}`;

  // Export describes the shown layout; run facts only when it is that run's adopted layout.
  const modelRun = hasModel && model?.run && runDetail?.config.id === model.run ? runDetail : null;
  const holdout = modelRun?.result?.holdout;
  const exportInfo: ExportInfo = {
    name: shownLabel, geometry, chars: viewChars, a: analysis, corpus: corpus?.name, profile: profile.name,
    ref: refAnalysis && !isRef ? { name: refName, a: refAnalysis } : undefined,
    run: modelRun ? { id: modelRun.config.id, algo: modelRun.config.params.algo, holdout: holdout && { corpus: holdout.corpus, retained: holdout.retained } } : undefined,
  };

  const liveBoard: LiveBoard | undefined = live?.running && live.algo === "geometry" && liveNow && !liveNow.error ? {
    geometry: liveNow.geometry, ghost: liveStart?.geometry.key, colors: collisionColors(liveNow.geometry.key.length, liveNow.collisions),
    focus: liveStart ? stableFocus(liveStart.geometry.key) : undefined, collisions: liveNow.collisions.length,
    caption: lgLast ? `Live best board · generation ${lgLast.generation} · ${fmt(lgLast.best, 3)}` : "Start board",
  } : undefined;

  const sel = langs.selected;
  const canSummarize = !!sel && sel.local && sel.status !== "Summarized" && !!sel.sources?.length && langs.busy === null;

  return (
    <div className="shell">
      <header id="top">
        <nav className="views" aria-label="Tabs">
          {TAB_GROUPS.map(([group, tabs], gi) => (
            <div key={gi} className="tabgroup">
              {gi > 0 && <span className="tabdivider" aria-hidden="true" />}
              {group && <span className="grouplabel">{group}</span>}
              <div className="toolgroup" aria-label={group || "Home"}>
                {tabs.map(([v, name]) => {
                  const Icon = ICONS[v];
                  return <button key={v} className={`tool ${view === v ? "on" : ""}`} onClick={() => setView(v)} title={name} aria-label={name} aria-pressed={view === v}><Icon /></button>;
                })}
              </div>
            </div>
          ))}
        </nav>
        <div className="transportwrap">
          {(optTab || live?.running) && <>
            <div className="transport" aria-label="Optimization">
              <button className="play" onClick={play} disabled={!corpus || (live?.running ? !paused : !optTab || ((optTab === "shape" || optTab === "both") && !geometry?.param))} aria-label={paused ? "Resume" : "Play"} title={playTitle}>
                <IconPlay />
              </button>
              <button className={`tool ${paused ? "on" : ""}`} onClick={() => setPaused(!paused)} disabled={!live?.running}
                title={paused ? "Resume" : "Pause"} aria-label="Pause" aria-pressed={paused}><IconPause /></button>
              <button className="tool" onClick={run.stop} disabled={!live?.running} title="Stop now (results are kept)" aria-label="Stop"><IconStop /></button>
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
        {view === "home" && (
          <HomeView meta={meta} geometry={geometry} isCustomBoard={isCustomBoard} hand={hand} profile={profile} profileEdited={profileEdited} corpus={corpus}
            search={search} score={analysis?.breakdown.score ?? null} label={shownLabel} matching={matching.length} runs={runs} active={lastOpt} onGo={setView} />
        )}
        {view === "board" && (
          <BoardView geometries={geometries} customBoards={customBoards} setCustomBoards={setCustomBoards} geometry={geometry} setGeomId={setGeomId}
            constraints={constraints} setConstraints={setConstraints} custom={boardCustom} setCustom={setBoardCustom} />
        )}
        {view === "hand" && <HandView geometry={geometry} chars={baseView.chars} hand={hand} setHand={setHand} />}
        {view === "language" && <LanguageView langs={langs} corpusId={corpusId} setCorpusId={setCorpusId} />}
        {view === "profile" && (
          <ProfileView meta={meta} profiles={profiles} profileId={profileId} setProfileId={setProfileId} profile={profile} setProfile={setProfile} hand={hand}
            geometry={geometry} chars={viewChars} layouts={layouts} a={analysis} />
        )}
        {view === "summary" && <SummaryView inputs={inputs} meta={meta} geometry={geometry} corpus={corpus} profile={profile} layouts={layouts} />}
        {view === "layout" && (
          <LayoutView meta={meta} layouts={layouts} geometry={geometry} corpora={corpora} corpusId={corpusId} search={search} setSearch={setSearch}
            locked={!!live?.running} live={layoutLive} runs={matching} chars={editChars} label={hasModel ? model!.label : label} a={analysis}
            pins={pins} setPins={setPins} onSwap={swap} canUndo={chars !== startChars} onUndo={() => setChars(startChars, "start")} onLoad={(c, l) => setChars(c, l)}
            view={baseView} shownLabel={shownLabel} hasModel={hasModel} refA={refAnalysis} isRef={isRef} refName={refName} progress={progress} run={runDetail} />
        )}
        {(view === "shape" || view === "both") && (
          <BoardSearchView key={view} target={view === "shape" ? "board" : "both"} meta={meta} layouts={layouts} geometry={geometry} search={search} setSearch={setSearch}
            locked={!!live?.running} geomVars={geomVars} handWeighted={["G01", "G03", "G05", "G06"].some((m) => (profile.weights[m] ?? 0) !== 0)}
            chars={editChars} label={hasModel ? model!.label : label} pins={pins} onLoad={(c, l) => setChars(c, l)} liveBoard={liveBoard} />
        )}
        {view === "explore" && (
          <ExploreView meta={meta} runs={runs} geometries={geometries} corpora={corpora} layouts={layouts} profile={profile} setProfile={setProfile}
            scorer={scorer} scorerKey={scorerKey} search={search} setSearch={setSearch} onStart={() => startRun("explore")}
            live={live && live.algo === "explore" ? { id: live.id, version: liveP?.explore?.version ?? 0, running: live.running } : null}
            onOpen={(c, l) => { adopt(c, l, null); setView("layout"); }} />
        )}
        {view === "runs" && (
          <SavedView runs={runs} run={runDetail} progress={progress} rank={rank} refScore={refAnalysis?.breakdown.score} refName={refName} geometries={allBoards}
            onOpenRun={(id) => openRun(id, false)} onPickRank={pickRank} onShowLayout={() => setView("layout")}
            current={chars} onAdopt={(c, l, s) => { adopt(c, l, s, runDetail?.config.id); setRank(null); }}
            geomRun={geomDetail} onUseBoard={(id) => adoptBoard(id, geomDetail?.config.id === id ? geomDetail : undefined)} />
        )}
        {view === "export" && <ExportView info={exportInfo} label={shownLabel} onError={setErr} />}
      </main>
    </div>
  );
}
