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
  effort: number;
}

export interface Geometry {
  id: string; name: string; description: string; key: Key[];
  /** Comfort metric id → table name, and the loaded N×N tables (passed through to WASM). */
  comfort?: Record<string, string>; comfort_tables?: Record<string, number[]>;
}

export interface Profile {
  id?: string;
  name: string;
  description: string;
  weights: Record<string, number>;
  finger_target: number[];
  trigram_coverage: number;
  /** Metric id → reference spread; the scorer uses weight / scale (weights in spread units). */
  scale?: Record<string, number>;
  scale_basis?: string;
  sfb?: { distance: "linear" | "squared" | "fitts"; lateral: number; finger_factor: number[]; skip_decay: number };
}

export type Dir = "min" | "max" | "info";
export interface MetricDef { id: string; name: string; unit: string; order: string; dir: Dir; desc: string }

export interface LayoutInfo { id: string; name: string; geometry: string; source: string; chars: string }
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
}
export interface ScoreResponse { schema: number; chars: string; rows: string[]; analysis: Analysis }

export interface Progress { elapsed_ms: number; evals: number; best: number; best_chars: string }
export interface Ranked { rank: number; score: number; chars: string; rows: string[]; hash: string; breakdown: Breakdown }
export interface RunResult { schema: number; id: string; geometry: string; evals: number; elapsed_ms: number; evals_per_sec: number; best: Ranked[] }
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
};

export interface ScoreRequest { geometry: string; corpus: string; profile_override: Profile; chars: string; top?: number }
export interface RunRequest extends ScoreRequest {
  algo: string; time_ms: number; threads: number; seed: number; pins: string; sa_steps?: number; ga_pop?: number;
}
