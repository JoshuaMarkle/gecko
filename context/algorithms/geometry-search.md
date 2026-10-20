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

**Pitfalls:** inner-loop noise (short SA) misleads the outer search; use common random
seeds and warm starts. The hand model must be validated before results mean anything.

## Results

_None yet._
