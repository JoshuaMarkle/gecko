# Web UI: the screens

What each tab shows and does. Stack, code layout, rendering and API: [web.md](web.md).
Visual design: [design.md](design.md).

## Shell

Navbar (see design.md "App shell"): Home | Inputs (Board, Hand, Language, Profile, Summary) |
Optimize (Layout, Board, Both, Explore) | Saved (Runs, Bench, Export), icon tabs without labels so more
tabs fit. Play / pause / stop sit centred with run % beside, on Optimize tabs (and on any tab
while a run is going). Working model right. The tab is remembered in `localStorage`; old names
map (Corpus → Language, Search / Analysis / Stats / Optimize → Layout, Saved / Results → Runs).

- Inputs live in one store (`inputs.ts`); Play starts what the current Optimize tab searches:
  a layout run (Layout), a board-only run (Board), a board + layout run (Both) or an exploration
  (Explore); it resumes when paused. Board and Both are disabled on boards without a parameter
  file. Pause freezes the workers and the budget clock; Stop cancels (results so far are
  saved). The % is time or evaluations used of the budget.
- **Working model**: the layout adopted when a run finishes, when a saved layout is picked
  (Runs) or from Explore / Runs (Open in Layout). Navbar right shows its name and score, "No
  model" without one, and a live status line during a run.
- Input tabs and the Layout / Board / Both tabs share the three-panel `Frame`: sidebar | main
  visual over a bottom panel.
- Every board has a 3D view (`Keyboard3D`) and a 2D drawing (`Board2D`: `KeyGrid`,
  `FitBoard`, `Thumb`) from `geometry.flat`; 2D is used for board cards, thumbnails, the
  layout editor, heatmaps and per-key overlays.

## Home
The real start page: one flowchart (`FlowChart.tsx`, data-driven SVG) of how the optimization
works, every box a link to its tab with a live one-line state. Board, Hand and Profile feed
**Physical cost** (effort per key, finger load, hand model); Language feeds **Frequencies**;
both feed the **Scoring model** (QWERTY's score), with Summary hanging below it (dashed). The
scoring model feeds the four optimize modes (Layout, Board, Both, Explore; the last-used one is
outlined blue, Board / Both are amber when the board is not parametric), then Runs → Export.

## Board
Sidebar "Boards" with a **Custom** switch: off = 2-column board cards (2D drawing,
`name · keys`); on = physical constraint groups (rows/columns, stagger, splay, thumb
cluster, spacing, keys per finger, size), each enabled on its own (saved, not applied yet).
Main: 3D board; selecting a card resets the camera and starts a slow rotation that eases
to a stop on any interaction and does not resume until another card is picked. Clicking a
key opens a popup (x, y, z, rx, ry, rz, finger); the first edit forks the board to
`<name> (custom)` (browser storage, WASM-scored, cannot be run yet). Bottom: quick stats
(keys, split, rows × columns, thumbs, stagger, size) and a rough geometry-only estimate
(`board.ts`) ranked against the other boards. Below the bundled boards, **From board + layout
runs** lists each finished geometry run's best board (`run:<id>`, newest first).

## Hand
Sidebar tabs: **Hand** (default) and **Simulation**. Hand contains **Use hand in optimization** (default off; off greys out the settings, the views
still preview; on, every score and run gets this hand as the profile's `hand` plus G01 1.0 / G03 1.0 /
G05 0.5 when the profile has no weight for them, `hand.ts` `withHand` applied in `inputs.ts`), presets Average / Large / Small (Custom appears after an edit), Advanced:
every segment length, palm width / length, span, reach, and Arms (elbow width, forearm; sent as
`hand.body` once edited, "Arms from hand size" clears them). The 3D thumb is the scorer's 4-DoF
thumb (`hand.ts` `Thumb`, solved in hand-local mm via `toLocal`). Main: skeletal hands in 3D on the
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
a synthetic population). A **Reach** bottom tab (first) lists the scorer's per-key hand costs
(WASM `hand_costs`, `handCosts.ts`) per finger: home, mean, hardest key, press angle and keys out
of reach (mm short), then each wrist's resting deviation, extension, pronation and mean G03; the
**Reach** view toggle (default on) colours the 3D keys by G01.

## Language
Sidebar "Languages · N": collapsible rows with a status (Summarized / Unsummarized /
Outdated); expanding lists sources with sizes; clicking a source opens it in the text
viewer. Bundled corpora are Summarized and read-only; choosing one makes it the corpus for
scoring and runs (`in use`). Footer: New language, Upload sources (local languages only).
Navbar **Summarize** builds a local language's statistics in WASM (enabled when it is
Unsummarized or Outdated and has sources). Main: text viewer (first 400k characters) or
"No text selected". Bottom: characters, words, sources, charset; key-frequency bars; top
bigrams, trigrams, skipgrams (`corpus_stats`). Local languages cannot be used for runs yet.

## Profile
Sidebar: profile picker + reset, then every parameter in sections (Finger load with
relative effort inputs, Rows and reach, Hand model (G01/G05/G03 weights; visual: G01 per key for the
profile's hand, ringed keys out of reach), Same-finger with skip decay, Stretches and scissors,
Rolls and alternation, Redirects, Measured comfort and timing (E01 is a judge, weight 0 by
default), Layers, Shortcuts and familiarity,
Precision); each field has a tooltip. Retired metrics (B03, S03) are not shown. Scroll-spy marks the focused section (click a title to
jump). Main: a visual of the focused section on the current layout (finger heat + finger
load, effort heat, SFB/SFS arcs, stretch arcs, roll arcs, a redirect example, layer keys,
Z X C V and keys moved from QWERTY). Bottom: what it measures, why, how weights act, and the
section's metrics (value, weight, contribution). Edits re-score live and are sent with
each run (`profile_override`).

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

## Layout (optimize the layout for the board)
Sidebar: algorithm; starting layout (Auto = the loaded layout, or a named layout) and **Edit
layout**; **Symbols** (the starting layout's, or a keyset: letters only, letters + Shift-paired
symbols with a Pairs field, letters + a symbol layer; Place modifiers, Letters on thumbs;
[keysets.md](keysets.md)); budget (time or evaluations), threads, seed; holdout corpus (L06: results are also
scored on it, never searched on); stop confidence (saved, not applied yet); the algorithm's own
parameters (memetic islands: population per island, HC/tabu refinement and its step cap, island
count (0 = threads), migration interval (0 disables), migrant count; `POST /api/runs` accepts
optional `memetic` in the core `MemeticParams` shape). Main: **Result** (3D working model or live
best; Default / Heat / Fingers / N-grams) or **Algorithm** (a short description and animation),
or the **layout editor** overlay (`LayoutEditor`: 2D board, click two keys to swap, Pin mode,
layer picker, heat, undo). Bottom: convergence (hover: mini keyboard of the best layout then) |
"Chance we've found the optimal layout" from past runs on the **same inputs** (board, corpus and
profile hashes, not names; `estimate.ts` `sameInputs`), time to optimum, search-space size (after
pins) and the Gilmore–Lawler lower bound (Load) | score vs the reference (none when the layout is
the reference) and the main metrics.

## Board · Both (optimize the board; board and layout together)
Only on parametric boards, otherwise a note says why; a note also warns when the profile
weights no hand-model metric. **Board** keeps the layout fixed (pick it under Layout) and spends
the whole budget on the board's shape (`--mode board`). **Both** starts from a layout and
searches board and layout together (mode bilevel / alternate, layout swaps per board). Sidebar:
**Board shape** (2D / 3D / Custom presets and one switch per group: stagger, splay, spread,
thumb cluster, row and column curvature, column height, tenting, tilt, thumb 3D), **Board search
settings** (clearance, collision penalty, keep key efforts), budget in time or boards, threads,
seed. Main: the 3D board with the picked parameter at a previewed value (slider across its
bounds, Sweep, Reset), the start board outlined, colliding keys red; during a run the live best
board. Bottom: every free parameter (`/api/geometries/{id}/free`) with editable lower / upper
bounds (kept per board; Default bounds resets) and a range bar; click a row to preview it. When
a run finishes its best board becomes the board (`run:<id>`) and its layout the working model.

## Summary
Last input tab: a full-width, scrollable reference sheet with paired parameter tables,
a complete metric table (weight, scale, effective weight, direction and description; retired
metrics left out),
finger costs / generated targets, and per-key geometry. Reads the live shared inputs;
all sections stay expanded. Gray rows identify zero-weight metrics, missing comfort tables
or familiarity references, other algorithms' settings, and saved-only board / stop-confidence
controls. **Hand & simulation** shows whether the hand model is used and which hand-model
metrics the profile weights. This describes the next Layout run, not a saved run or Explore.

## Runs
All runs, each with a mini keyboard of its best layout; the selected run's convergence (hover as on Layout), best layouts as 2D thumbnails (click one to make it
the working model) and reproducibility facts. With a holdout corpus, the best-layouts table adds
each layout's score on it, and the Run facts show the holdout corpus, start → best on both
corpora, the gain kept and the rank ρ.

**Process** (`runs/ProcessPanel`, from the `workers` trace in `progress.jsonl`): Workers (one
thin line per worker's current score, the best so far bold), Schedule (SA temperature on a log
scale over the share of worsening moves accepted, blue, and of improving proposals, dashed),
Symbol migration (slot × time raster of the best layout or of one worker's current layout,
each symbol in the color of the finger it ends on; hover names it) and Settling order (when
each symbol last moved in the best layout, latest first, with its move count). Older runs and
population algorithms show only the raster and settling order.

NSGA-II / RWSA runs add **Pareto front** (`components/runs/ParetoPanel`): the front as a scatter
over any two metrics (objectives starred, the run's first two by default; the line joins the
layouts non-dominated in that plane); hover shows a mini keyboard, click makes the layout the
working model and shows its keyboard and objective values below. qd runs add **MAP-Elites grid**
(`runs/EliteGrid`, from `/api/runs/{id}/qd`): cells over two chosen behaviour axes (or one
row), shaded white → ink by elite score (darker = better), empty cells outlined; with more
than two axes a cell shows the best elite projecting onto it. Hover shows the elite's
descriptors and keyboard; click makes it the working model (outlined in blue).

A board or board + layout run (algo "board") opens its own panel: the board at any point of the run over
the start board's dashed outline (2D top-down or 3D; a scrubber and Play step through every
generation to the polished best board), convergence, result (start vs best board after equal
layout polish, collisions, the weighted metrics that moved most), every free parameter (start,
best, bounds bar) and the run facts. **Use this board** makes its best board the board and its
layout the working model.

## Bench
Saved `gecko bench` files (`runs/bench/`, `/api/bench`), newest first, with their solvers,
seeds and budget. The selected bench: leaderboard (mean, best, stdev, seeds that hit the
overall best, speed), **Performance profile** (Dolan–Moré: share of seeds on which a solver
is within τ of the best solver on that seed), **Time to target** (ECDF over seeds of the time
to reach the overall best + 0 / 0.5 / 1 / 2 %) and **Best so far** (each seed's trace, dashed
target). Solvers keep one series color across the three charts. No runs start from here; the
empty state shows the CLI command.

## Export
Last tab: the shown layout (working model, or the live best during a run) as files.
Sidebar: name (defaults to the model's label), image options (key color Plain / Fingers /
Heat, all layers, title and score, PNG scale) with Download PNG / SVG, then every file with
Copy and Download: GECKO layout `.toml` (the `data/layouts` format; `{token}` names, one
`[[layer]]` per layer), text, JSON (layout, layers, score, every metric, finger load, run and
holdout facts), KLE (the board's 2D drawing with legends, layers as extra legends, the
image's key colors), Oxeylyzer / Genkey / cmini (30 alpha keys, through WASM
`export_layout` = `gecko layouts export`; disabled with the reason on boards without a
3 × 10 alpha block), metrics CSV and a Markdown summary. **3D print** (STL): switch MX / Choc,
plate, wall, clearance and bridge (mm, over the switch's defaults), Download case / plate /
bottom `.stl` (WASM `CaseBuild` = `gecko build stl`, piece gap from the default build limits),
then pieces and their size, connectors left out, blocked switch holes and keycap hits.
Main: a **3D** toggle (on by default when any key is raised or tilted, off for flat boards;
reset when the board changes). Off: the image exactly as exported (`boardSvg`, a standalone
SVG; layers two per row). On: `three/Case3D` renders the STL triangles (bottom plate darker)
with switch housings and the base layer's keycaps; **Keycaps** and **Bottom** toggles hide
them, keycaps that hit the case when pressed are red. Bottom: score vs the reference, board /
language / profile / run / holdout, weighted metrics and finger load; "No stats" when the
layout is not scored (stats-only files are disabled, the rest still export).

## Explore (`components/explore/`)

Left: atlas picker, new exploration (budget, threads, seed; other inputs from the input tabs), ranking
(atlas profile, or the current profile with editable weights; every panel re-ranks), focus
(best N, family radius, brush) and landscape stats. Centre, linked cards: Map (PCA of slot,
finger or metric encodings), Fitness vs distance (+ reference layouts), Consensus keyboard,
Families, Trade-off (any two metrics, Pareto front), Local optima network, Path, Funnels,
Parallel coordinates, Weight stability, Discovery curve, How good is good (random vs optima),
Ruggedness. Right: the picked layout — metrics vs best, provenance, its whole swap
neighbourhood; Open in editor; Set as path start.

**Local optima network** (`Lon.tsx`): the first 300 focused optima and the kick edges among
them (`edges.bin`; repeats merged, width ∝ count), force-directed (`lon.ts`, seeded, so the
same atlas lands the same way) or with y = score (better lower). Node size ∝ log hits, color
= funnel (the 7 best funnels, the rest grey), funnel bottoms ringed in ink, picked in blue;
click picks. **Path** (`PathPanel.tsx`): path relinking from the path start (the best unless
set in the right pane) to the picked layout, each step the cheapest swap that puts one more
symbol where the target has it (WASM `Engine.path` when the scorer matches, else
`/api/path`). Score along the path, its barrier above the worse end, and the layout at the
hovered step (grey keys still differ from the target).

All atlas statistics run in WASM (`AtlasView`) on the binary files; the live atlas reloads
on each snapshot (throttled 2.5 s). Reference layouts are scored via `/api/score`. Swap
neighbourhoods use WASM when the geometry and corpus match the loaded scorer, else
`/api/swap-matrix`.
