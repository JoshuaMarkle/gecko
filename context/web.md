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
  wasm.ts              gecko-wasm loader (Scorer (analyze, swapMatrix, path), flat2d, export_layout, corpus stats / build, profileHash,
                       paramBoard: a parametric board at given parameter values + collisions,
                       buildCase: plate / walls / bottom triangles and STL files)
  format.ts            pure helpers (rows, swaps, layers, verdicts, validation, cap)
  export.ts            Export tab file builders (image SVG, layout TOML/text/JSON, KLE, ergogen, stats)
  board.ts hand.ts handCosts.ts estimate.ts explore.ts fingerEffort.ts languages.ts
                       per-view logic; pure parts unit-tested (*.test.ts); hand.ts is checked
                       against Rust by hand.parity.test.ts
  colors.ts            data colors (finger palette, heat ramp)
  components/          Form (Frame, Sec, Num, …), Board2D, LayoutEditor, Charts, Icons
    views/             one file per tab (Home, Board, Hand, Language, Profile, Summary, Layout,
                       BoardSearch (Board and Both), Saved (Runs), Export); FlowChart is
                       Home's inputs → physical cost / frequencies → scoring → optimize diagram
    hand/              Hand tab parts: rigs (hand placement), Hands3D, HandMesh (skinned hands,
                       assets/: WebXR generic-hand glTF, MIT), Panels (bottom tabs)
    explore/           Explore tab: ExploreView, Panels, Board, Scatter, Picked, Lon (local optima
                       network), PathPanel (path between two layouts)
    runs/              Saved view panels for multi-objective (ParetoPanel) and qd (EliteGrid) runs,
                       and every run's process views (ProcessPanel)
    views/BenchView    Saved › Bench: saved `gecko bench` leaderboards
    geom/              board + layout search: GeomSearch (Board / Both tab parts, `useGeomVars`),
                       GeomRunPanel (Saved), GeomBoard (`useParamBoard`, 2D overlay)
  runViews.ts          metric columns of ranked layouts, 2-axis projection of a qd grid
  process.ts           worker lines, schedule, layout frames and settling order from progress records
  bench.ts             performance-profile gaps, time-to-target ECDFs of a bench
  lon.ts               local optima network subgraph + deterministic force layout
  geom.ts              free-parameter groups, `--free` spec, labels, progress → chart shape
  three/Keyboard3D.tsx 3D keyboard (auto-rotate, click position, scene children)
  three/Case3D.tsx     Export tab: printable case from WASM `CaseBuild`, switches and keycaps
web/contract/          API contract check (loose.ts; generated samples are gitignored)
```

## 2D boards

`Board2D` draws `geometry.flat` (gecko-core `Geometry::flat2d`, so curved boards get a clean
projection) via `format.ts` `flatKeys` / `flatBounds`, with each key's in-plane rotation and
optional arcs between keys. Custom boards get `flat` from WASM `flat2d`.
`Board2D.Thumb` is the one **mini keyboard** for every place a layout is referenced: Saved run
rows and best layouts, Explore families and point tooltips (map, FDC, trade-off; keys that
differ from the best shaded), and convergence tooltips (Home, Saved). `width` caps its size.

## 3D rendering

Each key is a keycap (sharp base, tapered walls with smooth rounded corners, flat top; `capGeometry`, cached per size) placed from the geometry's `pos`, `rot`, `size` (1 unit = 1u =
19.05 mm; board y → three z). Legends are cached canvas textures. Overlays: none (default,
plain keys), frequency heat, finger colors, n-grams (finger-colored keys plus one arc per undirected bigram or skipgram, all
of them; width and opacity ∝ √(frequency / most frequent visible pair); colored by class: SFB/SFS red, LSB orange, scissors
violet, other same-hand ink, alternation faint; same-hand and alternate start hidden; legend chips toggle each class and show its %).
`spin` (Board tab) resets the camera and auto-rotates until the first interaction; children
render in board-centred coordinates (`boardBounds`, `place`, `CAP_H`; the Hand tab's
hands; a `focus` region makes the camera frame board and hands; `pressed`, a per-key depth array read each frame, sinks and tints keys without re-rendering). A 3D toggle (default on) switches between the orbit camera and a flat top-down view with rotation
locked; both presets fit the board to the viewport. A floor grid (1u cells, 5u sections) sits under the case. Flat and curved boards use the same code path.
Boards with keys below the case top (tented or tilted halves pivot about their inner edge) are
raised until every keycap clears it. `colors` overrides key colors (collisions); `ghost` outlines
another board's keys in the same coordinates (a geometry run's start board).

Hand tab hands: a Skin toggle (default on) shows skinned meshes (WebXR generic hand, 25 bones),
else the joint skeletons. Each frame the bones are set from the solved joints (`HandPose`): the
wrist frame from the knuckles, finger metacarpals carried by the wrist, every other bone swung
from its parent onto its posed segment and stretched to its length; uniform scale from palm size.
The model's tip joint sits above the pad, so the mesh poses from targets lifted per finger
(`padLift`, fit by `HandMesh.test.ts`). Finger skin tints by state; the forearm is clipped 30 mm
behind the wrist.
Hovering a key (`Keyboard3D` `onHover`) sends the finger that types it there while typing is
stopped (`reachFor`, then `framesAt`'s `over`): the fingertip eases toward the key top
(exponential, 12/s, lifting up to 10 mm on the way), the hand follows by its wrist-movement
share, and the IK poses the rest. Leaving the key eases the finger back home.

## Export

The Export tab turns the working layout into files (`export.ts`, pure builders tested in
`export.test.ts`): an SVG/PNG image; **Files** (GECKO layout TOML, text, JSON); **Other tools**
(KLE, ergogen, and Oxeylyzer/Genkey/cmini through WASM `export_layout`); **Stats** (metrics CSV,
Markdown). It also previews and downloads the printable case (STL: `three/Case3D`).
KLE and ergogen both use the board's 2D drawing (`flatKeys`). The ergogen file (`ergogenYaml`)
holds one single-key zone per key, anchored at the key center in mm (1u = 19.05 mm). Its y axis
points up and its rotation is counter-clockwise, so both are negated. Each key gets keycap
`width`/`height`, and its legends, finger, row and column as custom key attributes. A `plate`
outline adds 14 mm switch cutouts. Load the file at ergogen.xyz to build a PCB or case.

## Board + layout search

Parametric boards (`/api/geometries` sends their parameter file as `param`) can be searched with
the board's shape free (gecko-core `geomsearch`, [algorithms/geometry-search.md](algorithms/geometry-search.md)).
The browser builds any candidate board with WASM `param_board` (same generator and collision
test as the search), so previews, the live board and the run replay need no server round trip.
A finished run's best board is listed as `run:<id>` (with `run` and its own `param`), so it can
be scored, layout-optimized or shape-searched again.

## API (gecko serve)

| Route | |
|---|---|
| `GET /api/meta` | defaults, algos, metric definitions (with `retired`; `api.meta` drops retired ones for the UI), core count |
| `GET /api/layouts` · `/api/geometries` · `/api/profiles` · `/api/corpora` | data dir contents; geometries add each finished geometry run's best board (`run:<id>`) and `param` on parametric boards; geometries and corpora carry their content `hash` |
| `GET /api/geometries/{id}/free?spec=` | free parameters of a parametric board for a `--free` spec: `{vars: [{name, lo, hi, start}]}` |
| `GET /api/corpora/{name}` | `.gkc` bytes (for WASM) |
| `GET /api/corpora/{name}/sources` · `/sources/{file}` | source texts of a corpus (`data/corpora/src/<name>/`; loose `src/` files = `english`): list `[{name, bytes}]`, or one file as text |
| `POST /api/score` | `{geometry, corpus, profile \| profile_override, chars \| layout, top}` → analysis |
| `GET /api/runs` · `POST /api/runs` | list (each with `kind`: layout / geometry and the `geometry_hash`, `corpus_hash`, `profile_hash` of its inputs, so the UI matches runs by content, not names) / start (`algo, time_ms \| evals, threads, seed, pins, sa_steps, ga_pop, memetic, moves, holdout` + score inputs; with `geom: {free, mode (bilevel / alternate / board), inner_evals, margin, penalty, keep_effort, …}` a board + layout search, `evals` then counts boards) |
| `GET /api/runs/{id}` | config, result, progress, running, paused; a geometry run adds `kind: "geometry"` and its start `param` |
| `GET /api/runs/{id}/events` | SSE: `progress`, then `done` (RunResult) or `failed` |
| `POST /api/runs/{id}/cancel` | stop early (results are still saved) |
| `GET /api/runs/{id}/atlas` | explore run summary (`atlas.json`) |
| `GET /api/runs/{id}/qd` | qd run grid and elites (`qd.json`) |
| `GET /api/runs/{id}/atlas/{optima\|edges\|samples}` | atlas binary files (for WASM `AtlasView`) |
| `POST /api/swap-matrix` | score inputs + `chars` → Δ score of every slot-pair swap |
| `POST /api/path` | score inputs + `chars` + `to` → path relinking `{start, steps: [{a, b, score}], barrier}` (fallback for WASM `Engine.path`) |
| `GET /api/bench` · `/api/bench/{id}` | saved `gecko bench` files (`runs/bench/`), newest first without per-seed traces / one in full |
| `POST /api/runs/{id}/pause` · `/resume` | pause / resume a running run; paused time does not count against `time_ms` or toward `elapsed_ms` |

## Dev

`npm run wasm:dev` (after Rust changes to the scorer; skips wasm-opt, ~0.5 s) or `npm run wasm` (release, with wasm-opt), `npm run dev` (proxies `/api` to :7777),
`npm run build:dev` (vite only) / `npm run build` (tsc + vite) → `web/dist`, `npm run typecheck`, `npm test`.
`npm run test:parity` builds WASM and the native CLI and checks every reference score and
metric against fresh CLI output across all bundled profiles (1e-6 relative).
`npm run test:contract` checks the `api.ts` types against real server JSON ([testing.md](testing.md)).
