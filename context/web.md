# Web UI

For the human. Local only, served by `gecko serve`. Visual design: [design.md](design.md).

## Stack

TypeScript (strict), Vite, React 19, Three.js via `@react-three/fiber` + `@react-three/drei`.
Charts are hand-rolled SVG. Scoring in the browser through `gecko-wasm` (same code as the CLI).

```
web/src/
  App.tsx              shell, state, run lifecycle (SSE)
  api.ts               API client + types mirroring the Rust serde shapes
  wasm.ts              gecko-wasm loader (Scorer.create / analyze)
  format.ts(.test.ts)  pure helpers (rows, swaps, verdicts, validation)
  colors.ts            data colors (finger palette, heat ramp)
  components/          InputsPanel, ResultPanel, Charts, ResultsView, AnalysisView, Icons
  three/Keyboard3D.tsx 3D keyboard
```

## Screen

Toolbar (see design.md "App shell"): view tabs | Start optimization · Pause · End | run progress.
Views: **Home** (Inputs | 3D keyboard over chart tabs | Result), **Results** (all runs, the
selected run's convergence and best layouts), **Layout analysis** (all charts for the current
layout), **Explore** (atlas of an explore run, below). The view is remembered in `localStorage`.

- Play starts a run with Inputs › Search (algorithm, budget) or resumes a paused one. Pause
  freezes the workers and the budget clock. End cancels; results so far are saved.
- Click two keys (3D or heatmap) to swap them; the score updates live via WASM.
- Profile weights and targets are editable; edits re-score live and are sent with each run
  (`profile_override`), so the run's `config.json` records exactly what was optimized.
- While a run is going, the 3D view shows the live best layout.
- UI preferences (view, group, tab, overlay, sizes, current layout) persist in `localStorage`.

## Explore view (`components/explore/`)

Left: atlas picker, new exploration (budget, threads, seed; other inputs from Home), ranking
(atlas profile, or the current profile with editable weights; every panel re-ranks), focus
(best N, family radius, brush) and landscape stats. Centre, linked cards: Map (PCA of slot,
finger or metric encodings), Fitness vs distance (+ reference layouts), Consensus keyboard,
Families, Trade-off (any two metrics, Pareto front), Funnels, Parallel coordinates, Weight
stability, Discovery curve, How good is good (random vs optima), Ruggedness. Right: the
picked layout — metrics vs best, provenance, its whole swap neighbourhood; Open in editor.

All atlas statistics run in WASM (`AtlasView`) on the binary files; the live atlas reloads
on each snapshot (throttled 2.5 s). Reference layouts are scored via `/api/score`. Swap
neighbourhoods use WASM when the geometry and corpus match the loaded scorer, else
`/api/swap-matrix`.

## 3D rendering

Each key is a keycap (sharp base, tapered walls with smooth rounded corners, flat top; `capGeometry`, cached per size) placed from the geometry's `pos`, `rot`, `size` (1 unit = 1u =
19.05 mm; board y → three z). Legends are cached canvas textures. Overlays: finger colors,
frequency heat, SFB arcs (top same-finger bigrams, width ∝ frequency). 3D/top camera presets
fit the board to the viewport. Flat and curved boards use the same code path.

## API (gecko serve)

| Route | |
|---|---|
| `GET /api/meta` | defaults, algos, metric definitions, core count |
| `GET /api/layouts` · `/api/geometries` · `/api/profiles` · `/api/corpora` | data dir contents |
| `GET /api/corpora/{name}` | `.gkc` bytes (for WASM) |
| `POST /api/score` | `{geometry, corpus, profile \| profile_override, chars \| layout, top}` → analysis |
| `GET /api/runs` · `POST /api/runs` | list / start (`algo, time_ms \| evals, threads, seed, pins, sa_steps, ga_pop` + score inputs) |
| `GET /api/runs/{id}` | config, result, progress, running, paused |
| `GET /api/runs/{id}/events` | SSE: `progress`, then `done` (RunResult) or `failed` |
| `POST /api/runs/{id}/cancel` | stop early (results are still saved) |
| `GET /api/runs/{id}/atlas` | explore run summary (`atlas.json`) |
| `GET /api/runs/{id}/atlas/{optima\|edges\|samples}` | atlas binary files (for WASM `AtlasView`) |
| `POST /api/swap-matrix` | score inputs + `chars` → Δ score of every slot-pair swap |
| `POST /api/runs/{id}/pause` · `/resume` | pause / resume a running run; paused time does not count against `time_ms` or toward `elapsed_ms` |

## Dev

`npm run wasm` (after Rust changes to the scorer), `npm run dev` (proxies `/api` to :7777),
`npm run build` → `web/dist`, `npm run typecheck`, `npm test`.
