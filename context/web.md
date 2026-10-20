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
  components/          InputsPanel, ResultPanel, Charts
  three/Keyboard3D.tsx 3D keyboard
```

## Screen

One screen (see design.md "App shell"): Inputs (Layout, Board, Corpus, Profile, Search) |
3D keyboard over chart tabs (Convergence, Metrics, Finger load, Heatmap, Worst n-grams) |
Result (score vs reference, metric table, run results, runs list, exports).

- Click two keys (3D or heatmap) to swap them; the score updates live via WASM.
- Profile weights and targets are editable; edits re-score live and are sent with each run
  (`profile_override`), so the run's `config.json` records exactly what was optimized.
- While a run is going, the 3D view shows the live best layout.
- UI preferences (group, tab, overlay, sizes, current layout) persist in `localStorage`.

## 3D rendering

Each key is a rounded keycap placed from the geometry's `pos`, `rot`, `size` (1 unit = 1u =
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
| `GET /api/runs/{id}` | config, result, progress, running |
| `GET /api/runs/{id}/events` | SSE: `progress`, then `done` (RunResult) or `failed` |
| `POST /api/runs/{id}/cancel` | stop early (results are still saved) |

## Dev

`npm run wasm` (after Rust changes to the scorer), `npm run dev` (proxies `/api` to :7777),
`npm run build` → `web/dist`, `npm run typecheck`, `npm test`.
