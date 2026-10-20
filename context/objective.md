# Objective

Full metric list with IDs: `research/metrics-catalog.md`. Implemented now (`gecko list metrics`):
B01 SFB, S01 SFS, B03 finger travel, B04 LSB, B05 scissors, M01 effort, M02 finger-load error,
M04 hand balance, M05 home row, M06 center columns, B08 alternation, T01–T08 trigram classes,
B02 repeats (info).

## Metric contract

Metric definitions live in `gecko-core/src/metrics/` (one table builder per metric) and provide:

- `id` (catalog ID, e.g. `B01`), name, direction, n-gram order(s) it needs.
- `cost tables`: its contribution as per-slot (`[N]`), per-pair (`[N×N]`), or
  per-triple (`[N×N×N]`) costs given a geometry. Most metrics are only this.
- `raw value`: how to report it (%, per-1000) from a layout + corpus.
- Optional `nonlinear` hook for metrics that are not a sum over n-grams (balance RSS,
  soft targets, max finger use). These run on small accumulators (per-finger totals)
  that are also updated incrementally.

## Folding

At scorer build time, linear metrics fold into one table per order:

```
C1[s]       = Σ_m w_m · c1_m[s]
C2[s][t]    = Σ_m w_m · c2_m[s][t]     (bigrams; skipgrams get their own C2k)
C3[s][t][u] = Σ_m w_m · c3_m[s][t][u]
score = Σ_x f1[x]·C1[p(x)] + Σ_{x,y} f2[x][y]·C2[p(x)][p(y)] + … + nonlinear terms
```

Adding a linear metric costs nothing at search time.

## Delta evaluation

Swapping symbols a and b changes only terms that involve a or b:
bigrams O(S), trigrams O(S²). Nonlinear terms update from per-finger accumulators.
Every optimizer uses `delta_swap(layout, a, b)`; full evaluation is for verification
and reports. A property test enforces `full(after) − full(before) == delta`.

Trigrams are stored sparsely per symbol (CSR of non-zero trigrams containing it), so a swap
touches only trigrams with `a` or `b`. Current speed (`gecko perf`, i9-13900H, 1 thread):
≈ 400k swap deltas/s with the default profile (trigrams on), ≈ 3M/s without trigrams.

Next speed tricks, in order of expected payoff: cache per-trigram current cost in the state;
pre-multiplying frequencies into cost tables where the layout term allows it; SIMD.

## Normalization and profiles

- Frequencies are normalized over the symbols on the layout: values read as "% of the
  keystrokes / bigrams / skipgrams / trigrams typed on this layout".
- Raw metric values are in natural units (%, average effort). For now weights apply to
  these values directly (tuned by hand so the ranking test passes). Normalizing by spread
  across reference layouts is still planned.
- `trigram_coverage` (default 0.99) keeps the most frequent trigrams covering that share
  and renormalizes; scoring, deltas and reports all use the same truncated table, so they
  stay consistent. About 2× faster search than exact (1.0).
- Profiles are TOML in `data/profiles/` (`default`, `carpalx`, `low-sfb`, …):
  weights, soft targets, constraints. Profiles are versioned; runs record the hash.

## Calibration

The default profile must rank known layouts sensibly (modern layouts > Colemak >
Dvorak > QWERTY) and reproduce published SFB percentages within tolerance. A
regression test pins these rankings (`ranking_sanity_on_english`). On the bundled corpus
QWERTY SFB = 6.63% (published ≈ 6.6%).

## Multi-objective

Besides the weighted sum: rank-sum selection over many results (Kühlmak) and Pareto
fronts (NSGA-II). Reports always show the per-metric breakdown.
