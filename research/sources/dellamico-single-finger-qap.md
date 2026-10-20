# Dell'Amico, Díaz Díaz, Iori, Montanari — "The single-finger keyboard layout problem" (C&OR 2009)

doi:10.1016/j.cor.2009.01.018. Preprint: https://iris.unimo.it/retrieve/e31e124b-fcc2-987f-e053-3705fe0a095a/dellamico_r01.pdf
Benchmarks (24 instances, English/French/Italian/Spanish + random) and best solutions were
published at www.or.unimore.it.

## Key ideas

- Single-finger keyboard design is a **generalized QAP**: flow `a_ik` = symmetric digraph
  frequency, distance `b_jl` = Fitts time `α + β·log2(D/W + 1)` with α = 0, β = 10/49.
  Extra freedom: more locations than symbols (empty cells, "contour filling").
- Delta formulas: pair exchange `Δ = Σ_{k≠r,s} (a_kr − a_ks)(b_φ(s)φ(k) − b_φ(r)φ(k))`, O(n);
  moving a symbol to an empty cell O(n), and O(1) for consecutive moves with a cached Δ.
- Methods compared: SA, tabu search, VNS, **FANT (fast ant system)**, local search refine.
  FANT (reinforcement R = 8) found the best solution on all 24 instances; TS and SA on 22;
  VNS on 21.
- Speed-ups: canonical form (shrink empty rows/cols, symmetries) + hashing of visited
  solutions as long-term memory.

## How to use in GECKO

- Confirms GECKO's delta approach and gives the classic Δ-matrix update for tabu
  (`../../context/algorithms/tabu-search.md`).
- **Empty slots** (more keys than symbols) need a "move to empty" neighbourhood, needed once
  geometries have spare keys (thumbs, extra columns).
- **Symmetry canonicalization** (mirror halves) cuts duplicate optima in result sets.
- FANT is cheap to add as an experiment (pheromone matrix symbol×slot, sample + local search).
