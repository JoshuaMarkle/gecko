# Web UI

For the human. Local only, served by `gecko serve`. Visual design: [design.md](design.md).
Not built yet (the open points of the redesign): [frontend.md](frontend.md).

## Stack

TypeScript (strict), Vite, React 19, Three.js via `@react-three/fiber` + `@react-three/drei`.
Charts are hand-rolled SVG. Scoring in the browser through `gecko-wasm` (same code as the CLI).

```
web/src/
  App.tsx              shell: navbar, view switch, run lifecycle (SSE), working model
  tabs.ts              tab lists, old-name mapping
  inputs.ts            the one input store (board, custom boards, constraints, hand, language,
                       profile, layout, pins, search, working model)
  api.ts               API client + types mirroring the Rust serde shapes
  wasm.ts              gecko-wasm loader (Scorer, flat2d, corpus stats / build)
  format.ts            pure helpers (rows, swaps, layers, verdicts, validation)
  board.ts hand.ts estimate.ts languages.ts   per-view logic (board stats, hand model + IK,
                       estimators, local languages); pure parts unit-tested (*.test.ts)
  colors.ts            data colors (finger palette, heat ramp)
  components/          Form (Frame, Sec, Num, …), Board2D, LayoutEditor, Charts, AnalysisView,
                       ResultsView (Saved), Icons, views/ (one file per tab), explore/
  three/Keyboard3D.tsx 3D keyboard (auto-rotate, click position, scene children)
```

## Screen

Navbar (see design.md "App shell"): Inputs tabs (Board, Hand, Language, Profile, Search) |
Results tabs (Home, Explore, Analysis, Saved) · play / pause / stop centred with run % beside
(on Results tabs only) ·
working model right. The tab is remembered in `localStorage`; old names map (Corpus →
Language, Stats → Analysis, Runs → Saved, Optimize → Home).

- Inputs live in one store (`inputs.ts`); Play on any Results tab uses them: the
  Search settings, an exploration on the Explore tab, or resume when paused. Pause freezes
  the workers and the budget clock; Stop cancels (results so far are saved). The % is time
  or evaluations used of the budget.
- **Working model**: the layout adopted when a run finishes, when a saved layout is picked
  (Saved) or from Explore (Open in Home). Navbar right shows its name and score, "No model"
  without one, and a live status line during a run.
- All five input tabs and Home share the three-panel `Frame`: sidebar | main visual over a
  bottom panel.
- Every board has a 3D view (`Keyboard3D`) and a 2D drawing (`Board2D`: `KeyGrid`,
  `FitBoard`, `Thumb`) from `geometry.flat`; 2D is used for board cards, thumbnails, the
  layout editor, heatmaps and per-key overlays.

### Board
Sidebar "Boards" with a **Custom** switch: off = 2-column board cards (2D drawing,
`name · keys`); on = physical constraint groups (rows/columns, stagger, splay, thumb
cluster, spacing, keys per finger, size), each enabled on its own (saved, not applied yet).
Main: 3D board; selecting a card resets the camera and starts a slow rotation that eases
to a stop on any interaction and does not resume until another card is picked. Clicking a
key opens a popup (x, y, z, rx, ry, rz, finger); the first edit forks the board to
`<name> (custom)` (browser storage, WASM-scored, cannot be run yet). Bottom: quick stats
(keys, split, rows × columns, thumbs, stagger, size) and a rough geometry-only estimate
(`board.ts`) ranked against the other boards.

### Hand
Sidebar tabs: **Hand** (default) and **Simulation**. Hand contains **Use hand in optimization** (default off; off greys out the settings, the views
still preview), presets Average / Large / Small (Custom appears after an edit), Advanced:
every segment length, palm width / length, span, reach. Main: skeletal hands in 3D on the
current board (`hand.ts`: one right-hand skeleton mirrored for the left, each hand placed by
fitting natural finger reach to the home keys, thumb on the nearest thumb key, finger IK
within joint limits; fingertips target each keycap's top centre and press along its normal,
so tilted keys on 3D boards such as the Dactyl work), a Lengths toggle for segment labels; the Simulation tab in the left
sidebar contains the Wrist controls (view only, outside the "use hand"
lock). These offset the solved placement, mirrored per hand: height, distance, spread,
angle, tilt (roll, + thumb side up) and wrist movement (how much the hand follows a reaching
finger); fingertips that can no longer reach their home key are listed in red. Play animates
a sample text on the loaded layout once or on Loop (the button returns to Play when a single
pass ends): every finger moves at its natural pace toward a target WPM
(5–200, default 120); a key waits until its finger gets there, so the speed reached can fall short. A Parallel slider (0–300%, default 100%) sets how early fingers set off for their
next key (100%: during the previous press; above: keys roll over; 300%: as soon as free).
The playback bar background fits its controls, wrapping within the viewport on narrower screens.
The wrist follows far reaches. Hands are lines with square joints; moving fingers are light
blue, pressing ones accent; a pressed key sinks 3.5 mm with the fingertip and turns blue.
Bottom tabs: Measurements (the hand drawn horizontally, fingers right, with dimension lines,
plus derived lengths), Typing (per-finger timeline of moves, waits and presses with a
playhead, late keys marked; speed reached vs target, late keys (same finger or late start),
keys started early, overlap, alternation, travel, per-finger load), Size distribution (normal curve of hand length,
Small / Average / Large bands, this hand marked), Similarity (typicality of this hand among
a synthetic population).

### Language
Sidebar "Languages · N": collapsible rows with a status (Summarized / Unsummarized /
Outdated); expanding lists sources with sizes; clicking a source opens it in the text
viewer. Bundled corpora are Summarized and read-only; choosing one makes it the corpus for
scoring and runs (`in use`). Footer: New language, Upload sources (local languages only).
Navbar **Summarize** builds a local language's statistics in WASM (enabled when it is
Unsummarized or Outdated and has sources). Main: text viewer (first 400k characters) or
"No text selected". Bottom: characters, words, sources, charset; key-frequency bars; top
bigrams, trigrams, skipgrams (`corpus_stats`). Local languages cannot be used for runs yet.

### Profile
Sidebar: profile picker + reset, then every parameter in sections (Finger load with
relative effort inputs, Rows and reach, Same-finger with SFB distance settings, Stretches and scissors,
Rolls and alternation, Redirects, Measured comfort, Layers, Shortcuts and familiarity,
Precision); each field has a tooltip. Scroll-spy marks the focused section (click a title to
jump). Main: a visual of the focused section on the current layout (finger heat + finger
load, effort heat, SFB/SFS arcs, stretch arcs, roll arcs, a redirect example, layer keys,
Z X C V and keys moved from QWERTY). Bottom: what it measures, why, how weights act, and the
section's metrics (value, weight, contribution). Edits re-score live and are sent with
each run (`profile_override`).

### Search
Sidebar: algorithm; starting layout (Auto = the loaded layout, or a named layout) and **Edit
layout**; budget (time or evaluations), threads, seed; stop confidence (saved, not applied
yet); the algorithm's own parameters. Main: a short description and a small animation of
the algorithm (bars being swapped, or a population evolving), or the **layout editor**
overlay (`LayoutEditor`: 2D board, click two keys to swap, Pin mode, layer picker, heat,
undo). Bottom: search-space size (arrangements, after pins), estimates from past runs on the
same inputs (rediscovery rate, chance optimal, time-to-optimum guess, `estimate.ts`), and an
**Estimate** button (bottom-right) that loads the Gilmore–Lawler bound of the best matching
run.

### Home (read-only)
Sidebar: summary blocks for Board, Hand, Language, Profile, Search with on/off flags; each
opens its tab. Main: 3D working model (3D toggle, Default / Heat / Fingers / N-grams).
Bottom: convergence, "Chance we've found the optimal layout" (rediscovery; bound gap when
the run has one), score vs reference and the main metrics.

### Analysis · Saved
Analysis: every chart for the current layout (click two heatmap keys to swap). Saved: all
runs; the selected run's convergence, best layouts as 2D thumbnails (click one to make it
the working model) and reproducibility facts.

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
`npm run test:parity` builds WASM and the native CLI and checks every reference score and
metric against fresh CLI output across all bundled profiles (1e-6 relative).

Finger load uses five fingers with symmetric hands by default; disable symmetry to edit
each hand. Each finger has typing, side-to-side, and up/down effort. Generated target
percentages are read-only, normalized from inverse typing costs over fingers on the board;
M09 uses the combined generated thumb target. Turning symmetry back on copies left to
right. Web profiles acquire these defaults on selection/reset; legacy CLI profiles keep
their percentage targets unless `finger_effort` is supplied.

Profile parameter rows omit trailing metric-ID labels and use equal left/right sidebar
padding (`--s4`, 16px). Finger effort groups appear in index, middle, ring, pinky, thumb order for each hand,
using standard spacing and hairline dividers, with explicit
Typing effort / Sideways effort / Vertical effort labels. Metric IDs remain in tooltips.
