# Web UI: the screens

What each tab shows and does. Stack, code layout, rendering and API: [web.md](web.md).
Visual design: [design.md](design.md).

## Shell

Navbar (see design.md "App shell"): Inputs tabs (Board, Hand, Language, Profile, Search, Summary) |
Results tabs (Home, Explore, Analysis, Saved, Export) · play / pause / stop centred with run % beside
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

## Board
Sidebar "Boards" with a **Custom** switch: off = 2-column board cards (2D drawing,
`name · keys`); on = physical constraint groups (rows/columns, stagger, splay, thumb
cluster, spacing, keys per finger, size), each enabled on its own (saved, not applied yet).
Main: 3D board; selecting a card resets the camera and starts a slow rotation that eases
to a stop on any interaction and does not resume until another card is picked. Clicking a
key opens a popup (x, y, z, rx, ry, rz, finger); the first edit forks the board to
`<name> (custom)` (browser storage, WASM-scored, cannot be run yet). Bottom: quick stats
(keys, split, rows × columns, thumbs, stagger, size) and a rough geometry-only estimate
(`board.ts`) ranked against the other boards.

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
profile's hand, ringed keys out of reach), Same-finger with SFB distance settings, Stretches and scissors,
Rolls and alternation, Redirects, Measured comfort, Layers, Shortcuts and familiarity,
Precision); each field has a tooltip. Scroll-spy marks the focused section (click a title to
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

## Search
Sidebar: algorithm; starting layout (Auto = the loaded layout, or a named layout) and **Edit
layout**; budget (time or evaluations), threads, seed; holdout corpus (L06: results are also
scored on it, never searched on); stop confidence (saved, not applied yet); the algorithm's own
parameters. Main: a short description and a small animation of
the algorithm (bars being swapped, or a population evolving), or the **layout editor**
overlay (`LayoutEditor`: 2D board, click two keys to swap, Pin mode, layer picker, heat,
undo). Bottom: search-space size (arrangements, after pins), estimates from past runs on the
same inputs (rediscovery rate, chance optimal, time-to-optimum guess, `estimate.ts`), and an
**Estimate** button (bottom-right) that loads the Gilmore–Lawler bound of the best matching
run.

Search includes memetic islands: population per island, HC/tabu refinement and its step
cap, island count (0 = threads), migration interval (0 disables), and migrant count.
`POST /api/runs` accepts optional `memetic` using the core `MemeticParams` shape; omitted
fields use defaults. The server limits islands to core count and caps population/refinement
work. Older saved Search preferences acquire defaults for the new fields.

## Summary
Above Board & language, an inline flowchart shows language frequencies and physical
keyboard costs feeding the scoring model, then the selected search algorithm and output
layout. The graphic is unframed with padded margins, page-style titles and subtitles, and
bulleted details, with no overall heading. It shares the right column with that table,
opposite Search & stopping, and scrolls on narrow screens. Box titles use the standard
12px section typography and details use 11px text; the SVG does not enlarge on wide screens.

Final input tab: a full-width, scrollable reference sheet with paired parameter tables,
a complete metric table (weight, scale, effective weight, direction and description),
finger costs / generated targets, and per-key geometry. Reads the live shared inputs;
all sections stay expanded. Gray rows identify zero-weight metrics, missing comfort tables
or familiarity references, other algorithms' settings, and saved-only hand / board /
stop-confidence controls. This describes the next Search run, not a saved run or Explore.

## Home (read-only)
Sidebar: summary blocks for Board, Hand, Language, Profile, Search with on/off flags; each
opens its tab. Main: 3D working model (3D toggle, Default / Heat / Fingers / N-grams).
Bottom: convergence, "Chance we've found the optimal layout" (rediscovery; bound gap when
the run has one), score vs reference and the main metrics.

## Analysis · Saved
Analysis: every chart for the current layout (click two heatmap keys to swap). Saved: all
runs; the selected run's convergence, best layouts as 2D thumbnails (click one to make it
the working model) and reproducibility facts. With a holdout corpus, the best-layouts table adds
each layout's score on it, and the Run facts show the holdout corpus, start → best on both
corpora, the gain kept and the rank ρ.

## Export
Last tab: the shown layout (working model, or the live best during a run) as files.
Sidebar: name (defaults to the model's label), image options (key color Plain / Fingers /
Heat, all layers, title and score, PNG scale) with Download PNG / SVG, then every file with
Copy and Download: GECKO layout `.toml` (the `data/layouts` format; `{token}` names, one
`[[layer]]` per layer), text, JSON (layout, layers, score, every metric, finger load, run and
holdout facts), KLE (the board's 2D drawing with legends, layers as extra legends, the
image's key colors), Oxeylyzer / Genkey / cmini (30 alpha keys, through WASM
`export_layout` = `gecko layouts export`; disabled with the reason on boards without a
3 × 10 alpha block), metrics CSV and a Markdown summary. Main: the image exactly as exported
(`boardSvg`, a standalone SVG; layers two per row). Bottom: score vs the reference, board /
language / profile / run / holdout, weighted metrics and finger load; "No stats" when the
layout is not scored (stats-only files are disabled, the rest still export).

## Explore (`components/explore/`)

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
