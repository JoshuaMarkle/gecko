# Web UI

For the human. Local only, served by `gecko serve`. Visual design: [design.md](design.md).

## Stack

TypeScript (strict), Vite, React 19, Three.js via `@react-three/fiber` + `@react-three/drei`.
Charts are hand-rolled SVG. Scoring in the browser through `gecko-wasm` (same code as the CLI).

```
web/src/
  App.tsx              shell, tabs, run lifecycle (SSE)
  inputs.ts            the one input store (board, corpus, profile, layout, pins, search)
  api.ts               API client + types mirroring the Rust serde shapes
  wasm.ts              gecko-wasm loader (Scorer.create / analyze)
  format.ts(.test.ts)  pure helpers (rows, swaps, verdicts, validation)
  colors.ts            data colors (finger palette, heat ramp)
  components/          Inputs (tab pages + Layout panel), ResultPanel, Charts, ResultsView, AnalysisView, Icons
  three/Keyboard3D.tsx 3D keyboard
```

## Screen

Toolbar (see design.md "App shell"): tabs | Start optimization · Pause · End | run progress.
Tabs follow the design process: **Board**, **Hand**, **Corpus**, **Profile**, **Search** (the
inputs, one form page each), then **Optimize** (Layout | 3D keyboard over chart tabs | Result),
**Explore** (atlas of an explore run, below), **Stats** (all charts for the current layout),
**Runs** (all runs, the selected run's convergence and best layouts). The tab is remembered in
`localStorage` (old `home`/`results`/`analysis` values map to Optimize/Runs/Stats).

- Inputs live in one store (`inputs.ts`, `useInputs`); every tab edits and reads the same
  state, so Play uses the same inputs whichever tab it is pressed on. Hand has no inputs yet
  (#92). Optimize's left panel keeps the start layout, editor, pins and reference, plus a
  summary of the other inputs that links to their tabs.
- Play starts a run with the Search tab's settings (algorithm, budget) — an exploration on the
  Explore tab — or resumes a paused one. The transport and progress show on every tab. Pause
  freezes the workers and the budget clock. End cancels; results so far are saved.
- Click two keys (3D or heatmap) to swap them; the score updates live via WASM.
- Layered layouts (flat string, see `representation.md`): a Base / ① Layer 1 / … toggle in the
  view controls picks the layer the 3D view shows (symbols, heat); keys on the holding finger
  are dimmed and ignore clicks. A selection survives switching layers, so symbols swap across
  layers. The Edit box shows the base and each layer as blocks separated by a blank line.
  `format.ts` `layerSlots` / `fitChars` mirror `Geometry::layered` / `Layout::fit`; 30-key
  layouts get the board's thumb fills client-side too. The 2D heatmap shows the base layer.
- Profile weights and targets are editable; edits re-score live and are sent with each run
  (`profile_override`), so the run's `config.json` records exactly what was optimized.
- While a run is going, the 3D view shows the live best layout.
- UI preferences (tab, chart tab, overlay, sizes, current layout) persist in `localStorage`.

## Explore view (`components/explore/`)

Left: atlas picker, new exploration (budget, threads, seed; other inputs from the input tabs), ranking
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

## 2D boards

The heatmap and the Explore mini keyboards (`KeyGrid`, consensus, thumbnails) draw
`geometry.flat` (gecko-core `Geometry::flat2d`, so curved boards get a clean projection) via
`format.ts` `flatKeys` / `flatBounds`, with each key's in-plane rotation.

## 3D rendering

Each key is a keycap (sharp base, tapered walls with smooth rounded corners, flat top; `capGeometry`, cached per size) placed from the geometry's `pos`, `rot`, `size` (1 unit = 1u =
19.05 mm; board y → three z). Legends are cached canvas textures. Overlays: none (default,
plain keys), frequency heat, finger colors, n-grams (finger-colored keys plus one arc per undirected bigram or skipgram, all
of them; width and opacity ∝ √(frequency / most frequent visible pair); colored by class: SFB/SFS red, LSB orange, scissors
violet, other same-hand ink, alternation faint; same-hand and alternate start hidden; legend chips toggle each class and show its %).
A 3D toggle (default on) switches between the orbit camera and a flat top-down view with rotation
locked; both presets fit the board to the viewport. A floor grid (1u cells, 5u sections) sits under the case. Flat and curved boards use the same code path.

## API (gecko serve)

| Route | |
|---|---|
| `GET /api/meta` | defaults, algos, metric definitions, core count |
| `GET /api/layouts` · `/api/geometries` · `/api/profiles` · `/api/corpora` | data dir contents |
| `GET /api/corpora/{name}` | `.gkc` bytes (for WASM) |
| `POST /api/score` | `{geometry, corpus, profile \| profile_override, chars \| layout, top}` → analysis |
| `GET /api/runs` · `POST /api/runs` | list / start (`algo, time_ms \| evals, threads, seed, pins, sa_steps, ga_pop, memetic, moves` + score inputs) |
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


Search includes memetic islands: population per island, HC/tabu refinement and its step
cap, island count (0 = threads), migration interval (0 disables), and migrant count.
`POST /api/runs` accepts optional `memetic` using the core `MemeticParams` shape; omitted
fields use defaults. The server limits islands to core count and caps population/refinement
work. Older saved Search preferences acquire defaults for the new fields.
