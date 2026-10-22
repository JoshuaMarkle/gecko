// Types mirror the Rust serde shapes (gecko-core report/run/store). Keep them identical.

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
}

/** A key in the board's 2D drawing (gecko-core `Geometry::flat2d`): top-left corner, rotation in degrees about its center. */
export interface Flat { pos: [number, number]; rot: number }

export interface Geometry {
  id: string; name: string; description: string; key: Key[];
  /** 2D drawing, one per key (served by `/api/geometries`; curved boards projected, overlaps removed). */
  flat?: Flat[];
  /** Comfort metric id → table name, and the loaded N×N tables (passed through to WASM). */
  comfort?: Record<string, string>; comfort_tables?: Record<string, number[]>;
}

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
  /** Metric id → reference spread; the scorer uses weight / scale (weights in spread units). */
  scale?: Record<string, number>;
  scale_basis?: string;
  row_offset?: [number, number][];
  familiar_to?: string;
  sfb?: { distance: "linear" | "squared" | "fitts"; lateral: number; finger_factor: number[]; skip_decay: number };
  /** Hand model for G01 / G05 and B14's spread (`hand.ts` `withHand`). */
  hand?: import("./hand").HandSpec;
}

export type Dir = "min" | "max" | "info";
export interface MetricDef { id: string; name: string; unit: string; order: string; dir: Dir; desc: string }

/** `chars` is slot-ordered; for a layered layout it is the flat string over its layer slots
 *  (see `layerSlots`), and `layers` names layers 1, 2, …. */
export interface LayoutInfo { id: string; name: string; geometry: string; source: string; chars: string; layers?: string[] }
export interface CorpusInfo { id: string; name: string; chars: number; charset: string }

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
export interface Progress { elapsed_ms: number; evals: number; best: number; best_chars: string; explore?: ExploreProgress }

/** `atlas.json` of an explore run (gecko-core `explore::atlas::Summary`). */
export interface AtlasBest { score: number; chars: string; d_best: number }
export interface AtlasSummary {
  schema: number; id: string; done: boolean; elapsed_ms: number; evals: number;
  /** Symbol id → char for every `sym` array. */
  chars: string; slots: number; metrics: string[]; symmetric: boolean; table_hash: string;
  counts: { optima: number; constrained: number; hits: number; edges: number; pruned: number };
  best: AtlasBest | null;
  probe: { samples: number; random_mean: number; random_sd: number; random_min: number; walks: number; walk_len: number; rho: number[]; corr_length: number };
  fdc: number; funnels: number;
  coverage: { within: number; observed: number; estimate: number };
  distinct_dist: number; distinct: AtlasBest[];
}
export interface Ranked { rank: number; score: number; chars: string; rows: string[]; hash: string; breakdown: Breakdown; holdout?: number }
/** L06: the start layout and best result on the search corpus and on a held-out corpus (gecko-core `run::HoldoutReport`). */
export interface HoldoutReport { corpus: string; start: number; best: number; start_search: number; best_search: number; retained: number; rank_corr?: number }
/** A lower bound and the optimality gap of a run's best layout (gecko-core `bound::Gap`). */
export interface BoundGap { lower: number; gilmore_lawler: number; value: number; gap_pct: number }
export interface BoundReport { method: string; symbols: number; iters: number; total: BoundGap; pair: BoundGap; elapsed_ms: number }
export interface RunResult { schema: number; id: string; geometry: string; evals: number; elapsed_ms: number; evals_per_sec: number; best: Ranked[]; bound?: BoundReport; holdout?: HoldoutReport }
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
  holdout?: { name: string; hash: string };
}
export interface RunSummary {
  id: string; created: string; algo: string; profile: string; corpus: string; geometry: string;
  best_score: number | null; best: string | null; elapsed_ms: number | null; status: string;
}
export interface RunDetail { config: RunConfig; result: RunResult | null; progress: Progress[]; running: boolean; paused?: boolean }

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
  meta: () => req<Meta>("/api/meta"),
  layouts: () => req<LayoutInfo[]>("/api/layouts"),
  geometries: () => req<Geometry[]>("/api/geometries"),
  profiles: () => req<Profile[]>("/api/profiles"),
  corpora: () => req<CorpusInfo[]>("/api/corpora"),
  corpusBytes: async (id: string) => new Uint8Array(await (await fetch(`/api/corpora/${encodeURIComponent(id)}`)).arrayBuffer()),
  runs: () => req<RunSummary[]>("/api/runs"),
  run: (id: string) => req<RunDetail>(`/api/runs/${encodeURIComponent(id)}`),
  cancel: (id: string) => post<unknown>(`/api/runs/${encodeURIComponent(id)}/cancel`, {}),
  pause: (id: string, on: boolean) => post<unknown>(`/api/runs/${encodeURIComponent(id)}/${on ? "pause" : "resume"}`, {}),
  score: (body: ScoreRequest) => post<ScoreResponse>("/api/score", body),
  startRun: (body: RunRequest) => post<{ id: string; config: RunConfig }>("/api/runs", body),
  swapMatrix: (body: ScoreRequest) => post<{ n: number; score: number; matrix: number[] }>("/api/swap-matrix", body),
  atlas: (id: string) => req<AtlasSummary>(`/api/runs/${encodeURIComponent(id)}/atlas`),
  atlasFile: async (id: string, file: "optima" | "edges" | "samples") => {
    const r = await fetch(`/api/runs/${encodeURIComponent(id)}/atlas/${file}`);
    return r.ok ? new Uint8Array(await r.arrayBuffer()) : new Uint8Array();
  },
};

export interface ScoreRequest { geometry: string; corpus: string; profile_override: Profile; chars: string; top?: number }
export interface RunRequest extends ScoreRequest {
  algo: string; time_ms?: number; evals?: number; threads: number; seed: number; pins: string; sa_steps?: number; ga_pop?: number;
  /** Corpus id to score results on without searching on it (L06). */
  holdout?: string;
  memetic?: { ga: { pop_size: number }; local_search: "hc" | "tabu"; local_steps: number; islands: number; migration_interval: number; migrants: number };
}
