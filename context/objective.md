# Objective

Full metric list with IDs: `research/metrics-catalog.md`. Implemented now (`gecko list metrics`):
B01 SFB, S01 SFS, S02 far SFS (gaps 2–3, decayed), S03 SFS distance, B03 finger speed,
B04 LSB, B05 scissors, B14 geometric stretch (Oxeylyzer 2), M01 effort, M02 finger-load error,
M04 hand balance, M05 home row, M06 center columns, B08 alternation, T01–T08 trigram classes,
E04/E05 Engram key-pair discomfort (formula / measured; geometry `[comfort]` tables),
B02 repeats (info), Y01 layer switches, Y02 code-symbol bigrams. Profile `[sfb]` sets B03/S03's distance transform (linear, squared, fitts),
lateral multiplier and per-finger factors, and S02's `skip_decay`.

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
≈ 570k swap deltas/s with the default profile (trigrams and far skipgrams on). The far-skipgram
channel is compiled out of the delta when no metric uses it (~10% faster).

Next speed tricks, in order of expected payoff: cache per-trigram current cost in the state;
pre-multiplying frequencies into cost tables where the layout term allows it; SIMD.

## Normalization and profiles

- Frequencies are normalized over the symbols on the layout: values read as "% of the
  keystrokes / bigrams / skipgrams / trigrams typed on this layout".
- Raw metric values are in natural units (%, average effort). A profile's optional `[scale]`
  divides each weight (`weight / scale`); `gecko profile normalize` sets scale = the metric's
  stdev over the reference layouts and rewrites weights so the objective is unchanged. The
  default profile is in these spread units: weight 1 = "one reference spread".
- `gecko profile fit --target E05` fits non-negative spread-unit weights so the other metrics
  predict a judge metric over layouts sampled around the references (reports R²).
- `trigram_coverage` (default 0.99) keeps the most frequent trigrams covering that share
  and renormalizes; scoring, deltas and reports all use the same truncated table, so they
  stay consistent. About 2× faster search than exact (1.0).
- Profiles are TOML in `data/profiles/` (`default`, `sfb`, `engram` = independent judge, E05 only):
  weights, soft targets, constraints. Profiles are versioned; runs record the hash.

## Calibration

The default profile must rank known layouts sensibly (modern layouts > Colemak-DH and
Dvorak > QWERTY; Colemak-DH vs Dvorak is left open, see decisions.md) and reproduce published SFB percentages within tolerance. A
regression test pins these rankings (`ranking_sanity_on_english`). On the bundled corpus
QWERTY SFB = 6.63% (published ≈ 6.6%). Cross-checks: `gecko compare -p engram` (measured
comfort) and `gecko compare --corpora english,shai,mt-10k,mt-450k` (robustness, L05).

**Community likes (cmini, `gecko layouts likes`).** 4121 cmini layouts → 1965 importable
(30-key core, all letters, standard or angle fingering) → 1904 after merging 55 duplicates and
6 mirrors; 303 have at least one like. Spearman ρ of GECKO goodness (−score) vs likes, ansi-30:

| profile · corpus | all (1904) | liked only (303) | ortho (1504) | angle (358) | stagger (40) |
|---|---|---|---|---|---|
| default · english | +0.039 | +0.105 | +0.059 | +0.005 | −0.310 |
| default · shai | +0.039 | +0.105 | +0.064 | −0.007 | −0.310 |
| engram · english | +0.050 | +0.036 | +0.056 | +0.062 | −0.321 |
| default · english on ortho-30 | +0.022 | +0.098 | +0.047 | −0.030 | −0.284 |

Liked layouts sit at a mean GECKO rank percentile of 47.7 % vs 50.4 % for unliked ones.
Reading: likes barely track layout quality. They measure fame and age (Graphite 77 likes, rank
18; the joke layout `inqwerted` 76 likes, rank 1818; Dvorak and Colemak-DH ~20 likes, rank
~1460), and 84 % of layouts have zero likes (ties). The weak positive ρ among liked layouts is
the usable signal; `cmini_likes_ranking` pins it (ρ > 0.05, Graphite and Gallium in the top
10 % of liked layouts). Angle layouts are scored with standard fingering until angle-mod
geometry (#47), which hurts them (Canary: rank 1021). The stagger group is small (40, 9 liked) and
noisy. Per-metric ρ vs likes is printed too (all |ρ| ≤ 0.07). Use these
layouts for C7 as preference pairs between liked layouts, not as raw like counts.

## Multi-objective

Besides the weighted sum: rank-sum selection over many results (Kühlmak) and Pareto
fronts (NSGA-II). Reports always show the per-metric breakdown.

## Layers (Y01, Y02)

- **Y01** (bigram table on the layered geometry): 100 when the second symbol is on a
  non-base layer the first isn't on (one layer-key press, hold model), 200 when the first key
  is on the layer key's finger (same-finger into the layer key). Reads "presses per 100
  bigrams". Releases are free.
- **Y02** (own channel `fy`/`Cy`, `Order::SymBi`): frequencies are only bigrams of two ASCII
  punctuation symbols, normalized on their own; cost 100 when they are on one finger (two keys)
  or on different layers. Reads "% of symbol bigrams that are awkward". Adds one term to the
  pair delta only when weighted.
- Not modeled yet: the layer key's own keystroke in effort/finger load, one-shot layers,
  thumb-trigram classes. Profile `code` weights both (scales are rough estimates).
