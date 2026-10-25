# Parameters

Every input GECKO reads, by the file or flag that sets it, with its default. Three groups: what
**describes** a keyboard and its text, what **steers** a search, and what **judges** a layout
(the profile; metric definitions: `metrics-catalog.md`). This page is the index; the linked
topic docs explain behaviour.

## A. Describing

### Board (`data/geometries/<name>.toml`, [representation.md](representation.md#geometry))

| Field | Default | Notes |
|---|---|---|
| `name`, `description` | — / `""` | |
| `[comfort]` | none | Comfort metric → key-pair table, e.g. `engram_formula = "engram-2key"` (`data/comfort/<table>.csv`) |
| `[[key]]` | — | One per slot, row by row; fields below |

| Key field | Default | Notes |
|---|---|---|
| `pos` | required | `[x, y, z]` in u (1u = 19.05 mm), unrotated top-left at the keycap bottom; z = 0 on flat boards |
| `rot` | `[0, 0, 0]` | Degrees about x, y, z (tilt, tent, splay) about the keycap centre; flat boards use z only |
| `size` | `[1, 1]` | Width, height in u |
| `finger` | required | `LP LR LM LI LT RT RI RM RR RP`; the hand follows from it |
| `row`, `col` | required | Logical grid (i8). Thumbs are row 3, a number row −1 |
| `home` | false | Home key of its finger |
| `stretch` | false | Index stretch column; derived when no key on the board sets it ([geometry.md](geometry.md#derived-effort-and-stretch-g4)) |
| `effort` | derived | Base press cost; missing = derived from the distance to the finger's home key |
| `hrm` | rule | Home-row mod for `hrm_misfire`; unset = home keys on a finger outside the stretch columns |
| `fill` | none | What a layout that lists fewer keys puts here (token name or one char), e.g. thumb `space` |
| `alt` | `[]` | `[{ finger, effort? }]` other fingers for `optimize --fingering` ([objective.md](objective.md#fingering-search-c11)) |

`layer`, `phys`, `layer_keys` and `layer_modes` exist only on the virtual board that layered
scoring builds (`Geometry::layered`); board files do not set them. Fingering variants (angle
mod) are separate boards (`ansi-30-angle`, `ansi-30-alt`).

### Parametric board (`data/geometries/<name>.param.toml`)

Zones, rows and columns (stagger, splay, spread, split, thumb arc) and the 3D fields (curvature,
lift, tent, tilt): tables with defaults in [geometry.md](geometry.md#shape-and-metadata) and
[geometry.md](geometry.md#3d-shape). Which of them a geometry search may move: `--free`
([algorithms/geometry-search.md](algorithms/geometry-search.md)).

### Hand (profile `[hand]`)

`size = "small" | "average" | "large"` or `[hand.dims]` (segment lengths, palm, span, mm) and
optional `[hand.body]` (`elbow_width` 490, `forearm` 255 mm, scaled with hand length). Unset =
the average hand. Fields and ranges: [hand-model.md](hand-model.md#inputs).

### Layout (`data/layouts/<name>.toml`, [representation.md](representation.md#layout))

| Field | Default | Notes |
|---|---|---|
| `name` | required | |
| `geometry` | `""` | Board it was written for |
| `source` | `""` | Where it comes from |
| `rows` | required | Base layer, one string per board row; tokens such as `{space}`, `{layer1}` |
| `[[layer]]` | none | `name`, `rows`, `mode = "hold" \| "oneshot" \| "toggle"` (default hold) |
| `shifted` | `[]` | Shift pairs, base then shifted symbol: `["-_", "[{"]` ([keysets.md](keysets.md)) |
| `[[magic]]` | none | Adaptive key: `key` (glyph or token), `rules` (`"ao cy"`), `repeat` (false) |

Pins are not part of a layout: `--pin` per run.

### Text (corpora and languages, [representation.md](representation.md#corpus))

| Where | Parameter | Default | Notes |
|---|---|---|---|
| `<name>.mix.toml` | `description` | `""` | A language recipe (`gecko corpus mix`) |
| | `[[part]]` `corpus`, `weight` | — | Component corpora, weights normalized |
| | `edits` | 0 | Modelled `⌫` share of all keystrokes (bundled languages: 0.056) |
| | `nav` | 0 | Modelled arrow share of all keystrokes |
| `corpus build` | `--charset` | `default` | `default` (33 symbols), `code` (printable ASCII, digits, `⏎`, `⇥`, case), `nav`, or a symbol list |
| | `--fold` | off | Latin diacritics and ligatures → ASCII |
| | `--paragraphs` | off | Hard-wrapped prose: blank line = one `⏎` |
| | `--strip-markup` | off | Gutenberg `_emphasis_`, `[Illustration]`, `* * *` |
| `corpus edits` / `nav` | `--share` | 0.056 / 0.03 | Same models as a recipe's `edits` / `nav` |

Defaults for every command (board, corpus, profile, reference layout, data and run dirs):
`gecko.toml`.

## B. Steering a search

### Every run (`gecko optimize`)

| Flag | Default | Notes |
|---|---|---|
| `--algo` | `sa` | `hc sa ga memetic bls tabu explore qd nsga2 rwsa` |
| `--time` / `--evals` | 10 s | `--evals` is deterministic per seed and thread count; `--time` is not |
| `--seed` | 1 | |
| `--threads` | all cores | 0 = all cores |
| `--top` | 10 | Distinct best layouts kept |
| `--pin` | none | Symbols that stay on their slot |
| `--from` / `--keyset` | `gecko.toml` `reference` | Start layout, or build one ([keysets.md](keysets.md)) |
| `--moves`, `--depth2-width` | `swap`, 32 | Extra descent neighbourhoods `columns,cycle3,depth2` or `all` ([algorithms/README.md](algorithms/README.md)) |
| `--holdout` | none | Second corpus scored but never searched |
| `--fidelity K` | exact | Sets the profile's `fidelity.top_k` |
| `--fingering`, `--fingering-evals` | off, 300 000 | Fingering search after the layout search |

### Per algorithm (library defaults; `optimize/`, `explore/`)

| Algorithm | Parameter (flag) | Default |
|---|---|---|
| SA | `steps` (`--sa-steps`) | 0 = auto: about 4 restarts per worker in the budget |
| | `accept0`, `t_end_ratio` | 0.5, 0.002 |
| GA | `pop_size` (`--ga-pop`), `tournament`, `elitism` | 128, 3, 4 |
| | `crossover_rate`, `mutation_rate` (swaps per child) | 0.9, 1.0 |
| Memetic | `pop_size` per island (`--memetic-pop`), `elitism` | 24, 2 |
| | `local_search` (`--local-search hc\|tabu`), `local_steps` (`--local-steps`) | hc, 12 |
| | `islands` (`--islands`), `migration_interval`, `migrants` | one per thread, 5, 2 |
| | `collapse_generations` | 3 |
| BLS / explore | `kick_min`, `kick_max` | 2, 0 = a third of the movable symbols |
| | `accept_worse`, `stagnation`, `column_kick` | 0.1, 150 kicks, 0.2 |
| | explore: `exclusion`, `exclusion_dist` | 0.5, 8 |
| Explore probe | `samples` (`--samples`), `walks`, `walk_len`, `max_optima` (`--max-optima`) | 2000, 64, 400, 100 000 |
| | `within` (coverage band), `snapshot_ms` | 0.01, 2000 |
| Tabu | `tenure_min`, `tenure_max` (× movable symbols) | 0.9, 1.1 |
| | `diversify` (× m²), `stagnation` (× m), `refresh` | 5, 100, 1000 |
| MAP-Elites (`gecko qd`) | `axes` (`--axes`), `bins` (`--bins`), `seeds` (`--seeds`) | `roll_ratio,index_load`, 16, 64 |
| | `kick_min`, `kick_max`, `descent` (`--descent`), `snapshot_ms` | 2, 6, 0 = full descent, 2000 |
| NSGA-II / RWSA | `objectives` (`--objectives`) | `sfb,inroll,effort` |
| | `pop_size` (`--ga-pop`), `generations` (`--nsga-generations`), `archive_size` (`--front-size`) | 128, 0 = budget, 512 |
| | `crossover_rate`, `mutation_rate` | 0.9, 1.0 |

Parameters without a flag are set only through the library (tests, benches).

### Board + layout search (`gecko geometry optimize`, [algorithms/geometry-search.md](algorithms/geometry-search.md))

| Flag | Default | Notes |
|---|---|---|
| `--free` | — | `2d`, `3d`, `keys`, `keys2d` or a parameter list with bounds |
| `--mode` | `bilevel` | `bilevel`, `alternate`, `board` |
| `--time` / `--evals` | 60 s | `--evals` counts boards |
| `--inner-evals` | 20 000 | SA swaps per candidate board (inner `accept0` 0.05) |
| `--polish-evals` | 1 000 000 | Final SA polish; with `--time`, 20 % of the budget |
| `--sigma0`, `--popsize` | 0.1 of each range, CMA-ES default | |
| `--margin` | 0 u | Keycap clearance per side |
| `--penalty`, `--hard-fail` | 10, 2 u | Collision penalty per u of depth; depth that rejects a board |
| `--rounds` | 4 | Alternate mode only |
| `--keep-effort` | off | Keep hand-entered efforts instead of deriving them per board |
| `--thumb-keys` | board's | `N` or `auto` (keys along the thumb's arc) |
| `--smooth`, `--repair-passes` | 0.05, 20 | Free-form (`keys`) only |

Build limits come from the profile's `[build]` (below).

## C. Judging (the profile, `data/profiles/<name>.toml`, [objective.md](objective.md))

| Field | Default | Notes |
|---|---|---|
| `name`, `description` | — / `""` | |
| `[weights]` | 0 | Metric id → weight ≥ 0; direction comes from the metric |
| `[scale]` | 1 | Metric id → spread; the scorer applies `weight / scale` (`gecko profile normalize`) |
| `scale_basis` | `""` | How the scales were measured (informational) |
| `finger_target` | `[8, 11, 15, 16, 0, 0, 16, 15, 11, 8]` | % per finger for `finger_target`, LP … RP |
| `thumb_target` | 18 | Thumb share % for `thumb_load` |
| `finger_effort` | none | `typing`, `lateral`, `vertical` per finger, `symmetric` (true); replaces the targets ([objective.md](objective.md#relative-finger-effort)) |
| `row_offset` | 0 | `effort` added off the home row: pinky ring middle index × [above, below] |
| `[sfb]` | | Same-finger distance shape (retired distance metrics) and `sfs_far` decay |
| | `distance` | `linear` (or `squared`, `fitts`) |
| | `lateral`, `finger_factor` | 1.0, `[2.0, 1.538, 1.111, 1.0, 1.0]` (pinky ring middle index thumb) |
| | `skip_decay` | 0.5 (gap 2 × decay, gap 3 × decay²) |
| `trigram_coverage` | 0.99 | Share of trigram volume kept (renormalized); 1.0 = exact |
| `fidelity` | none (exact) | `{ top_k, grow = 4, misses = 3 }`: search speed only ([objective.md](objective.md#adaptive-fidelity-b8)) |
| `[hand]` | average hand | See A |
| `familiar_to` | none | `"qwerty"`: reference for `familiar` |
| `ctrl` | 0.01 | Ctrl shortcut presses per keystroke, charged only with a `⎈` key |
| `thumb_zone`, `base_letters` | none, false | `zones`: symbols that belong on thumbs; letters on the base layer ([keysets.md](keysets.md#special-keys)) |
| `vim` | none | `row`, `pairs` or `fingers` policy for `h j k l` ([vim.md](vim.md)) |
| `[build]` | off | Geometry-search build limits: `min_pitch` 0.82 u, `max_key_angle` 60°, `max_height` 4 u, `max_plate` [12, 12] u, `piece_gap` 1.25 u, `max_column_keys` 6, `max_columns` 0 (off), `case` off (`mx` / `choc`) ([geometry.md](geometry.md#manufacturability-g07)) |

Runs record the whole profile and its hash (`config.json`). `Profile::table_hash` covers
everything but weights and scales, so raw metric vectors re-score under any profile with the
same tables.
