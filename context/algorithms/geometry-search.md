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
- **Collisions**: 2D on flat candidates, `collisions_3d` otherwise, with `--margin` clearance.
  Objective = score + `penalty` · (|start board score| + 1) · total depth, the start board
  scored after its first layout search (not the raw start layout, e.g. QWERTY); past `hard_fail` (2 u) the
  layout search is skipped. The best collision-free board is reported.
- **Budget**: with `--time`, 10% for the first layout run, 70% for the search, 10% each for the
  final SA on the best board and on the start board, so the reported gain compares equal
  layout effort. With `--evals N`: N boards, `--polish-evals` swaps per layout run;
  reproducible for a fixed thread count (the layout runs split swaps over threads, as in
  `gecko optimize`).
- **Run directory** `runs/<stamp>-geom-<hash>/`: `config.json` (`kind: "geometry"`, params,
  free variables, input hashes), `start.param.toml`, `progress.jsonl` (one line per generation),
  `best.json` (start and best board: values, score, collisions, layout, breakdown) and the best
  board as `geometry.param.toml` and `geometry.toml` (keys, loads with `-g` anywhere, or by name
  as `-g run:<id>`). The best board is named `<start>-opt-<run hash>`. Each progress line carries
  the reported board (`values`, its layout `chars`, `collisions`), so the web UI can replay the
  run. `gecko list runs` lists geometry runs (algo `geometry`); `gecko verify` re-runs an
  `--evals` run from `start.param.toml` and checks input hashes, candidates, and start and best
  board (values, layout, score).
- **Pause** (`Control`): the search waits at the next generation and paused time does not count
  against the budget or toward `elapsed_ms`. Restarts left without budget are not listed in
  `cma_runs`.

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
