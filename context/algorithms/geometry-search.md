# Geometry search (continuous, phases 6–8)

Background: `research/hand-model-and-geometry.md`.

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
explicit boards exactly. Building blocks in place: `Scorer::set_geometry` (fast table rebuild,
[../objective.md](../objective.md#geometry-rebuild-g2)), `Geometry::collisions` (penalty by depth,
[../geometry.md](../geometry.md#collision-g04)), derived efforts that follow moved keys, and CMA-ES
below. The bilevel driver (G6, #54) is next.

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
seeds and warm starts. The hand model must be validated before results mean anything.

## Results

CMA-ES on test functions (`tests/cmaes.rs`, seeds 1–5, 2026-10-21): sphere 10-D from 3 to
f ≤ 1e-10 in 1680–1910 evaluations (σ0 1); Rosenbrock 10-D from 0 to f ≤ 1e-8 in 5660–6120
(one run, σ0 0.5); Rastrigin 5-D in [−5.12, 5.12] from 3: a single default-λ run ends in a local
optimum (f 2.98–6.96), IPOP reaches f ≤ 1e-8 in 3.8k–24k evaluations (2–5 runs, final λ 16–128).
These match published CMA-ES figures (≈ 1.8k and ≈ 6k for sphere / Rosenbrock 10-D).
