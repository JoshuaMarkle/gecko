// Types mirror the Rust serde shapes (gecko-core report/run/store); they may leave fields out.
// `npm run test:contract` typechecks live server responses against `api`'s return types.

export type Finger = "LP" | "LR" | "LM" | "LI" | "LT" | "RT" | "RI" | "RM" | "RR" | "RP";
export const FINGERS: Finger[] = ["LP", "LR", "LM", "LI", "LT", "RT", "RI", "RM", "RR", "RP"];

export interface Key {
  pos: [number, number, number];
  rot: [number, number, number];
  size: [number, number];
  finger: Finger;
  row: number;
  col: number;
  home: boolean;
  stretch: boolean;
  hrm?: boolean;
  effort: number;
  /** Symbol (token name or one char) this key gets when a layout lists only the other keys. */
  fill?: string;
  /** Other fingers that may press this key, for the fingering search (gecko-core `AltFinger`). */
  alt?: { finger: Finger; effort: number }[];
}

/** A key in the board's 2D drawing (gecko-core `Geometry::flat2d`): top-left corner, rotation in degrees about its center. */
export interface Flat { pos: [number, number]; rot: number }

export interface Geometry {
  id: string; name: string; description: string; key: Key[];
  /** 2D drawing, one per key (served by `/api/geometries`; curved boards projected, overlaps removed). */
  flat?: Flat[];
  /** Comfort metric id → table name, and the loaded N×N tables (passed through to WASM). */
  comfort?: Record<string, string>; comfort_tables?: Record<string, number[]>;
  /** Parameter file of a parametric board: it can be geometry-searched (`geom.ts`). */
  param?: ParamGeometry;
  /** Set on the best board of a geometry run (id `run:<run>`). */
  run?: string;
  /** `Geometry::hash` (served boards; a run records the same in `geometry_hash`). */
  hash?: string;
}

/** A parametric board (gecko-core `ParametricGeometry`); opaque here, passed to WASM `param_board`. */
export type ParamGeometry = { name: string; zone: unknown[] } & Record<string, unknown>;

export interface Profile {
  id?: string;
  name: string;
  description: string;
  weights: Record<string, number>;
  finger_target: number[];
  finger_effort?: import("./fingerEffort").FingerEffort;
  /** Target thumb share % for M09 (default 18 when absent). */
  thumb_target?: number;
  trigram_coverage: number;
  /** Vim motion policy for L03: "row" (h j k l in order), "pairs" (h l and j k), "fingers" (four fingers). */
  vim?: string;
  /** Adaptive fidelity: search on the `top_k` most frequent trigrams, confirm on the full tables. */
  fidelity?: { top_k: number; grow: number; misses: number };
  /** Metric id → reference spread; the scorer uses weight / scale (weights in spread units). */
  scale?: Record<string, number>;
  scale_basis?: string;
  row_offset?: [number, number][];
  familiar_to?: string;
  /** `distance`, `lateral`, `finger_factor` shape only the retired B03/S03; edited here: `skip_decay` (S02). */
  sfb?: { distance: "linear" | "squared" | "fitts"; lateral: number; finger_factor: number[]; skip_decay: number };
  /** Hand model for G01 / G05 and B14's spread (`hand.ts` `withHand`). */
  hand?: import("./hand").HandSpec;
}

export type Dir = "min" | "max" | "info";
/** `retired`: still scored for old runs and profiles, hidden in the UI (filtered by `api.meta`). */
export interface MetricDef { id: string; name: string; unit: string; order: string; dir: Dir; desc: string; retired: boolean }

/** `chars` is slot-ordered; for a layered layout it is the flat string over its layer slots
 *  (see `layerSlots`), and `layers` names layers 1, 2, …. */
export interface LayoutInfo { id: string; name: string; geometry: string; source: string; chars: string; layers?: string[] }
/** A language recipe's part (`data/corpora/<name>.mix.toml`); `share` = weight / Σ weights. */
export interface MixPart { corpus: string; weight: number; share: number }
export interface Recipe { parts: MixPart[]; edits: number; nav: number }
/** `language` (optimize on these), `component` (parts of languages), `reference` (older corpora). */
export type CorpusKind = "language" | "component" | "reference";
export interface CorpusInfo {
  id: string; name: string; chars: number; charset: string; hash: string;
  kind: CorpusKind; description: string; source: string;
  /** Has case counts (Shift presses). */
  shift: boolean;
  recipe: Recipe | null;
}
/** Per-symbol keystroke shares of a language's parts and of the built language (`/api/corpora/{name}/composition`). */
export interface Composition {
  corpus: string; edits: number; nav: number; mono: Record<string, number>;
  parts: (MixPart & { mono: Record<string, number>; recipe: boolean })[];
}
export interface MixRequest { name: string; parts: { corpus: string; weight: number }[]; edits: number; nav?: number; description?: string }

export interface Meta {
  schema: number;
  version: string;
  defaults: { geometry: string; corpus: string; profile: string; reference: string };
  algos: string[];
  metrics: MetricDef[];
  threads: number;
}

export interface MetricValue { id: string; name: string; unit: string; dir: Dir; value: number; weight: number; weighted: number }
export interface Breakdown { score: number; metrics: MetricValue[] }
export interface Ngram { ngram: string; freq: number }
/** One directional pair and its class (gecko-core `report::Pair`). */
export type PairKind = "sfb" | "sfs" | "lsb" | "scissor" | "same_hand" | "alternate";
export interface Pair { ngram: string; freq: number; kind: PairKind }
export interface Analysis {
  breakdown: Breakdown;
  finger_load: number[];
  finger_target: number[];
  fingers: Finger[];
  heat: number[];
  worst_sfb: Ngram[];
  worst_sfs: Ngram[];
  worst_lsb: Ngram[];
  worst_scissors: Ngram[];
  bigrams: Pair[];
  skipgrams: Pair[];
}
export interface ScoreResponse { schema: number; chars: string; rows: string[]; analysis: Analysis }

export interface ExploreProgress { phase: "probe" | "search"; optima: number; hits: number; edges: number; version: number }
/** One worker at one progress record (gecko-core `optimize::WorkerTrace`); move counts are since the previous record. */
export interface WorkerTrace {
  evals: number; current: number; best: number; temp?: number;
  proposed: number; improving: number; worse: number; worse_accepted: number;
  /** The worker's current layout, slot-ordered. */
  chars: string;
}
export interface Progress { elapsed_ms: number; evals: number; best: number; best_chars: string; explore?: ExploreProgress; workers?: WorkerTrace[] }

/** `atlas.json` of an explore run (gecko-core `explore::atlas::Summary`). */
export interface AtlasBest { score: number; chars: string; d_best: number }
export interface AtlasSummary {
  schema: number; id: string; done: boolean; elapsed_ms: number; evals: number;
  /** Symbol id → char for every `sym` array. */
  chars: string; slots: number; metrics: string[]; symmetric: boolean; table_hash: string;
  counts: { optima: number; constrained: number; hits: number; edges: number; pruned: number };
  best: AtlasBest | null;
  probe: { samples: number; random_mean: number; random_sd: number; random_min: number; walks: number; walk_len: number; rho: number[]; corr_length: number };
  fdc: number | null; funnels: number;
  coverage: { within: number; observed: number; estimate: number };
  distinct_dist: number; distinct: AtlasBest[];
}
export interface Ranked { rank: number; score: number; chars: string; rows: string[]; hash: string; breakdown: Breakdown; holdout?: number }
/** L06: the start layout and best result on the search corpus and on a held-out corpus (gecko-core `run::HoldoutReport`). */
export interface HoldoutReport { corpus: string; start: number; best: number; start_search: number; best_search: number; retained: number; rank_corr?: number }
/** A lower bound and the optimality gap of a run's best layout (gecko-core `bound::Gap`). */
export interface BoundGap { lower: number; gilmore_lawler: number; value: number; gap_pct: number }
export interface BoundReport { method: string; symbols: number; iters: number; total: BoundGap; pair: BoundGap; elapsed_ms: number }
/** An NSGA-II / RWSA objective (gecko-core `optimize::nsga2::Objective`). */
export interface Objective { id: string; unit: string; dir: string; cost: string }
/** A Pareto-front layout: `raw` values and minimized `costs` align with the run's `objectives`. */
export interface ParetoRanked extends Ranked { raw: number[]; costs: number[] }
export interface RunResult {
  schema: number; id: string; geometry: string; evals: number; elapsed_ms: number; evals_per_sec: number; best: Ranked[]; bound?: BoundReport; holdout?: HoldoutReport;
  /** Multi-objective runs (nsga2, rwsa) only. */
  objectives?: Objective[]; front?: ParetoRanked[];
  /** Adaptive-fidelity search (gecko-core `optimize::FidelityReport`). */
  fidelity?: { levels: number[]; confirmed: number; rejected: number; final_level: number[] };
  /** CLI `optimize --fingering` (gecko-core `fingering::FingeringReport`); a change makes `geometry` `run:<id>`. */
  fingering?: {
    choice: number[]; base_score: number; score: number; candidates: number; steps: number;
    changes: { key: number; row: number; col: number; from: Finger; to: Finger; effort: number }[];
  };
}
/** `qd.json` of a MAP-Elites run (gecko-core `explore::qd::QdSummary`). */
export interface QdAxis { name: string; lo: number; hi: number; bins: number; auto: boolean }
export interface QdElite { row: number; cell: number; bins: number[]; desc: number[]; score: number; chars: string; updates: number }
export interface QdSummary {
  schema: number; id: string; done: boolean; elapsed_ms: number; evals: number; chars: string; slots: number; metrics: string[]; table_hash: string;
  axes: QdAxis[]; cells: number; filled: number; coverage: number; offspring: number; added: number; improved: number;
  random_mean: number; qd_score: number; best: QdElite | null; elites: QdElite[];
}
export interface RunParams { algo: string; seed: number; threads: number; time_ms: number | null; evals: number | null; top: number }
export interface RunConfig {
  id: string;
  created: string;
  params: RunParams;
  geometry: { name: string; hash: string };
  corpus: { name: string; hash: string };
  profile: Profile;
  start: string;
  pins: string;
  /** Shift pairs of the start layout (keyset `shift` runs), base then shifted symbol. */
  shifted?: string[];
  holdout?: { name: string; hash: string };
}
export interface RunSummary {
  id: string; created: string; algo: string; profile: string; corpus: string;
  /** Board name; for a geometry run, its best board's (the `run:<id>` board) once done. */
  geometry: string;
  /** Input hashes (a geometry run: its start board's parameter file). */
  geometry_hash: string; corpus_hash: string; profile_hash: string;
  best_score: number | null; best: string | null; elapsed_ms: number | null; status: string;
  kind: "layout" | "geometry";
}
export interface RunDetail { kind?: undefined; config: RunConfig; result: RunResult | null; progress: Progress[]; running: boolean; paused?: boolean }

// ---- bench (`gecko bench`, saved in runs/bench/) ----

/** One seed of one solver: final score and every new best with its time (ms). */
export interface BenchRun { seed: number; score: number; evals: number; elapsed_ms: number; trace: [number, number][] }
export interface BenchResult {
  algo: string; moves: string; mean: number; best: number; stdev: number; evals_per_sec: number; scores: number[];
  /** Left out by the list route. */
  runs?: BenchRun[];
}
export interface Bench {
  schema: number; id: string; created: string; commit: string; time_ms: number; threads: number; seeds: number;
  geometry: string; corpus: string; profile: string; results: BenchResult[];
}

/** Path relinking between two layouts (gecko-core `explore::path`): slots swapped and the score after each step. */
export interface PathResult { start: number; steps: { a: number; b: number; score: number }[]; barrier: number }

// ---- geometry runs (board + layout search, gecko-core `geomsearch`) ----

/** A free parameter (`split.<f>`, `<zone>.<f>`, `<zone>.<col>.<f>`), its bounds and file value. */
export interface GeomVar { name: string; lo: number; hi: number; start: number }
export interface GeomProgress {
  elapsed_ms: number; phase: "search" | "layout" | "polish"; generation: number; candidates: number;
  /** Best objective (score + collision penalty) so far, and the best of collision-free boards. */
  best: number; best_feasible: number | null; generation_best: number;
  /** The board reported so far: free-parameter values (aligned with the run's vars), its layout, collisions. */
  values: number[]; chars: string; collisions: number;
}
export interface GeomPoint { values: number[]; score: number; f: number; collisions: number; depth: number; chars: string; rows: string[]; breakdown: Breakdown }
export interface GeomSearchParams {
  mode: "bilevel" | "alternate" | "board"; seed: number; threads: number; time_ms: number | null; evals: number | null;
  inner_evals: number; inner_accept0: number; polish_frac: number; polish_evals: number; sigma0: number; popsize: number | null;
  margin: number; penalty: number; hard_fail: number; rounds: number; keep_effort: boolean;
}
export interface GeomRunConfig {
  kind: "geometry"; id: string; created: string; params: GeomSearchParams; free: string; vars: GeomVar[];
  geometry: { name: string; hash: string }; corpus: { name: string; hash: string }; profile: Profile; start: string; pins: string;
}
export interface GeomResult {
  schema: number; id: string; mode: string; vars: GeomVar[];
  /** The start and the best board, each after an equal final layout polish. */
  start: GeomPoint; best: GeomPoint;
  search_start: number; search_best: number; candidates: number; generations: number; layout_evals: number; elapsed_ms: number;
  cma_runs: { popsize: number; generations: number; evals: number; best: number; stop: string }[];
  feasible: boolean;
}
export interface GeomRunDetail {
  kind: "geometry"; config: GeomRunConfig; param: ParamGeometry; result: GeomResult | null; progress: GeomProgress[]; running: boolean; paused?: boolean;
}

async function req<T>(path: string, init?: RequestInit): Promise<T> {
  const r = await fetch(path, init);
  const body = await r.json().catch(() => ({ error: `${r.status} ${r.statusText}` }));
  if (!r.ok) throw new Error(body.error ?? `${r.status}`);
  return body as T;
}

const post = <T>(path: string, body: unknown) =>
  req<T>(path, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(body) });

/** A source text of a corpus (`GET /api/corpora/{name}/sources`). */
export interface SourceInfo { name: string; bytes: number }

export const api = {
  meta: async () => {
    const m = await req<Meta>("/api/meta");
    return { ...m, metrics: m.metrics.filter((d) => !d.retired) };
  },
  layouts: () => req<LayoutInfo[]>("/api/layouts"),
  geometries: () => req<Geometry[]>("/api/geometries"),
  profiles: () => req<Profile[]>("/api/profiles"),
  corpora: () => req<CorpusInfo[]>("/api/corpora"),
  composition: (id: string) => req<Composition>(`/api/corpora/${encodeURIComponent(id)}/composition`),
  /** Save (and build) a language recipe on the server. */
  saveMix: (body: MixRequest) => post<{ id: string; hash: string; chars: number }>("/api/corpora/mix", body),
  corpusBytes: async (id: string) => new Uint8Array(await (await fetch(`/api/corpora/${encodeURIComponent(id)}`)).arrayBuffer()),
  runs: () => req<RunSummary[]>("/api/runs"),
  run: (id: string) => req<RunDetail>(`/api/runs/${encodeURIComponent(id)}`),
  /** A layout or a geometry run (same route as `run`; `kind` tells them apart). */
  anyRun: (id: string) => req<RunDetail | GeomRunDetail>(`/api/runs/${encodeURIComponent(id)}`),
  /** A geometry run's detail (same route as `run`). */
  geomRun: (id: string) => req<GeomRunDetail>(`/api/runs/${encodeURIComponent(id)}`),
  geometryFree: (id: string, spec: string) =>
    req<{ vars: GeomVar[] }>(`/api/geometries/${encodeURIComponent(id)}/free?spec=${encodeURIComponent(spec)}`),
  cancel: (id: string) => post<unknown>(`/api/runs/${encodeURIComponent(id)}/cancel`, {}),
  pause: (id: string, on: boolean) => post<unknown>(`/api/runs/${encodeURIComponent(id)}/${on ? "pause" : "resume"}`, {}),
  score: (body: ScoreRequest) => post<ScoreResponse>("/api/score", body),
  startRun: (body: RunRequest) => post<{ id: string; config: RunConfig | GeomRunConfig }>("/api/runs", body),
  startGeomRun: (body: RunRequest & { geom: GeomRequest }) => post<{ id: string; config: GeomRunConfig }>("/api/runs", body),
  path: (body: ScoreRequest & { to: string }) => post<PathResult>("/api/path", body),
  benches: () => req<Bench[]>("/api/bench"),
  bench: (id: string) => req<Bench>(`/api/bench/${encodeURIComponent(id)}`),
  swapMatrix: (body: ScoreRequest) => post<{ n: number; score: number; matrix: number[] }>("/api/swap-matrix", body),
  qd: (id: string) => req<QdSummary>(`/api/runs/${encodeURIComponent(id)}/qd`),
  atlas: (id: string) => req<AtlasSummary>(`/api/runs/${encodeURIComponent(id)}/atlas`),
  atlasFile: async (id: string, file: "optima" | "edges" | "samples") => {
    const r = await fetch(`/api/runs/${encodeURIComponent(id)}/atlas/${file}`);
    return r.ok ? new Uint8Array(await r.arrayBuffer()) : new Uint8Array();
  },
};

export interface ScoreRequest { geometry: string; corpus: string; profile_override: Profile; chars: string; top?: number; shifted?: string[] }
/** Build the symbol set and start layout from board and corpus (`gecko keyset`). */
export interface KeysetRequest { mode: "alpha" | "shift" | "layer"; pairs?: string; symbols?: number; keys?: string; layer_mode?: string; thumb_letters?: boolean; place?: boolean; place_evals?: number }
export interface RunRequest extends ScoreRequest {
  algo: string; time_ms?: number; evals?: number; threads: number; seed: number; pins: string; sa_steps?: number; ga_pop?: number;
  /** Corpus id to score results on without searching on it (L06). */
  holdout?: string;
  memetic?: { ga: { pop_size: number }; local_search: "hc" | "tabu"; local_steps: number; islands: number; migration_interval: number; migrants: number };
  /** A board + layout search on the (parametric) geometry; `evals` then counts boards. */
  geom?: GeomRequest;
  /** Start from a keyset instead of `chars`. */
  keyset?: KeysetRequest;
}
/** `free` as `gecko geom optimize --free`; the rest override `GeomSearchParams`. */
export interface GeomRequest { free: string; mode?: "bilevel" | "alternate" | "board"; inner_evals?: number; margin?: number; penalty?: number; keep_effort?: boolean }
