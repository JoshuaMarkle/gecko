// Explore view: the atlas of an explore run as linked panels. Left: atlas, ranking weights,
// focus and landscape numbers. Centre: map, fitness–distance, consensus, families, trade-offs,
// parallel coordinates, funnels, weight stability, landscape, discovery. Right: picked layout.
// Selection is shared: brushing any cloud focuses every panel; clicking picks one layout.
// Re-ranking under edited weights is exact and instant (raw metric vectors, gecko-wasm).
// research/studies/search-space-visualization.md §3, §6.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api, FINGERS, type AtlasSummary, type CorpusInfo, type Geometry, type LayoutInfo, type Meta, type Profile, type RunConfig, type RunSummary } from "../../api";
import { heatColor } from "../../colors";
import { logGrid, paretoFront, rankOrder, rowChars, sameTables, swapList, symIds, type Swap } from "../../explore";
import { fmt, fmtCount, fmtTime } from "../../format";
import { loadAtlasView, type AtlasView, type Scorer } from "../../wasm";
import type { SearchSettings } from "../../inputs";
import { ConsensusBoard } from "./Board";
import { Correlogram, Discovery, Families, Funnels, Landscape, ParallelCoords, Stability, stabilityRows, type Axis, type Family, type Funnel } from "./Panels";
import { Picked, type PickedInfo } from "./Picked";
import { Scatter, type Mark } from "./Scatter";

interface Data {
  id: string; summary: AtlasSummary; config: RunConfig; view: AtlasView; geometry: Geometry;
  n: number; slots: number; metrics: string[]; alphabet: string[];
  syms: Uint8Array; raw: Float32Array; hits: Uint32Array; firstMs: Uint32Array; flags: Uint16Array;
}

type Encoding = "slots" | "fingers" | "metrics";
const ENCODINGS: [Encoding, string, string][] = [
  ["slots", "Keys", "One-hot symbol × key: nearby points share many key positions"],
  ["fingers", "Fingers", "One-hot symbol × finger: nearby points type each letter with the same finger"],
  ["metrics", "Metrics", "Standardized metric values: nearby points type alike"],
];
const FACTORS = logGrid(0.25, 4, 25);
const LIGHT = "#e2e2e2";

function usePref<T>(key: string, init: T): [T, (v: T) => void] {
  const [v, setV] = useState<T>(() => {
    try { const s = localStorage.getItem(`gecko.explore.${key}`); return s ? (JSON.parse(s) as T) : init; } catch { return init; }
  });
  const set = useCallback((x: T) => { setV(x); try { localStorage.setItem(`gecko.explore.${key}`, JSON.stringify(x)); } catch { /* storage unavailable */ } }, [key]);
  return [v, set];
}

interface Props {
  meta: Meta;
  runs: RunSummary[];
  geometries: Geometry[];
  corpora: CorpusInfo[];
  layouts: LayoutInfo[];
  profile: Profile;
  setProfile: (p: Profile) => void;
  scorer: Scorer | null;
  scorerKey: string;
  /** The live explore run (id and atlas version from progress events), if any. */
  live: { id: string; version: number; running: boolean } | null;
  search: SearchSettings;
  setSearch: (s: SearchSettings) => void;
  onStart: () => void;
  onOpen: (chars: string, label: string) => void;
}

export function ExploreView(p: Props) {
  const explores = p.runs.filter((r) => r.algo === "explore");
  const [atlasId, setAtlasId] = usePref<string>("atlas", "");
  const [data, setData] = useState<Data | null>(null);
  const [loading, setLoading] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [rankBy, setRankBy] = usePref<"current" | "atlas">("rankBy", "atlas");
  const [topN, setTopN] = usePref("topN", 200);
  const [cut, setCut] = usePref("cut", 8);
  const [enc, setEnc] = usePref<Encoding>("encoding", "slots");
  const [tx, setTx] = usePref("tradeX", "B01");
  const [ty, setTy] = usePref("tradeY", "T03");
  const [brush, setBrush] = useState<number[] | null>(null);
  const [picked, setPicked] = useState<number | null>(null);
  const [swaps, setSwaps] = useState<Swap[] | null>(null);
  const [refs, setRefs] = useState<{ label: string; sym: Uint8Array; score: number }[]>([]);
  const [copied, setCopied] = useState(false);
  const lastLoad = useRef(0);

  // Follow a newly started explore run.
  useEffect(() => { if (p.live?.running && p.live.id !== atlasId) setAtlasId(p.live.id); }, [p.live?.id, p.live?.running]); // eslint-disable-line react-hooks/exhaustive-deps
  const id = atlasId && explores.some((r) => r.id === atlasId) ? atlasId : explores[0]?.id ?? "";
  const liveVersion = p.live && p.live.id === id ? p.live.version : 0;

  // ---- load (and reload while the run writes new snapshots) ----
  useEffect(() => {
    if (!id) { setData(null); return; }
    let alive = true;
    const wait = Math.max(0, 2500 - (Date.now() - lastLoad.current));
    const t = setTimeout(async () => {
      lastLoad.current = Date.now();
      setLoading(true);
      try {
        const [summary, detail] = await Promise.all([api.atlas(id), api.run(id)]);
        const [o, e, s] = await Promise.all([api.atlasFile(id, "optima"), api.atlasFile(id, "edges"), api.atlasFile(id, "samples")]);
        const geometry = p.geometries.find((g) => g.name === detail.config.geometry.name);
        if (!geometry) throw new Error(`board '${detail.config.geometry.name}' is not in data/geometries`);
        const view = await loadAtlasView(o, e, s);
        if (!alive) { view.free(); return; }
        setData((old) => {
          if (old && old.view !== view) old.view.free();
          return {
            id, summary, config: detail.config, view, geometry, n: view.count(), slots: view.slots(), metrics: JSON.parse(view.metric_ids()),
            alphabet: [...summary.chars], syms: view.syms(), raw: view.raw(), hits: view.hits(), firstMs: view.first_ms(), flags: view.flags(),
          };
        });
        setErr(null);
      } catch (x) {
        if (alive) setErr((x as Error).message);
      } finally {
        if (alive) setLoading(false);
      }
    }, wait);
    return () => { alive = false; clearTimeout(t); };
  }, [id, liveVersion, p.geometries]);
  useEffect(() => { setBrush(null); setPicked(null); }, [id]);

  const profile = rankBy === "current" || !data ? p.profile : data.config.profile;
  const profJson = useMemo(() => JSON.stringify(profile), [profile]);
  const tablesDiffer = !!data && rankBy === "current" && !sameTables(p.profile, data.config.profile);
  const defs = useMemo(() => new Map(p.meta.metrics.map((d) => [d.id, d])), [p.meta.metrics]);

  // ---- ranking under the chosen weights ----
  const ranked = useMemo(() => {
    if (!data || data.n === 0) return null;
    const scores = data.view.rescore(profJson);
    const order = rankOrder(scores);
    const rank = new Uint32Array(data.n);
    order.forEach((i, r) => { rank[i] = r; });
    const best = order[0];
    const tones: string[] = new Array(data.n);
    for (let i = 0; i < data.n; i++) {
      const t = Math.pow(1 - rank[i] / Math.max(data.n - 1, 1), 3);
      tones[i] = heatColor(0.1 + 0.9 * Math.round(t * 24) / 24);
    }
    const drawOrder = Uint32Array.from(order).reverse();
    const dBest = data.view.hamming_to(best);
    const sinks = data.view.funnels(scores);
    const samples = data.view.sample_scores(profJson);
    return { scores, order, rank, best, tones, drawOrder, dBest, sinks, samples };
  }, [data, profJson]);

  const brushSet = useMemo(() => (brush ? new Set(brush) : null), [brush]);
  const focus = useMemo(() => {
    if (!ranked) return new Uint32Array();
    if (brush) return Uint32Array.from([...brush].sort((a, b) => ranked.scores[a] - ranked.scores[b]));
    return ranked.order.slice(0, Math.min(topN, ranked.order.length));
  }, [ranked, brush, topN]);
  const tone = useCallback((i: number) => (!ranked ? null : brushSet && !brushSet.has(i) ? LIGHT : ranked.tones[i]), [ranked, brushSet]);
  const drawOrder = useMemo(() => {
    if (!ranked) return new Uint32Array();
    if (!brushSet) return ranked.drawOrder;
    const out = ranked.drawOrder.filter((i) => !brushSet.has(i));
    return Uint32Array.from([...out, ...ranked.drawOrder.filter((i) => brushSet.has(i))]);
  }, [ranked, brushSet]);
  const radius = useCallback((i: number) => (data ? 1.3 + Math.min(3, Math.log2(data.hits[i])) * 0.7 : 2), [data]);
  const pick = picked ?? ranked?.best ?? null;

  // ---- map ----
  const embed = useMemo(() => {
    if (!data || data.n === 0) return null;
    const finger = Uint8Array.from(data.geometry.key.map((k) => FINGERS.indexOf(k.finger)));
    const y = data.view.embed(enc, finger);
    const xs = new Float32Array(data.n), ys = new Float32Array(data.n);
    for (let i = 0; i < data.n; i++) { xs[i] = y[2 * i]; ys[i] = y[2 * i + 1]; }
    return { xs, ys, share: Array.from(data.view.embed_share()) };
  }, [data, enc]);

  // ---- fitness–distance (x jittered so integer distances do not stack) ----
  const fdcXs = useMemo(() => {
    if (!data || !ranked) return null;
    const xs = new Float32Array(data.n);
    for (let i = 0; i < data.n; i++) xs[i] = ranked.dBest[i] + (((i * 2654435761) >>> 0) / 4294967296 - 0.5) * 0.6;
    return xs;
  }, [data, ranked]);
  const fdc = useMemo(() => (data && ranked ? data.view.fdc(ranked.scores, new Uint32Array()) : NaN), [data, ranked]);

  // ---- reference layouts (scored by the server on the atlas's board and corpus) ----
  useEffect(() => {
    if (!data || !ranked) { setRefs([]); return; }
    let alive = true;
    const corpus = p.corpora.find((c) => c.name === data.config.corpus.name || c.id === data.config.corpus.name);
    const geomId = (data.geometry as Geometry & { id: string }).id;
    const t = setTimeout(async () => {
      const out: { label: string; sym: Uint8Array; score: number }[] = [];
      for (const l of p.layouts) {
        const sym = symIds(l.chars, data.summary.chars);
        if (!sym || !corpus || [...l.chars].length !== data.slots) continue;
        try {
          const r = await api.score({ geometry: geomId, corpus: corpus.id, profile_override: profile, chars: l.chars, top: 1 });
          out.push({ label: l.name, sym, score: r.analysis.breakdown.score });
        } catch { /* layout not valid on this board */ }
      }
      if (alive) setRefs(out);
    }, 300);
    return () => { alive = false; clearTimeout(t); };
  }, [data, ranked, profile, p.layouts, p.corpora]);
  const refMarks: Mark[] = useMemo(() => (data && ranked ? refs.map((r) => {
    const best = data.syms.subarray(ranked.best * data.slots, (ranked.best + 1) * data.slots);
    let d = 0;
    for (let k = 0; k < data.slots; k++) if (best[k] !== r.sym[k]) d++;
    return { x: d, y: r.score, label: r.label };
  }) : []), [data, ranked, refs]);

  // ---- consensus, families ----
  const cons = useMemo(() => (data && focus.length ? { p: data.view.consensus(focus), e: data.view.entropy(focus) } : null), [data, focus]);
  const families: Family[] = useMemo(() => {
    if (!data || !ranked || !focus.length) return [];
    const pool = focus.slice(0, 2000);
    const lab = data.view.families(pool, cut);
    const out: Family[] = [];
    pool.forEach((i, k) => {
      const f = lab[k];
      if (!out[f]) out[f] = { id: f, rows: [], leader: i, best: ranked.scores[i], worst: ranked.scores[i], d: ranked.dBest[i], chars: rowChars(data.syms, data.slots, i, data.alphabet) };
      out[f].rows.push(i);
      out[f].worst = Math.max(out[f].worst, ranked.scores[i]);
    });
    return out;
  }, [data, ranked, focus, cut]);
  const familyOf = useMemo(() => { const m = new Map<number, number>(); families.forEach((f) => f.rows.forEach((r) => m.set(r, f.id))); return m; }, [families]);

  // ---- funnels ----
  const funnels: Funnel[] = useMemo(() => {
    if (!data || !ranked) return [];
    const g = new Map<number, Funnel>();
    for (let i = 0; i < data.n; i++) {
      const b = ranked.sinks[i];
      let f = g.get(b);
      if (!f) { f = { bottom: b, rows: 0, hits: 0, score: ranked.scores[b], d: ranked.dBest[b] }; g.set(b, f); }
      f.rows++;
      f.hits += data.hits[i];
    }
    return [...g.values()].sort((a, b) => a.score - b.score);
  }, [data, ranked]);
  const funnelRank = useMemo(() => new Map(funnels.map((f, k) => [f.bottom, k])), [funnels]);
  const totalHits = useMemo(() => (data ? data.hits.reduce((a, b) => a + b, 0) : 0), [data]);

  // ---- weight stability ----
  const active = useMemo(() => (data ? data.metrics.map((id, k) => ({ id, k, def: defs.get(id) })).filter((x) => x.def && x.def.dir !== "info" && (profile.weights[x.id] ?? 0) > 0) : []), [data, defs, profile]);
  const stab = useMemo(() => {
    if (!data || !ranked || !active.length) return [];
    const ks = active.map((a) => a.k);
    const w = data.view.stability(profJson, Uint32Array.from(ks), Float64Array.from(FACTORS));
    return stabilityRows(w, ks, active.map((a) => a.def!), FACTORS, ranked.best);
  }, [data, ranked, active, profJson]);

  // ---- trade-off + parallel coordinates ----
  const col = useCallback((k: number) => { const v = new Float32Array(data!.n); for (let i = 0; i < data!.n; i++) v[i] = data!.raw[i * data!.metrics.length + k]; return v; }, [data]);
  const trade = useMemo(() => {
    if (!data || !ranked) return null;
    const kx = Math.max(0, data.metrics.indexOf(tx)), ky = Math.max(0, data.metrics.indexOf(ty));
    const xs = col(kx), ys = col(ky);
    const dx = defs.get(data.metrics[kx]), dy = defs.get(data.metrics[ky]);
    const front = paretoFront(xs, ys, focus, dx?.dir !== "max", dy?.dir !== "max");
    return { xs, ys, front, dx, dy };
  }, [data, ranked, tx, ty, col, focus, defs]);
  const axes: Axis[] = useMemo(() => {
    if (!data || !ranked) return [];
    const a: Axis[] = [{ key: "score", label: "Score", values: ranked.scores, lowerBetter: true }];
    for (const m of active.slice(0, 11)) a.push({ key: m.id, label: m.def!.name, values: col(m.k), lowerBetter: m.def!.dir !== "max" });
    return a;
  }, [data, ranked, active, col]);
  const context = useMemo(() => {
    if (!ranked) return new Uint32Array();
    const inFocus = new Set(focus);
    const rest = ranked.order.filter((i) => !inFocus.has(i));
    const step = Math.max(1, Math.floor(rest.length / 600));
    return rest.filter((_, k) => k % step === 0);
  }, [ranked, focus]);

  // ---- picked layout ----
  const info: PickedInfo | null = useMemo(() => {
    if (!data || !ranked || pick === null || pick >= data.n) return null;
    const m = data.metrics.length;
    const prof = JSON.parse(profJson) as Profile;
    const w = (id: string) => (prof.weights[id] ?? 0) / (prof.scale?.[id] ?? 1);
    const centre = (id: string) => (id === "M04" ? 50 : id === "M09" ? prof.thumb_target ?? 18 : null);
    const contrib = (id: string, dir: string, v: number) => {
      const c = centre(id);
      return id === "M02" ? w(id) * v * v : c !== null ? (w(id) * (v - c) * (v - c)) / 10 : dir === "max" ? -w(id) * v : w(id) * v;
    };
    return {
      row: pick, rank: ranked.rank[pick], total: data.n, chars: rowChars(data.syms, data.slots, pick, data.alphabet),
      bestChars: rowChars(data.syms, data.slots, ranked.best, data.alphabet), score: ranked.scores[pick], best: ranked.scores[ranked.best],
      hits: data.hits[pick], firstMs: data.firstMs[pick], constrained: (data.flags[pick] & 1) === 1, dBest: ranked.dBest[pick],
      funnel: funnelRank.get(ranked.sinks[pick]) ?? null, family: familyOf.get(pick) ?? null,
      metrics: active.map((a) => ({ def: a.def!, value: data.raw[pick * m + a.k], bestValue: data.raw[ranked.best * m + a.k], contrib: contrib(a.id, a.def!.dir, data.raw[pick * m + a.k]) })),
    };
  }, [data, ranked, pick, profJson, active, funnelRank, familyOf]);

  useEffect(() => {
    if (!info || !data) { setSwaps(null); return; }
    let alive = true;
    setSwaps(null);
    const corpus = p.corpora.find((c) => c.name === data.config.corpus.name || c.id === data.config.corpus.name);
    const geomId = (data.geometry as Geometry & { id: string }).id;
    (async () => {
      let mtx: ArrayLike<number> | null = null;
      if (p.scorer && corpus && p.scorerKey === `${geomId}|${corpus.id}`) mtx = p.scorer.swapMatrix(info.chars, profile);
      else if (corpus) mtx = (await api.swapMatrix({ geometry: geomId, corpus: corpus.id, profile_override: profile, chars: info.chars })).matrix;
      if (alive && mtx) setSwaps(swapList(mtx, data.slots));
    })().catch(() => alive && setSwaps([]));
    return () => { alive = false; };
  }, [info?.chars, profile, data, p.scorer, p.scorerKey, p.corpora]); // eslint-disable-line react-hooks/exhaustive-deps

  const describe = (row: number | null) => (row === null || !ranked || !data ? "stays best to the end of the range" : `#${ranked.rank[row] + 1} (${fmt(ranked.scores[row], 3)} now, ${ranked.dBest[row]} keys differ)`);
  const near = useMemo(() => (data && ranked ? Array.from(ranked.scores, (s) => s <= ranked.scores[ranked.best] + 0.01 * Math.abs(ranked.scores[ranked.best])) : []), [data, ranked]);
  const coverage = useMemo(() => {
    if (!data || !ranked) return null;
    const rows = Uint32Array.from(near.flatMap((x, i) => (x && !(data.flags[i] & 1) ? [i] : [])));
    return Array.from(data.view.chao1(rows));
  }, [data, ranked, near]);
  const sampleRandom = useMemo(() => {
    if (!ranked || ranked.samples.length === 0) return null;
    const s = ranked.samples, n = s.length;
    const mean = s.reduce((a, b) => a + b, 0) / n;
    return { mean, sd: Math.sqrt(s.reduce((a, b) => a + (b - mean) ** 2, 0) / n) };
  }, [ranked]);

  const tip = (i: number) => data && ranked && (
    <>
      <div><b>#{ranked.rank[i] + 1}</b> · {fmt(ranked.scores[i], 3)} · {ranked.dBest[i]} keys from best · {data.hits[i]} hit{data.hits[i] === 1 ? "" : "s"}</div>
      <div className="mono">{rowChars(data.syms, data.slots, i, data.alphabet).replace(/(.{10})/g, "$1 ").trim()}</div>
    </>
  );
  const onPick = (i: number) => setPicked(i);
  const selectRows = (rows: number[], lead: number) => { setBrush(rows); setPicked(lead); };
  const scoredDefs = p.meta.metrics.filter((d) => d.dir !== "info");
  const running = !!p.live?.running && p.live.id === id;

  return (
    <div className="explore">
      <aside className="pane xleft">
        <div className="panehead"><b>Explore</b><span className="hint">map the space of good layouts</span></div>
        <div className="scroll">
          <details className="sec" open>
            <summary>Atlas<span className="note">{explores.length} explore runs</span></summary>
            <div className="body">
              <div className="row wide">
                <select value={id} onChange={(e) => setAtlasId(e.target.value)}>
                  {explores.length === 0 && <option value="">No explore runs yet</option>}
                  {explores.map((r) => <option key={r.id} value={r.id}>{r.created.replace("T", " ").replace("Z", "")} · {r.profile} · {r.status === "running" ? "running" : r.best_score !== null ? fmt(r.best_score, 3) : r.status}</option>)}
                </select>
              </div>
              {data && <div className="hint">{data.config.geometry.name} · {data.config.corpus.name} · {data.config.profile.name}{data.config.pins ? ` · pins ${data.config.pins}` : ""} · {fmtCount(data.summary.evals)} evals · {fmtTime(data.summary.elapsed_ms)}{running ? " · live" : ""}</div>}
              {loading && !data && <div className="hint">Loading atlas…</div>}
              {err && <div className="notice error">{err}</div>}
            </div>
          </details>
          <details className="sec" open>
            <summary>New exploration<span className="note">uses the Board, Corpus and Profile tabs and the Optimize layout</span></summary>
            <div className="body">
              <div className="row"><label title="Wall-clock budget">Budget</label><input type="number" min={5} value={p.search.timeS} onChange={(e) => p.setSearch({ ...p.search, timeS: Math.max(5, +e.target.value || 60) })} /><span className="unit">s</span></div>
              <div className="row"><label title="Worker threads">Threads</label><input type="number" min={1} max={p.meta.threads} value={p.search.threads} onChange={(e) => p.setSearch({ ...p.search, threads: Math.max(1, Math.min(p.meta.threads, +e.target.value || 1)) })} /><span className="unit" /></div>
              <div className="row"><label title="Random seed">Seed</label><input type="number" value={p.search.seed} onChange={(e) => p.setSearch({ ...p.search, seed: +e.target.value || 1 })} /><span className="unit" /></div>
              <div className="row wide"><button className="btn primary" onClick={p.onStart} disabled={!!p.live?.running}>{p.live?.running ? "Running…" : "Start exploration"}</button></div>
            </div>
          </details>
          <details className="sec" open>
            <summary>Ranking<span className="note">re-scores the whole atlas, no new search</span></summary>
            <div className="body">
              <div className="seg">
                <button className={rankBy === "atlas" ? "on" : ""} onClick={() => setRankBy("atlas")} title="The weights the atlas was explored with">Atlas profile</button>
                <button className={rankBy === "current" ? "on" : ""} onClick={() => setRankBy("current")} title="The Profile tab (edit weights below or on that tab)">Current profile</button>
              </div>
              {tablesDiffer && <div className="notice warn">Finger targets, SFB settings or trigram coverage differ from the atlas; only the weights are applied.</div>}
              {rankBy === "current" && scoredDefs.map((d) => (
                <div className="row" key={d.id}>
                  <label title={`${d.id}: ${d.desc}`}>{d.name}</label>
                  <input type="number" step={0.1} min={0} value={+(p.profile.weights[d.id] ?? 0).toPrecision(6)}
                    onChange={(e) => { const v = Math.max(0, parseFloat(e.target.value) || 0); p.setProfile({ ...p.profile, weights: { ...p.profile.weights, [d.id]: v } }); }} />
                  <span className="unit">{p.profile.scale?.[d.id] ? "σ" : ""}</span>
                </div>
              ))}
              {rankBy === "atlas" && <div className="hint">Switch to Current profile to edit weights; the map stays put and every panel re-ranks.</div>}
            </div>
          </details>
          <details className="sec" open>
            <summary>Focus<span className="note">{brush ? `${brush.length} brushed` : `best ${Math.min(topN, data?.n ?? 0)}`}</span></summary>
            <div className="body">
              <div className="row"><label title="Consensus, families, trade-off front and parallel coordinates use the best N layouts unless a brush is active">Best N</label>
                <input type="range" min={20} max={2000} step={10} value={topN} onChange={(e) => setTopN(+e.target.value)} /><span className="unit">{topN}</span></div>
              <div className="row"><label title="A family is every layout within this many differing keys of its leader (the family's best)">Family radius</label>
                <input type="range" min={2} max={16} value={cut} onChange={(e) => setCut(+e.target.value)} /><span className="unit">{cut}</span></div>
              {brush && <div className="row wide"><button className="btn soft small" onClick={() => setBrush(null)}>Clear brush ({brush.length})</button></div>}
              <div className="hint">Drag on a scatter or an axis to brush; double-click to clear; click a point to inspect it.</div>
            </div>
          </details>
          {data && ranked && (
            <details className="sec" open>
              <summary>Landscape</summary>
              <div className="body">
                <div className="kv"><span>Local optima</span><span>{data.n - data.summary.counts.constrained} (+{data.summary.counts.constrained} constrained)</span></div>
                <div className="kv"><span>Descents · edges</span><span>{fmtCount(data.summary.counts.hits)} · {fmtCount(data.summary.counts.edges)}</span></div>
                <div className="kv" title="Sinks of the best-improving-edge forest under the current ranking"><span>Funnels</span><span>{funnels.length}</span></div>
                <div className="kv" title="Correlation between score and distance to the best. Strong positive = one big valley; weak = many separate good designs"><span>FDC</span><span>{fmt(fdc, 3)}</span></div>
                <div className="kv" title="Swaps after which scores decorrelate (random walks)"><span>Corr. length</span><span>{fmt(data.summary.probe.corr_length, 1)} swaps</span></div>
                {sampleRandom && <div className="kv"><span>Random layouts</span><span>{fmt(sampleRandom.mean)} ± {fmt(sampleRandom.sd)}</span></div>}
                {coverage && <div className="kv" title="Optima within 1% of the best: seen, and Chao1 estimate of how many exist"><span>Within 1%</span><span>{coverage[0]} seen · ~{fmt(coverage[1], 0)} est.</span></div>}
                {data.summary.symmetric && <div className="hint">Mirror images are merged (scores are left/right symmetric).</div>}
              </div>
            </details>
          )}
        </div>
      </aside>

      <main className="xcentre">
        {!data || !ranked ? (
          <div className="empty">{explores.length === 0 ? "No explore runs yet. Set a budget and press Start exploration." : loading ? "Loading atlas…" : err ?? "Waiting for the first snapshot…"}</div>
        ) : (
          <div className="cards">
            <section className="card">
              <h3>Map<span className="hint">PCA · {embed ? `${fmt(100 * embed.share[0], 0)}% + ${fmt(100 * embed.share[1], 0)}% of variance` : ""} · darker = better</span>
                <div className="seg inline right">{ENCODINGS.map(([e, n, t]) => <button key={e} className={enc === e ? "on" : ""} title={t} onClick={() => setEnc(e)}>{n}</button>)}</div></h3>
              <div className="chartbox tall">{embed && <Scatter n={data.n} xs={embed.xs} ys={embed.ys} tone={tone} order={drawOrder} radius={radius} picked={pick} best={ranked.best} map onPick={onPick} onBrush={setBrush} tooltip={tip} />}</div>
            </section>
            <section className="card">
              <h3>Fitness vs distance<span className="hint">FDC r = {fmt(fdc, 3)} · × reference layouts</span></h3>
              <div className="chartbox tall">{fdcXs && <Scatter n={data.n} xs={fdcXs} ys={ranked.scores} tone={tone} order={drawOrder} radius={radius} picked={pick} best={ranked.best} marks={refMarks}
                xLabel="keys different from the best" yLabel="score (lower is better)" onPick={onPick} onBrush={setBrush} tooltip={tip} />}</div>
            </section>
            <section className="card">
              <h3>Consensus<span className="hint">{focus.length} layouts · dark = all agree · most common symbol and its share · hover for the rest</span></h3>
              <div className="chartbox">{cons && <ConsensusBoard geometry={data.geometry} cons={cons.p} ent={cons.e} alphabet={data.alphabet} current={null} />}</div>
            </section>
            <section className="card">
              <h3>Families<span className="hint">{families.length} among {Math.min(focus.length, 2000)} · ≤ {cut} keys from the leader · grey = differs from best</span></h3>
              <div className="cardbody scrolly"><Families families={families} geometry={data.geometry} bestChars={info?.bestChars ?? ""} bestScore={ranked.scores[ranked.best]} picked={pick}
                onSelect={(f) => selectRows(f.rows, f.leader)} /></div>
            </section>
            <section className="card">
              <h3>Trade-off<span className="hint">line: Pareto front of the focus</span>
                <span className="right" style={{ display: "flex", gap: 4 }}>
                  <select value={tx} onChange={(e) => setTx(e.target.value)}>{data.metrics.map((m) => <option key={m} value={m}>{defs.get(m)?.name ?? m}</option>)}</select>
                  <select value={ty} onChange={(e) => setTy(e.target.value)}>{data.metrics.map((m) => <option key={m} value={m}>{defs.get(m)?.name ?? m}</option>)}</select>
                </span></h3>
              <div className="chartbox tall">{trade && <Scatter n={data.n} xs={trade.xs} ys={trade.ys} tone={tone} order={drawOrder} radius={radius} picked={pick} best={ranked.best} line={trade.front}
                xLabel={`${trade.dx?.name ?? tx} ${trade.dx?.dir === "max" ? "(higher is better)" : "(lower is better)"}`} yLabel={`${trade.dy?.name ?? ty} ${trade.dy?.dir === "max" ? "(higher is better)" : "(lower is better)"}`}
                onPick={onPick} onBrush={setBrush} tooltip={tip} />}</div>
            </section>
            <section className="card">
              <h3>Funnels<span className="hint">{funnels.length} bottoms · each optimum follows its best improving kick</span></h3>
              <div className="cardbody scrolly"><Funnels funnels={funnels} totalHits={totalHits} picked={pick} onSelect={(f) => {
                const rows: number[] = []; for (let i = 0; i < data.n; i++) if (ranked.sinks[i] === f.bottom) rows.push(i); selectRows(rows, f.bottom);
              }} /></div>
            </section>
            <section className="card wide">
              <h3>Parallel coordinates<span className="hint">focus in ink, a sample of the rest faint, picked in blue · better is down on every axis</span></h3>
              <div className="chartbox">{axes.length > 1 && <ParallelCoords axes={axes} focus={focus} context={context} picked={pick} onBrush={(r) => setBrush(r)} />}</div>
            </section>
            <section className="card">
              <h3>Weight stability<span className="hint">hover a row for the layout that takes over</span></h3>
              <div className="cardbody"><Stability rows={stab} factors={FACTORS} describe={describe} /></div>
            </section>
            <section className="card">
              <h3>Discovery<span className="hint">flattening = the archive is nearly complete</span></h3>
              <div className="chartbox"><Discovery firstMs={data.firstMs} near={near} elapsedMs={data.summary.elapsed_ms} /></div>
            </section>
            <section className="card">
              <h3>How good is good<span className="hint">score of random layouts vs local optima, current weights</span></h3>
              <div className="chartbox"><Landscape random={ranked.samples} optima={ranked.scores} refs={refs.map((r) => ({ label: r.label, score: r.score }))} /></div>
            </section>
            <section className="card">
              <h3>Ruggedness</h3>
              <div className="chartbox"><Correlogram rho={data.summary.probe.rho} ell={data.summary.probe.corr_length} /></div>
            </section>
          </div>
        )}
      </main>

      <aside className="pane xright">
        <div className="panehead"><b>Layout</b><span className="hint">lower score is better</span></div>
        <div className="scroll">
          {data && <Picked p={info} geometry={data.geometry} swaps={swaps} copied={copied}
            onOpen={() => info && p.onOpen(info.chars, `atlas #${info.rank + 1}`)}
            onCopy={() => info && navigator.clipboard?.writeText(info.chars).then(() => { setCopied(true); setTimeout(() => setCopied(false), 1200); })} />}
        </div>
      </aside>
    </div>
  );
}

