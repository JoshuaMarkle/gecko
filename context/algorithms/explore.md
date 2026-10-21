# Breakout local search and exploration

Spec and background: `research/search-space-visualization.md` (archive once, re-weight forever).

## BLS (`--algo bls`, `optimize/bls.rs`)

**Idea (Benlic & Hao):** steepest descent to a local optimum, then *kick* it with `k` random
swaps (or, with probability `column_kick`, exchange two whole columns) and descend again.
`k` resets to `kick_min` after an improvement or a move and grows by one after each failed
kick. A kick result that is worse is accepted with probability `accept_worse`. A worker
restarts from a random layout after `stagnation` kicks without improving its best since the
last restart, or when `k` passes `kick_max`.

Every kick is an edge between two local optima, so with an `Observer` in `Ctx` BLS samples
the local optima network (LON) for free.

**Parameters (`Params.bls`):** `kick_min` 2, `kick_max` 0 (auto = a third of the movable
symbols), `accept_worse` 0.1, `stagnation` 150, `column_kick` 0.2, `exclusion` 0.5,
`exclusion_dist` 8 (the last two only in explore mode).

## Explore (`gecko explore`, `--algo explore`, `explore/`)

One run maps the landscape instead of returning one winner:

1. **Probe** (`probe.rs`): `samples` random layouts (score + raw metric vector) and `walks`
   random walks of `walk_len` swaps → random-score distribution, autocorrelation and
   correlation length. Written to `samples.bin`.
2. **BLS in explore mode** for the rest of the budget, with the `Archive` observer
   (`archive.rs`) recording every local optimum (score, raw metric vector, hits, first-found
   time) and every kick edge. Optima are canonicalized under left/right mirroring only when
   the scores are provably mirror-symmetric and nothing is pinned.
3. **Exclusion restarts:** a share `exclusion` of restarts begin next to a random entry of
   the exclusion set (up to 16 greedy-distinct best optima plus their mirrors), apply random
   swaps until ≥ `exclusion_dist` slots from every entry, then descend under that Hamming
   constraint. The constrained end point is archived with flag `CONSTRAINED`, then a free
   steepest descent follows. This finds the best layouts that are *different* from the ones
   already known instead of falling back into the same funnel.
4. **Snapshots** every `snapshot_ms` (atomic writes), so the UI can watch a live atlas.

**Parameters (`Params.explore`):** `samples` 2000, `walks` 64, `walk_len` 400,
`max_optima` 100 000 (worst pruned past 1.2×), `snapshot_ms` 2000, `within` 0.01 (the
"near the best" band for the summary).

**Analysis (`stats.rs`, shared by CLI and WASM):** re-scoring from raw vectors under any
weights (exact, incl. squared and hand terms), FDC, PCA (power iteration on ≤ 3000 strided
rows), consensus and per-slot entropy, families (leader clustering at radius `cut`),
funnels (follow each optimum's best improving edge to a sink), Chao1 richness estimate,
weight stability (how far each weight can scale before the best changes).

**Pitfalls:** archive scores are recomputed from scratch, not from the worker's running
delta score, so the atlas never carries drift. Constrained end points are not local optima;
filter on the flag when a statistic assumes they are. PCA axes explain little variance
(~10 %) on 30-key boards; read the map for clusters, not distances.

## Results

ansi-30, default profile, `english`, 20 threads (2026-10-20):

- Landscape: random layouts 91.6 ± 17.8 (min 45.7); correlation length ≈ 9 swaps; scores
  are not mirror-symmetric. Canary 23.48, Colemak-DH 29.15, QWERTY 66.02 (5 % of random
  layouts beat QWERTY).
- 20 s explore: 84–108 M evals, 8–11 k distinct optima, 22–28 k descents, FDC 0.18–0.23
  (weak global structure), 6–7.7 k funnels; only 3–4 optima within 1 % of the best. Best
  15.7037 `jkndfzquo, rthcpxseai lmbwvyg/;.` — the same layout SA and BLS find in 20 s
  (`bench sa,bls` 10 s × 3: both 15.7037).
- Exclusion restarts produce new distinct entries at d = 8 and d = 12; a near-mirror family
  sits at d = 30. Re-ranking the same atlas under the `sfb` profile gives entirely different
  winners, so one atlas answers many weightings.
- Runs: `runs/20261020-203032-explore-ea34e8fa`, `20261020-203446-explore-e0b34b3c`,
  `20261020-204749-explore-863c1309` (10 s from the UI).
- 120 s, 8 threads, seed 2, `'` in place of `;` (`20261020-210913-explore-f5db8e69`): same
  best (15.9089, the `'` variant), found at 4.7 s and unbeaten for 456 M evals. 41 k optima,
  Chao1 ≈ 151 k (the tail is far from enumerated), but no singletons within 2 % of the best
  and the top 3 have 360–610 hits each: the top band is saturated. The top is a plateau:
  #2 is 0.13 % worse at d = 15, 12 optima sit within 1 %, and only 13/30 slots agree across
  them (`n d h`, `u o ,`, `e a i`, `b`, `/ ' .` fixed; the rest of the consonants vary).
