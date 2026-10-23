# Geometry search (continuous, phases 6–8)

Background: `research/sources/ergogen-dactyl.md`, `research/sources/search-methods-general.md`.

**Problem:** choose key poses (2D: x, y, θ; 3D: full pose) so the best layout on that
geometry scores as well as possible under the hand-model costs, with no intersecting keys.

**Approach, in order of implementation:**
1. **Parametric geometry:** a few parameters (column stagger, splay, row curvature, tent,
   thumb arc). Optimize with **CMA-ES**; each candidate scored by a short warm-started
   inner SA over the layout.
2. **Alternating optimization:** fix layout → optimize geometry (CMA-ES or gradient on a
   smooth surrogate of the cost tables) → fix geometry → re-optimize layout. Repeat until stable.
3. **Free-form:** per-key poses (high dimension). CMA-ES with restarts, or gradient-based
   if the hand-model cost is made differentiable. Collision as a penalty plus a repair step.
   Implemented as `--free keys` (below).

**Parameters:** `sigma0`, `popsize`, `inner_budget`, `collision_margin`, which parameters are free.

The candidate representation is implemented in `gecko_core::parametric`:
zones/columns/rows with column stagger/splay, split spread/angle and thumb arc/pose, plus 3D
row/column curvature, column lift and zone/split tent and tilt.
See [../geometry.md](../geometry.md); existing parametric Corne/ortho models reproduce the
explicit boards exactly. Building blocks: `Scorer::set_geometry` (fast table rebuild,
[../objective.md](../objective.md#geometry-rebuild-g2)), `Geometry::collisions` / `collisions_3d`
(penalty by depth, [../geometry.md](../geometry.md#collision-g04)), derived efforts that follow
moved keys, CMA-ES and the bilevel driver below.

## Board + layout search (`gecko_core::geomsearch`, G6)

`gecko geometry optimize -g <name>.param --free 3d -p hand --time 5m` (alias `gecko geom`).

- **Free parameters** (`--free`, comma list; `name=lo:hi` sets bounds): presets `2d` (column
  stagger, splay and spread, thumb cluster anchor/rotation/staggers) and `3d` (2d + row and column
  curvature, column lift, split opening angle, tent and tilt, thumb tent/tilt/lift/curve), groups of those, or
  single fields `split.<f>`, `<zone>.<f>`, `<zone>.<col>.<f>`. Finger zones are zones containing
  their home row; the others are thumb zones. Mirrored zones move both halves. Default bounds
  sit around the start value (stagger ±0.75 u, splay ±15°, spread and row pitch 1–1.6 u, i.e. not
  below MX pitch, row curve 0–30°, column curve 0–15°, split tent 0–45°).
- **Posture priors** (H6, `research/sources/split-keyboard-ergonomics.md`): split `tilt` −4–0°
  (slope 0 to −4°) and `angle` 10–20° (opening ≈ 15°) by default, absolute rather than around
  the file's value; the start board keeps its own values and CMA-ES starts from them clamped.
  Pass `split.tilt=lo:hi` / `split.angle=lo:hi` to search outside them.
- **Efforts** are dropped and derived from key positions per candidate (`--keep-effort` keeps
  the file's), so M01 follows moved keys.
- **Bilevel** (default): one full SA on the start board first, so candidates warm-start from a
  good layout. Then CMA-ES (σ0 0.1 of each range, IPOP restarts from the best point) proposes
  boards; each gets a 20k-swap low-temperature SA (accept0 0.05) from the incumbent layout plus
  a steepest descent. The incumbent is the layout of the reported board (best collision-free
  one when there is one), so an overlapping board never supplies it. All candidates of a generation share one inner seed (common random
  numbers) and run in parallel, one scorer per thread (`set_geometry`, ≈ 1 ms per board).
- **Alternate** (`--mode alternate`): rounds of a full SA on the current board, then CMA-ES over
  the board with that layout fixed (one rebuild + full score per candidate). Stops when a round
  gains nothing.
- **Board** (`--mode board`): the board for a fixed layout. The start layout (`--from`, else the
  board's) is never changed: no warm-start SA, no inner layout search, no final polish; the whole
  budget goes to CMA-ES over the board, one rebuild + full score per candidate. The web UI's
  Board tab runs this; Both runs bilevel or alternate.
- **Thumb keys** (`--thumb-keys N|auto`, I2): resize every thumb zone to N columns per side before
  the search (`geomsearch::resize_thumbs`); `auto` is `hand::thumb_slots` for the profile's hand
  (3 for the average hand: the 1 u keys that fit along the thumb's 73 mm operational arc, G06).
  Shrinking keeps columns with a fill the corpus types first, then the cheapest by G01 + G06;
  growing clones the cheaper end column outward as empty 1 u keys. Kept keys stay in place, so
  the thumb pose parameters (`thumb`, `thumb3d`) start from the old cluster. Pin `{none}` to keep
  letters off the empty keys: unpinned, the layout search fills them with letters (thumb letters).
  Count is not searched by score: with the empty keys pinned every count scores the same (Results).
- **Collisions**: 2D on flat candidates, 3D keycap + switch + finger access volumes (`collisions_3d`,
  GJK) otherwise, with `--margin` clearance.
  Objective = score + `penalty` · (|start board score| + 1) · total depth, the start board
  scored after its first layout search (not the raw start layout, e.g. QWERTY); past `hard_fail` (2 u) the
  layout search is skipped. The best collision-free board is reported. A profile `[build]`
  section (G07, [../geometry.md](../geometry.md#manufacturability-g07)) adds its violations to
  `collisions` and their excess to the depth, so build limits share the penalty and hard fail;
  without one, 3D candidates get the default limits (key angle ≤ 60°, height ≤ 4 u, …).
- **Budget**: with `--time`, 10% for the first layout run, 70% for the search, 10% each for the
  final SA on the best board and on the start board, so the reported gain compares equal
  layout effort. With `--evals N`: N boards, `--polish-evals` swaps per layout run;
  reproducible for a fixed thread count (the layout runs split swaps over threads, as in
  `gecko optimize`).
- **Run directory** `runs/<stamp>-geom-<hash>/`: `config.json` (`kind: "geometry"`, params,
  free variables, input hashes), `start.param.toml`, `progress.jsonl` (one line per generation),
  `best.json` (start and best board: values, score, collisions, layout, breakdown) and the best
  board as `geometry.param.toml` and `geometry.toml` (keys, loads with `-g` anywhere, or by name
  as `-g run:<id>`). The best board is named `<start>-opt-<run hash>`. If no collision-free board
  was found, `geometry-repaired.toml` holds its keys pushed apart (`Geometry::repair`, I3); it is
  no longer a parametric board. Each progress line carries
  the reported board (`values`, its layout `chars`, `collisions`), so the web UI can replay the
  run. `gecko list runs` lists geometry runs (algo `geometry`); `gecko verify` re-runs an
  `--evals` run from `start.param.toml` and checks input hashes, candidates, and start and best
  board (values, layout, score).
- **Pause** (`Control`): the search waits at the next generation and paused time does not count
  against the budget or toward `elapsed_ms`. Restarts left without budget are not listed in
  `cma_runs`.

## Free-form keys (I1)

`gecko geom optimize -g run:<id> --free keys -p hand` (`geomsearch::freeform`): every key's pose
is a variable instead of the board parameters, starting from a parametric board, usually a
geometry run's best (`-g run:<id>` loads its `geometry.param.toml`).

- **Variables**: `keys` (x, y, z, rx, ry, rz per key), `keys2d` (x, y, rz) or `keys.<c>`, named
  `key.<i>.<c>`; default range ±0.5 u / ±15° around each key's start, `=lo:hi` relative to it
  (`keys,keys.z=-0.2:0.2`). They cannot be mixed with board parameters. On a mirror-symmetric
  split only the left half is free and the right half mirrors it (x reflected about the split,
  ry and rz negated), so Corne has 108 variables, not 216.
- **Smoothness**: objective += `smooth` · (|start score| + 1) · roughness, where roughness sums,
  over neighbouring keys (same hand, both thumb or both finger keys, row/col labels one apart),
  the squared difference of their moves from the start board (u, radians). `--smooth 0.05`
  default; it penalizes bending relative to the start, not the start's own shape.
- **Repair**: a colliding candidate gets `Geometry::repair(margin, repair_passes)` (default 20
  passes, I3) and is scored at the repaired point when that collides less; the repaired point
  is what CMA-ES's incumbent and the reported best carry (not fed back into the CMA-ES update).
  Remaining depth takes the usual penalty.
- Everything else (bilevel/alternate/board modes, budget split, `[build]` limits, efforts
  derived per candidate, `verify`) is as above. The run writes `geometry.toml` only: the board
  has no parameters. `best.json` carries `rough`; the CLI summarizes moves per component.

## CMA-ES (`gecko_core::cmaes`, G5)

(μ/μ_w, λ)-CMA-ES after Hansen's tutorial (arXiv:1604.00772): positive recombination weights,
cumulative step-size adaptation, rank-one + rank-μ covariance update, `h_σ` stall, eigen
decomposition (cyclic Jacobi) every ⌊1 / (10 n (c1 + cμ))⌋ generations. Own implementation on
`rng::Rng` (Box–Muller normals), so a seed reproduces the run on native and WASM.

- `minimize(f, x0, bounds, params, rng)` or `minimize_batch(eval, …)`, where `eval` scores a whole
  generation (for parallel inner layout searches). Returns the best in-box point, value,
  evaluations, per-run info (λ, generations, stop reason) and a best-so-far history.
- **Bounds:** with `Bounds { lo, hi }` the search runs in the unit box and `sigma0` is a fraction
  of each range (default 0.3). The objective only ever sees in-box points: a sample outside is
  scored at its clamped point plus `(1 + max |f| of the generation) · ‖u − clamp(u)‖²`, while the
  update uses the sample itself (no Lamarckian repair, so C is not biased toward the faces).
- **Stops per run:** budget, `target`, TolFun (best of the last 10 + ⌈30n/λ⌉ generations and the
  current population within `tol_fun`), TolX (every coordinate step below `tol_x · sigma0`),
  condition number > 1e14, stagnation (median of the last 20 % of generations no better than
  the 20 % before, after 120 + 30n/λ generations).
- **IPOP restarts:** up to `restarts` more runs, λ doubling each time (default first λ =
  4 + ⌊3 ln n⌋), starting from a uniform point in the bounds (from `x0` without bounds), until
  `max_evals` or `target`.

Parameters: `sigma0` 0.3, `popsize` default, `max_evals` 10 000, `restarts` 9, `tol_fun` 1e-12,
`tol_x` 1e-11, `target` −∞.

**Pitfalls:** inner-loop noise (short SA) misleads the outer search; use common random
seeds and warm starts. The hand model must be validated before results mean anything: the
search pushes every unpenalized direction to its bound (see Results), so bounds are part of the
experiment.

## Results

CMA-ES on test functions (`tests/cmaes.rs`, seeds 1–5, 2026-10-21): sphere 10-D from 3 to
f ≤ 1e-10 in 1680–1910 evaluations (σ0 1); Rosenbrock 10-D from 0 to f ≤ 1e-8 in 5660–6120
(one run, σ0 0.5); Rastrigin 5-D in [−5.12, 5.12] from 3: a single default-λ run ends in a local
optimum (f 2.98–6.96), IPOP reaches f ≤ 1e-8 in 3.8k–24k evaluations (2–5 runs, final λ 16–128).
These match published CMA-ES figures (≈ 1.8k and ≈ 6k for sphere / Rosenbrock 10-D).

Board + layout search (`gecko geom optimize -g corne-36.param -p hand -c english`, QWERTY start,
20 threads, 2026-10-21). Score = after the equal final SA polish; "hand" = G01 + G03 + G05 part.

| run | free | budget | boards | start → best | hand / other | notes |
|---|---|---|---|---|---|---|
| `…034058-geom-263d9e6b` | 2d, seed 1 | 60 s | 6.8k | 11.63 → 10.53 (−9.4%) | −0.69 / −0.41 | thumbs inward ≈ 1 u, mild splay |
| `…034158-geom-ee9166d5` | 2d, seed 2 | 60 s | 6.8k | 11.68 → 10.56 (−9.6%) | −0.69 / −0.42 | |
| `…034259-geom-de24db86` | 2d, alternate | 60 s | 45k | 11.70 → 10.46 (−10.6%) | −0.74 / −0.50 | cheap board steps win in 2D |
| `…033829-geom-10c6ce3e` | 3d | 120 s | 14k | 11.56 → 7.24 (−37.4%) | −3.84 / −0.48 | row curve 30° (bound), tent 36°, tilt −12° |
| `…034605-geom-2239a9cb` | 3d, seed 2 | 120 s | 14k | 11.64 → 7.29 (−37.4%) | −3.71 / −0.64 | row curve 30° (bound), tent 40°, tilt −13° |
| `…034405-geom-3599b4d0` | 3d, row curve ≤ 20°, tent ≤ 30°, tilt −5–0° | 120 s | 14k | 11.63 → 7.27 (−37.5%) | −3.56 / −0.81 | col curve 15° (bound), tent 26° |

- 2D gains are ~10%, split between hand model and layout metrics (B14, S04); seeds agree to 0.3%.
- 3D gains come from the hand model: G05 press angle (−1.7) and G03 wrist posture (1.67 → ≈ 0)
  dominate, G01 reach −0.5. Every 3D run tents 26–40° and tilts negative, matching the posture
  literature's direction; curvature runs to its bound, but row and column curvature trade off
  (Dactyl-like bounds reach the same score), so the optimum is a plateau, not one shape.
- Caveat: the hand model has no finger-vs-neighbouring-keycap clearance and G03's optimum
  (≈ 58° effective pronation relief) is uncalibrated, so 3D shapes are model-optimal, not
  validated. Use bounds as priors (H6 #59, H10 #69).

**3D bilevel search (H10, 2026-10-22)**: same command, `--free 3d` (34 parameters, with the split
opening), H8 GJK volumes and the H6 posture priors (tilt −4–0°, opening 10–20°), 120 s.

| run | seed | boards | start → best | G01 / G03 / G05 (start → best) | shape |
|---|---|---|---|---|---|
| `20261022-174027-geom-10c6ce3e` | 1 | 5.9k | 11.66 → 7.48 (−35.9%) | 1.26 → 0.59 / 1.67 → 0.02 / 1.84 → 0.40 | row curve 30° (bound), col 0°, tent 38°, tilt −4° (bound), opening 13.5° |
| `20261022-174228-geom-2239a9cb` | 2 | 5.8k | 11.66 → 7.41 (−36.4%) | 1.17 → 0.61 / 1.67 → 0.02 / 2.10 → 0.33 | row curve 30°, col 15° (both bounds), tent 32°, tilt −4°, opening 13.8° |

Comparison, each board with the same 12 s SA from QWERTY (`gecko optimize -p hand -c english`):

| board | score | G01 | G03 | G05 | run |
|---|---|---|---|---|---|
| Corne, flat (search start, derived efforts) | 11.66 | 1.2 | 1.67 | 1.8–2.1 | above |
| Corne keys, Dactyl shape (α 15°, β 5°, row pitch 18.4 mm, tent 15°, derived efforts) | 10.16 | 1.11 | 0.75 | 1.53 | `…174520-sa-0d99155b` |
| `dactyl-manuform-5x6` (its own efforts, more keys) | 9.90 | 1.00 | 1.04 | 1.07 | `…174537-sa-dfd7ec48` |
| H10 best (seed 2) | 7.52 | 0.60 | 0.02 | 0.33 | `…174550-sa-e7c7a720` |

- The posture priors cost little: 7.41–7.48 against 7.24–7.29 with tilt free to −15° (G6 runs
  above, with 2.4× the boards). G03 is ≈ 0 either way, so the extra tilt was not needed for the
  wrist; the difference is in G01/G05.
- The search beats a Dactyl shape on the same 36 keys by 26%, mostly hand model: deeper row
  curvature (30° vs 15°) cuts G05 press angle by ~4/5, more tent (32–38° vs 15°) clears G03.
  Both are far past what the Dactyl builds, and row curvature still sits on its bound, so this
  is the uncalibrated hand model talking (no finger-vs-keycap clearance, G03 optimum), not a
  validated design.
- Seeds again agree on the score (0.9%) but not on the shape: row vs column curvature and the
  thumb tent (39° vs 15°) trade off on a plateau.
- Throughput fell from ~14k to ~5.9k boards per 120 s while another agent shared the machine;
  3D collision costs 0.24 ms per 36-key board against ~0.3 s of layout search per candidate,
  so it is not the bottleneck.

**3D search under G07 limits, then STL (J4, 2026-10-22)**: `--free 3d`, 90 s, seed 3, `hand`
profile plus a default `[build]` section (key angle ≤ 60°): `20261022-184243-geom-19d9c66f`,
11.56 → 7.57 (−34.5%), within every build limit (split tent 36°, tilt −4°, opening 10°, row
curve 30° at its bound). `gecko build stl run:<id>` gives a 215 × 114 × 77 mm case with every
switch cutout clear (12 connectors dropped) but 8 keycaps hitting webs within their travel;
the H10 runs without `[build]` (tent to 104°) leave 2–24 cutouts blocked. A buildable board
needs `[build]` during the search, and keycap clearance is not yet a search constraint.

**Invalid 3D boards fixed (2026-10-22).** 3D runs gave insane tents, vertical thumb clusters and keys
buried under neighbours, with any profile (`20261022-230957-geom-51902824`: board mode, `default`,
`code`: B14 stretch 2.84 → 0.34 of a 3.2 gain; split tent 5°, thumbs tent 40° / tilt 34°, row and
column curve at their bounds, index columns 1.5 u below the middle). Three causes:
1. B14/S04/S05/T09 stretch measured only in-plane x, y in the hand frame, so sinking, lifting and
   curving columns hid stretch for free (and derived effort measured 2D). Now 3D distance.
2. 3D collision tested solids only: a key under a neighbour's cap with vertical room clears it,
   though no finger can press it. Now each key has a finger access volume (see geometry.md).
3. Without a profile `[build]` nothing bounded key angle or height. Defaults now apply to 3D
   candidates.

Same command after the fix (`20261022-232354-geom-f7fdd84b`, 60 s): 28.47 → 25.87 (−9.1%), no
collisions, split tent 0°, row curve 0°, all keys accessible. Column curve still runs to its 15°
bound and the index stretch column reaches the 60° key-angle limit: tilted caps nest closer
(tops 0.88 u apart), which shortens true 3D fingertip distances. Nothing in a distance-only profile
prices pressing a steep key (that is G05), so use `-p hand` or tighter `col_curve` bounds for
3D shape. With `-p hand`, 3D bilevel 90 s (`20261022-232628-geom-0a6a57dd`): 11.63 → 7.70
(−33.8%), split tent 28°, row curve 30° (bound), every key ≤ 50°, thumbs ≈ 26°. Earlier 3D
results above predate the fix (3D boards now score their out-of-plane stretch), so their
numbers are not comparable.

**Build guide (#77, 2026-10-23)**: `corne-36.param`, `--free '3d,fingers.row_curve=0:20,split.tent=0:30'`,
thumb keys pinned, `-p data/examples/my-hand.toml` (a 205 mm hand), `--evals 4000 --seed 1 --threads 8`
([../build-guide.md](../build-guide.md)).

| run | case check | start → best | cap hits | shape |
|---|---|---|---|---|
| `20261023-120734-geom-6864b5ad` | off, thumbs free, 800 boards | 5.61 → 2.90 | 0 | layout put space/enter/bspc on letter keys: unusable |
| `20261023-120749-geom-0306cb88` | off | 10.47 → 6.60 (−37.0%) | 8 (inner index columns, 2 thumbs) | row curve 20° (bound), tent 30° (bound) |
| `20261023-120955-geom-c671f1cc` | off, column lift fixed | 10.47 → 6.63 | 4 thumbs, 1 blocked switch | |
| `20261023-120955-geom-a9081ea6` | off, `--margin 0.1` | 10.47 → 8.45 | | no collision-free board: the start Corne collides at 0.1 u |
| `20261023-121212-geom-e91bdfbb` | `mx` | 10.47 → 6.62 (−36.8%) | 0 | row curve 20° (bound), col 6.8°, tent 27°, thumbs tilt −21° |

- The case check costs 0.2% of score and ~30% more time (41 s vs 31 s); the unchecked board is
  not printable as is, so the guide turns it on.
- Pin the thumb tokens: an unpinned 3D search moves them onto letter keys (G01 favours the
  home row for frequent keys, and space is the most frequent).
- The `20261022-174*` H10 runs above no longer load as boards (keycap overlap under the
  current 2D/3D check); their numbers stand, their `run:<id>` boards do not.

**Thumb cluster (I2, #71, 2026-10-23)**: `corne-36.param`, `-p hand -c english`, `--free
'3d,fingers.row_curve=0:20,split.tent=0:30'`, `--evals 2000 --seed 1 --threads 5–6`, thumb tokens
pinned. "G06" is the best board's G06 per keystroke under `hand` (G06 weight 1.0).

| run | thumb keys / side | pins | start → best | G06 | thumbs: rotate, tent, tilt, col curve |
|---|---|---|---|---|---|
| `20261023-152222-geom-1e909629` | 3, G06 weight 0 | tokens | 10.90 → 7.36 | 0.155 | 8°, −24°, −12°, 5° |
| `20261023-152222-geom-467179ae` | 3 | tokens | 10.96 → 7.43 | 0.034 | 12°, −3°, −21°, 14° |
| `20261023-152222-geom-7ac0d517` | 2 | tokens | 10.96 → 7.49 | | 10°, −2°, −27°, 20° |
| `20261023-152222-geom-eb9527e1` | 4 | tokens | 10.15 → 6.61 | | 16°, 4°, −14°, 6° |
| `20261023-152424-geom-e4eae0c0` | 2 | tokens + `{none}` | 12.31 → 9.00 | | 5°, −12°, −11°, 11° |
| `20261023-152424-geom-b6b0e0d2` | 3 | tokens + `{none}` | 12.31 → 8.86 | 0.012 | 8°, −9°, −20°, 17° |
| `20261023-152424-geom-fa2d972c` | 4 | tokens + `{none}` | 12.31 → 9.00 | | 6°, 10°, −26°, 2° |

- G06 does what it is for: weighted, the best board's thumb keys sit 4.5× closer to the arc
  (0.155 → 0.034) for +0.10 G01, +0.08 G03 and −0.11 G05 (weighted), mostly by curving the
  cluster (col curve 5° → 14°) instead of tenting it (−24° → −3°).
- Without `{none}` pinned the layout search puts letters on empty thumb keys: Corne's right
  outer key gets `s`, and the 4-key board's three empty keys get `x`, `r`, `l` (score 6.61).
  The model charges a thumb press like any cheap key; the thumb's higher press force and the
  learning cost of thumb letters are not in the score, so this is a profile question, not a
  board one.
- With `{none}` pinned, every count starts at the same score (12.31) and ends within 0.14 of
  the others (one seed): empty keys carry no keystrokes, so the score cannot choose the count.
  Hence `--thumb-keys auto` sizes the cluster from the arc's capacity instead.

**Free-form keys (I1, #70, 2026-10-23)**: from the post-fix 3D `-p hand` best
(`run:20261022-232628-geom-0a6a57dd`), `-p hand -c english --evals 1500 --seed 1 --threads 5`, thumb
tokens pinned, four runs in parallel (≈ 75 s each). Score after the equal final polish; "other" =
everything but G01/G03/G05/G06.

| run | free | start → best | G01 / G05 | other | roughness | largest moves |
|---|---|---|---|---|---|---|
| `20261023-165958-geom-e3882601` | `3d` (parametric, continued) | 7.33 → 7.28 (−0.7%) | 0.76 → 0.72 / 0.43 → 0.48 | 6.04 → 5.84 | | |
| `20261023-165958-geom-9904e465` | `keys2d` | 7.33 → 6.54 (−10.9%) | 0.76 → 0.67 / 0.43 → 0.27 | 6.04 → 5.51 | 0.95 | 0.29 u, 8° |
| `20261023-165958-geom-4759ead3` | `keys` | 7.33 → 6.05 (−17.5%) | 0.76 → 0.54 / 0.43 → 0.12 | 6.04 → 5.33 | 2.07 | 0.34 u, 9.5° |
| `20261023-165958-geom-64cd2623` | `keys`, `--smooth 0` | 7.33 → 5.32 (−27.5%) | 0.76 → 0.49 / 0.43 → 0.15 | 6.04 → 4.63 | 10.45 | 0.48 u, 14° |

- The parametric optimum is a local one for the parameter space only: the same budget gains
  0.7% more in it and 17.5% with free keys, under the same hand model and collision checks.
  Of the 1.29 gain, 0.53 is hand model (G05 press angle −0.31, G01 reach −0.22) and 0.71 is
  the other metrics, which the moved keys and the new layout share.
- Smoothness trades score for regularity: without it the search gains another 10 percentage
  points but the board is 5× rougher, with neighbouring keys moved up to 14° apart, which is
  where the uncalibrated model is least trustworthy (no finger-vs-neighbouring-cap clearance
  beyond the access volumes). No move reached its bound in any run, so the ranges did not bind.
- `gecko verify` reproduces the `keys2d` run (1486 boards, start and best). The boards load
  as `-g run:<id>`. The smoke run without pins (`20261023-165806-geom-982dab2d`) put space and
  enter on letter keys, as in the parametric case: pin the thumb tokens.
