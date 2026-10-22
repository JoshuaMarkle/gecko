# Web UI

For the human. Local only, served by `gecko serve`. Visual design: [design.md](design.md).
What each tab shows: [web-screens.md](web-screens.md).

## Stack

TypeScript (strict), Vite, React 19, Three.js via `@react-three/fiber` + `@react-three/drei`.
Charts are hand-rolled SVG. Scoring in the browser through `gecko-wasm` (same code as the CLI).

```
web/src/
  main.tsx · App.tsx   shell: navbar, view switch, working model
  liveRun.ts           the navbar run: start, SSE progress, pause / resume / stop, budget %
  tabs.ts              tab lists, old-name mapping
  inputs.ts            the one input store (board, custom boards, constraints, hand, language,
                       profile, layout, pins, search, working model) + usePref
  api.ts               API client + types mirroring the Rust serde shapes (checked by test:contract)
  wasm.ts              gecko-wasm loader (Scorer, flat2d, export_layout, corpus stats / build)
  format.ts            pure helpers (rows, swaps, layers, verdicts, validation, cap)
  export.ts            Export tab file builders (image SVG, layout TOML/text/JSON, KLE, stats)
  board.ts hand.ts handCosts.ts estimate.ts explore.ts fingerEffort.ts languages.ts
                       per-view logic; pure parts unit-tested (*.test.ts); hand.ts is checked
                       against Rust by hand.parity.test.ts
  colors.ts            data colors (finger palette, heat ramp)
  components/          Form (Frame, Sec, Num, …), Board2D, LayoutEditor, Charts, Icons
    views/             one file per tab (Board, Hand, Language, Profile, Search, Summary,
                       Home, Analysis, Saved, Export); OptimizationFlow is Summary's
                       inputs → scorer → search diagram
    hand/              Hand tab parts: rigs (hand placement), Hands3D, Panels (bottom tabs)
    explore/           Explore tab: ExploreView, Panels, Board, Scatter, Picked
  three/Keyboard3D.tsx 3D keyboard (auto-rotate, click position, scene children)
web/contract/          API contract check (loose.ts; generated samples are gitignored)
```

## 2D boards

`Board2D` draws `geometry.flat` (gecko-core `Geometry::flat2d`, so curved boards get a clean
projection) via `format.ts` `flatKeys` / `flatBounds`, with each key's in-plane rotation and
optional arcs between keys. Custom boards get `flat` from WASM `flat2d`.

## 3D rendering

Each key is a keycap (sharp base, tapered walls with smooth rounded corners, flat top; `capGeometry`, cached per size) placed from the geometry's `pos`, `rot`, `size` (1 unit = 1u =
19.05 mm; board y → three z). Legends are cached canvas textures. Overlays: none (default,
plain keys), frequency heat, finger colors, n-grams (finger-colored keys plus one arc per undirected bigram or skipgram, all
of them; width and opacity ∝ √(frequency / most frequent visible pair); colored by class: SFB/SFS red, LSB orange, scissors
violet, other same-hand ink, alternation faint; same-hand and alternate start hidden; legend chips toggle each class and show its %).
`spin` (Board tab) resets the camera and auto-rotates until the first interaction; children
render in board-centred coordinates (`boardBounds`, `place`, `CAP_H`; the Hand tab's
skeletal hands; a `focus` region makes the camera frame board and hands; `pressed`, a per-key depth array read each frame, sinks and tints keys without re-rendering). A 3D toggle (default on) switches between the orbit camera and a flat top-down view with rotation
locked; both presets fit the board to the viewport. A floor grid (1u cells, 5u sections) sits under the case. Flat and curved boards use the same code path.

## API (gecko serve)

| Route | |
|---|---|
| `GET /api/meta` | defaults, algos, metric definitions, core count |
| `GET /api/layouts` · `/api/geometries` · `/api/profiles` · `/api/corpora` | data dir contents |
| `GET /api/corpora/{name}` | `.gkc` bytes (for WASM) |
| `GET /api/corpora/{name}/sources` · `/sources/{file}` | source texts of a corpus (`data/corpora/src/<name>/`; loose `src/` files = `english`): list `[{name, bytes}]`, or one file as text |
| `POST /api/score` | `{geometry, corpus, profile \| profile_override, chars \| layout, top}` → analysis |
| `GET /api/runs` · `POST /api/runs` | list / start (`algo, time_ms \| evals, threads, seed, pins, sa_steps, ga_pop, memetic, moves, holdout` + score inputs) |
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
`npm run test:parity` builds WASM and the native CLI and checks every reference score and
metric against fresh CLI output across all bundled profiles (1e-6 relative).
`npm run test:contract` checks the `api.ts` types against real server JSON ([testing.md](testing.md)).
