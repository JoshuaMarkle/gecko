# Objective

Full metric list with IDs (and the old `B01`-style codes, still accepted on input):
`context/metrics-catalog.md`. Implemented now (`gecko list metrics`):
`sfb`, `sfs`, `sfs_far` (gaps 2–3, decayed), `skip_stretch` (`stretch` on skipgrams),
`lsb`, `scissor`, `half_scissor`, `bigram_inroll`, `pinky_ring`, `row_jump`
(each key measured from its finger's home key), `stretch` (Oxeylyzer 2), `movement`
(KLO finger pairs), `effort`, `finger_target`, `finger_evenness` (RSS), `finger_peak`,
`hand_balance`, `thumb_load`, `home_row`, `center_cols`, `hand_switch`, the trigram classes
`alternate`–`sft` and `thumb_roll`–`thumb_alt`, `contortion`, `irregular`, `typing_time`
(a judge, below), `engram_formula` / `engram` key-pair discomfort (geometry `[comfort]`
tables), `reach`, `press_angle`, `wrist` and `thumb_arc` (hand model, profile `[hand]`;
`context/hand-model.md`), `repeat` (info), `layer_switch` (and Shift), `symbol_bigram`,
`brackets`, `hrm_misfire`, and the placement metrics `shortcuts`, `familiar`, `vim`, `zones`.
Profile `[sfb]`'s `skip_decay` sets `sfs_far`. Profile `row_offset` (pinky ring middle index ×
[above, below] home) adds per-finger row costs to `effort`'s geometry effort.

**Retired** (2026-10-22): `sfb_distance` and `sfs_distance`. Keystroke timing shows no
same-finger distance effect beyond `sfb`. They still score, so old runs and profiles verify;
`gecko list metrics` and the API mark them `retired`, the web UI hides them, fits skip them,
and no bundled profile weights them. `[sfb]`'s `distance`, `lateral` and `finger_factor` only
shape them.

## Hand model (`reach`, `wrist`, `press_angle`)

Mono tables from `gecko_core::hand` (`context/hand-model.md`): each hand is placed on its home
keys and every key solved by finger IK (planar fingers, 4-DoF thumb). `reach` = joint deviation from
rest + splay + fingertip shortfall; `press_angle` = press misalignment; `wrist` = wrist deviation, extension
and pronation of the resting hand against the forearm (elbows from `[hand.body]`), plus the
deviation of sideways reaches. Layer slots add the holder's cost (like `effort`). Profile `[hand]`
(`size` or `dims`, optional `body`) sets the hand; without it `reach`/`wrist`/`press_angle` use the average hand and
arms and `stretch`'s relaxed spread stays 1.35 u (with it: × span / 200 mm). Bundled `default` weights
them 0; `hand` weights `reach` 0.427, `press_angle` 0.306 (1.0 and 0.5 on the old 10-layout scale basis, same
objective), `wrist` 1.0 and halves `effort`. Scales (stdev over the 9 non-QWERTY references, english /
ansi-30): `reach` 0.0847, `press_angle` 0.0365; `wrist` has none (its spread over layouts is ~0.002: a board
metric, weighted in natural units). Folded at construction; no hot-loop cost.

## `movement` finger-pair movement

Same-hand, different non-thumb fingers (`metrics::movement`):
`pair[from][to] · direction · (0.1 + rows² + lateral)`, reported per 100 bigrams.

- `pair`: KLO's `finger_switch_factor` (P→R 4, P→M 2, R→P 6, R→M 2, R→I 0.1, M→P 3, M→R 5,
  M→I 0.6, I→P 0.1, I→R 0.3, I→M 0.9; P→I 0). Ring pairs are scaled by Häger-Ross coupling
  `(2 − I_a − I_b)` over its mean across the six pairs: ×1.30 with pinky, ×1.35 with middle,
  ×0.96 with index.
- `direction`: 1 + 2·|Δlength| (lengths P1 R2 M3 I2) when a shorter finger moves down to a
  longer one or a longer finger up to a shorter one — the scissor where the short finger is high.
- `rows` = logical row change; `lateral` = column span beyond the fingers' own spacing (LSB;
  KLO ships this at 0).

Findings (english, ansi-30): `movement` spread over the references = 18.3. It agrees with `stretch` on the
optimum (20 s SA, seed 7: `movement` at 0, 0.5, 1 gives near-identical layouts, `stretch` 1.87–1.88, `engram`
5.22–5.27) but re-sorts references with ring-up scissors: Semimak (`bl`) 93.7 and Halmak (`ed`)
97.1 are above QWERTY (85.9); Sturdy 41.5 and Graphite 46.1 are the lowest. Weight 1 swaps
Halmak and Colemak; 0.5 keeps the reference order. Default 0.5 until the 2026-10-22 retune
halved it to 0.25 (0.2478 on the 9-layout basis, now `balanced`); the 2026-10-24 default cuts it
to 0.086. Measurements above use the 2026-10-21 profile.

## Metric contract

Metric definitions live in `gecko-core/src/metrics/` (one table builder per metric) and provide:

- `id` (catalog ID, e.g. `sfb`), name, direction, n-gram order(s) it needs.
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

Adding a linear metric to a channel that is already on costs nothing at search time; switching a
channel on does. Measured 2026-10-21 (one thread, `bench/software-paper-2026-10-21/metrics_scaling.py`):
swap deltas run at 4.8 M/s with bigram and gap-1 skipgram metrics only, 2.4 M/s once a far-skipgram
metric (`sfs_far`) is on, 2.1 M/s with the load terms (`finger_target`, `finger_evenness`, `finger_peak`), and 0.64 M/s once any trigram
metric is on; the rate is flat within each step.

## Delta evaluation

Swapping symbols a and b changes only terms that involve a or b:
bigrams O(S), trigrams O(S²). Nonlinear terms (`finger_target`, `finger_evenness`, `hand_balance`, `finger_peak`) update from per-finger accumulators.
Every optimizer uses `delta_swap(layout, a, b)`; full evaluation is for verification
and reports. A property test enforces `full(after) − full(before) == delta`.

Trigrams are stored sparsely per symbol (CSR of non-zero trigrams containing it), so a swap
touches only trigrams with `a` or `b`. Each `State` caches the current folded cost of every
nonzero trigram; deltas read the cached old cost, and `apply` refreshes only touched entries
(including rollback swaps). `full` independently recomputes every term for verification.
The cache is empty when no trigram metric is weighted; evaluating/applying swaps allocates
nothing. Sparse entries retain stable cache IDs, avoiding an extra indirect trigram lookup.

Measured 2026-10-21 on the i9-13900H, ansi-30 / english / default, alternating five
`taskset -c 2 gecko perf --json` samples per binary: baseline median 318,249 deltas/s,
cached median 370,267 deltas/s (**+16.3%**). Individual runs vary under concurrent builds;
these are single-thread results, below the ≥10M/s aggregate target. `bench/history.jsonl`
records the before/after measurements as well (including deliberately retained contended
runs as low as 104k/s; compare timings under equivalent load). The far-skipgram channel
is compiled out of the delta when no metric uses it.

An initial CSR of indices into the shared trigram list regressed (~303–310k/s), so it was
rejected in favor of inline sparse entries with cache IDs. Frequency pre-multiplication is
not shared across layout positions: frequencies are symbol-specific and costs slot-specific.
Caching frequency-weighted contributions would change f32 rounding, so the cache retains
the exact unweighted f32 cost and preserves the existing arithmetic. Future work: SIMD.

Measured 2026-10-23 (ansi-30 / english / default: 30 symbols, 3,414 trigrams, ~330 per
symbol, so a swap reads ~660 trigrams but only ~120 pair terms). The trigram loops index a
256-entry copy of the new slots, so `u8` symbol indices need no bounds check: 652k → 702k
deltas/s (+7.5%, three runs each, same results bit for bit (`gecko verify`)). Dropping the
cached-cost read entirely (an upper bound for keeping per-symbol cost sums instead) gave
≤5% more, so that is not worth its `apply` cost and drift risk.

## Adaptive fidelity (B8)

Profile `fidelity = { top_k = 1000 }` (optional `grow` = 4, `misses` = 3; CLI `optimize
--fidelity K`) makes the search cheaper when trigrams dominate the delta. The runner builds coarse
scorers that keep only the `top_k`, `top_k·grow`, … most frequent trigrams (`Scorer::coarse`,
not renormalized), then the full one. Every worker starts on the cheapest. When its best
improves, the layout is re-scored on the full tables, and only a lower full score counts as a
new best. After `misses` cheap bests that the full tables reject, the worker moves up one level
and restarts its optimizer from the best confirmed layout. At the end every kept layout gets a
steepest descent on the full tables. So reported scores are exact and results are exact local
optima. Used by hc, sa, ga, bls and tabu. memetic, nsga2 and rwsa ignore it, and explore and qd
always search exactly because their archives record scores. `best.json` `fidelity` lists the
levels, the confirmed and rejected counts and each worker's final level. `trigram_coverage`
still applies first: it decides what "full" is.

Code corpus / code profile, SA, 4 threads, seeds 1–3, the same `--evals` per run
(`bench/fidelity-2026-10-23/`, `summary.py`). Wall times were measured while other jobs ran on
the machine: compare within a row only.

| board (trigrams) | evals | exact: time · best (mean) | K = 200 | K = 1000 |
|---|---|---|---|---|
| ansi-30 (4,758) | 20M | 21.2 s · 16.093 | 12.2 s · 16.104 | 10.9 s · **15.977** (all seeds) |
| corne-36 + code layers (24,778) | 20M | 46.3 s · **15.103** | 25.1 s · 15.286 | 24.4 s · 15.209 |
| corne-36 + code layers | exact 20M, K 40M | 37.6 s · 15.103 | | 45.3 s · 15.044 |

- Each evaluation costs about half as much (1.7–1.9×).
- ansi-30: with the same number of evaluations, K = 1000 reaches the best known layout (15.977) on
  every seed and exact search on one of three. K = 200 escalates often (levels up to 3) and misses
  on one seed.
- Layered corne (102 slots): at equal evaluations the coarse search is worse (+0.1). At about
  equal time (twice the evaluations) it is level with exact search (15.04 vs 15.10, seed spread
  0.3). The gain here is small: the 102-slot pair delta, which fidelity does not shrink, is a
  large share of the cost.
- Default: unset (exact). Use K ≈ 1000 when trigrams dominate (30-key boards with trigram
  metrics weighted). Not worth it on large layered boards.

## Geometry rebuild (G2)

Geometry search needs a scorer per candidate board. `Scorer::set_geometry(board)` (or
`with_geometry`) re-costs a scorer for a board with keys moved but the same slots, keeping
everything that comes from the corpus (frequency tables, sparse trigram lists, brackets) and
the weights. Per slot it keeps two masks: **logical** (any key field but `pos`/`rot`/`size`
changed) and **moved** (the key or its `metric_points` entry changed). Pair/triple tables that
read metric points (`metrics::reads_points`: `sfb_distance` `sfs_distance` `stretch` `skip_stretch` `row_jump` `brackets` `contortion`) recompute the rows
and columns of moved slots; every other pair/triple table only those of logical changes, so a
pose-only move touches none of the trigram-class, SFB or scissor tables. Mono and place tables
(O(N); the hand model and home-row lookups are board-wide) are rebuilt in full, sharing one
`metrics::Prep` (metric points, hand model computed once, not per metric). Changed entries are
refolded as `new` folds them (0, then `+ w·t` in `METRICS` order), so the result equals
`Scorer::new` on the moved board bit for bit (`tests/rebuild.rs`). A changed layer key or
comfort table marks every slot; a different slot structure is an error. On a 3D board moving one
key re-fits its hand's plane, so all of that hand's keys are "moved" (geometric tables only).
`row_jump` measures each key from its finger's home keys, so a moved home key marks its whole hand
moved; key effort (`effort`, `typing_time`) reads the home row, so a logical change to a home key marks every
slot. `row_jump` and `typing_time` rebuild their whole table (O(N²)) on every rebuild.
States must be `refresh`ed afterwards. Until 2026-10-23 the pose-only check was inverted. A
logical-only change (a finger reassignment) then left the pair and trigram tables stale, and a
pose move also rebuilt the logical tables. Effort changes are mono, so the tests missed it;
`tests/fingering.rs` covers it now. After the fix, a one-key pose rebuild on ansi-30 takes
0.46 ms (`gecko perf`, under load).

Measured 2026-10-21 (i9-13900H, english / default, `gecko perf`, one key nudged per rebuild):

| board | `Scorer::new` | one-key rebuild | speed-up |
|---|---|---|---|
| ansi-30 | 3.1 ms | 0.64 ms | 4.8× |
| corne-36 | 4.6 ms | 0.78 ms | 5.9× |
| dactyl-manuform-5x6 (3D) | 19.5 ms | 2.2 ms | 9× |

Criterion (`tables/build`, `tables/rebuild_one_key`, ansi-30): 2.80 ms vs 0.66 ms. The floor on
flat boards is the hand model (`reach`/`wrist`/`press_angle`, ≈ 0.5 ms for 30 keys), which is board-wide (hand
placement on the home keys, elbow midline) and so recomputed on every rebuild.

## Normalization and profiles

- Frequencies are normalized over the symbols on the layout: values read as "% of the
  keystrokes / bigrams / skipgrams / trigrams typed on this layout".
- Raw metric values are in natural units (%, average effort). A profile's optional `[scale]`
  divides each weight (`weight / scale`); `gecko profile normalize --layouts …` sets scale = the
  metric's stdev over those layouts and rewrites weights so the objective is unchanged. Bundled
  profiles use the 9 references without QWERTY (`scale_basis`): QWERTY is so far out that it set
  every scale (SFB spread 1.66 with it, 0.79 without). Weight 1 = "one spread among decent
  layouts".
- `gecko profile fit --target engram` fits non-negative spread-unit weights so the other metrics
  predict a judge metric over layouts sampled around the references (reports R²).
- `trigram_coverage` (default 0.99) keeps the most frequent trigrams covering that share
  and renormalizes; scoring, deltas and reports all use the same truncated table, so they
  stay consistent. Equal frequencies at the cut break by characters, so the kept set does not
  depend on the start layout's symbol order. About 2× faster search than exact (1.0).
- `gecko profile fit --target likes` learns non-negative weights with pairwise logistic loss
  on unique liked cmini layouts, holding out 20% of whole layouts before computing scales.
  The fit got 53.8% held-out pair accuracy vs default 52.0% and Engram 49.4% (seed 1,
  2026-10-21 profiles; `research/experiments/2026-10-21-profile-ranking.md`). It is a ranking
  aid, not an optimization target: its optimum games the fit (home row 8%, SFB 5.4%), so the
  bundled `learned` profile was removed on 2026-10-22 and the command prints a warning.
- Profiles are TOML in `data/profiles/`: `default`; `hand`, `code`, `familiar` (default plus the
  hand model, layer costs, QWERTY familiarity); `balanced` (the default until 2026-10-24; Phase 2
  and the validation below used it); `sfb`; `engram` (independent judge, `engram` only).
  Weights, soft targets, constraints. Profiles are versioned; runs record the hash.

## Default profile (2026-10-24)

Found by a bilevel search ([cost-function optimization](../research/experiments/2026-10-24-cost-function-optimization.md),
[decision](decisions/scoring.md)): CMA-ES over the weights, each candidate scored by the layout
it optimizes to. That optimum is judged on typing time (key-pair and trigram timing regressed on
the metric tables, refit on a held-out half of the typists) and on Engram measured comfort, with
equal weight per spread. The community ranking of 16 tier layouts is kept (ρ ≥ 0.8).

- Weights: `engram` 0.62 → 2.08, `hand_switch` 0 → 1.0, `bad_redirect` ×2, `effort` ×2, `finger_target`/`hand_balance` ×3–5;
  `sfs`/`sfs_far`/`skip_stretch`, `alternate`, `redirect`, `scissor`/`half_scissor`, `movement`, `home_row` cut to ≤ 0.1 (no measured cost). `sfb`, `pinky_ring`, `stretch`, rolls kept.
- Timing coefficients (log time per unit, 90 % CI): SFB +0.16 (0.11–0.21), alternation −0.085,
  pinky–ring +0.08, stretch +0.05, row jump +0.04, effort +0.03; third keystroke: bad redirect
  +0.036, roll out −0.014, SFS +0.015 (0–0.026), trigram alternation 0.
- Optimum (BLS 25M, 10/10 seeds): `wgncvzuo,q dtrsfyheia pblmjxk/;.`, 8.074. Against the
  `balanced` optimum it is better on all 7 judges (time model on either typist half, empirical
  time judge under two practice models, `typing_time` 6.54 vs 6.64, `engram` 4.04 vs 4.91, `engram_formula` 4.38 vs 4.79). It
  pays with SFS 8.5 vs 6.4 %, SFB +0.06 pp, home row −2 pp. Both english and shai winners rank
  1st–2nd on all five corpora. Reference order: Graphite, Sturdy, Semimak, Canary, Dvorak,
  Halmak, Colemak-DH, Colemak, Workman, QWERTY (Halmak moves up two).

### Previous default (`balanced`, 2026-10-22)

Retuned against keystroke timing and Engram comfort
([validity study](../research/experiments/2026-10-22-cost-function-validity.md),
[decision](decisions/scoring.md)). In old-basis spread units: `sfb` 8.28 → 6, `sfs` 1.06 → 0.75,
`sfs_far` 0.31 → 0.22, `stretch` 2 → 1, `movement` 0.5 → 0.25, `skip_stretch` 0.19 → 0.12, `effort` 0.279 → 0.6, `alternate` 0.176 → 0.59,
`inroll` 0.105 → 0.31, `outroll` 0.126 → 0.63; new `scissor` 1.2, `half_scissor` 0.3, `pinky_ring` 1.2, `row_jump` 0.8, `home_row` 0.3; `sfb_distance` dropped;
`finger_target`, `hand_balance`, `redirect`, `bad_redirect`, `engram` unchanged. Then renormalized to the 9-layout basis (same objective).

- Why: timing gives SFB the largest effect (+0.55 median interval) and scissors (+0.10),
  pinky–ring (+0.13) and row jump real ones, rates in- and out-rolls equal, and gives `stretch` almost
  nothing. The old profile spent 35 % of the score variance among good layouts on `stretch`. Engram
  comfort is led by key effort and row jump. The weights that reproduce the time judge (NNLS,
  ρ 0.997) put 11 % on alternates (`alternate`) and give rolls their share; trigram timing has rolls
  5.7 % faster than alternation. Home row (`home_row`) backs the effort term.
- Checks: reference order unchanged (Graphite, Sturdy, Semimak, Canary, Dvorak, Colemak-DH,
  Colemak, Halmak, Workman, QWERTY). Spearman of score vs the time judge over good cmini
  layouts: 0.44 → 0.87. Optimum (SA 20M evals, two seeds, identical):
  `lwdgbzquo. rntsfpheai jxmcvky/;,`. It sits at the 1st percentile of cmini on the time judge,
  with SFB 0.89 %, SFS 6.43 %, home row 63.4 % and effort 1.689, and scores 18.82 under the old
  default (Graphite 21.68).
- Rejected: keeping SFB at 8.28 (optimum home row 59.6 %, worse effort and `engram`); halving `sfs` and
  `stretch` (SFS 7.83 %).

## `typing_time`

A bigram table: the extra time of typing slot j after slot i, in % of a median keystroke
interval. The model is fitted to fast touch typists (≥ 70 WPM, 18.5k people, 351 letter bigrams
of the Aalto 136M-keystroke data, median relative interval controlling log bigram frequency;
cross-validated R² 0.67 with log frequency vs 0.28 without the model):
`0.5522·SFB + 0.0578·same hand + 0.0476·adjacent fingers + 0.0373·rows + 0.1004·two-row scissor
+ 0.1263·pinky–ring + 0.0201·effort(j)`. Thumb pairs get only the effort term; same-key pairs 0.
Logical rows, so it reads no metric points. Fitted without SFB distance or roll direction (both
≈ 0). `typing_time` is a judge: weight 0 in every bundled profile, and fits skip it (like `engram_formula`/`engram`).
Optimizing it would chase the model that checks the score. English / ansi-30: Graphite 6.84,
Dvorak 7.29, Colemak-DH 8.17, QWERTY 13.06. `gecko judge` reports it with the other judges.

## Robustness

A score is linear in the weights: `Σ (weight / scale) · u` with `u` each metric's signed cost at
unit weight (`robust::UnitCosts`, `gecko features`). So a ranking can be re-scored under other
weights without rebuilding scorers.

- `gecko compare --robust` (σ = 0.5 log-normal noise on every weight, 2,000 draws, default
  profile): Graphite first in 100 % of draws, Sturdy, Semimak, Canary and Dvorak share ranks
  2–5, Halmak and Colemak-DH 6–7, QWERTY last in all. Report adjacent ranks as ties unless
  `P(>next)` is high. (`balanced`: Graphite 99 %, Sturdy 2nd in 89 %.)
- `gecko sweep` (default profile, 10M evaluations per search, 30 draws + one-at-a-time;
  `bench/costfit-2026-10-24/results/sweep-evidence.json`): the base optimum's family collects
  60 of 94 weight vectors, the near-tie `cwdlxqyou, gstnvpheia fbmrjzk/;.` family 25. σ = 0.5
  draws move a median 19 keys at a median regret of 0.55. Only `sfb`, `outroll`, `inroll`, `pinky_ring`, `engram` and `stretch`
  move the optimum when dropped or doubled; `hand_balance`, `home_row`, `sfs`, `sfs_far`, `alternate`, `redirect` are inactive. The
  optimum is a family, not one layout. (`balanced`, `bench/sweep-2026-10-22/`: 55 of 111, regret 1.10.)
- `gecko optimize` notes results within 0.4 of the best that differ in more than 4 keys
  (mirror-invariant) as near ties.

## Calibration

The default profile must rank known layouts sensibly (modern layouts > Colemak-DH and
Dvorak > QWERTY; Colemak-DH vs Dvorak is left open, see [decisions](decisions/scoring.md)) and reproduce published SFB percentages within tolerance. A
regression test pins these rankings (`ranking_sanity_on_english`). On the bundled corpus
QWERTY SFB = 6.63% (published ≈ 6.6%). Independent judges per layout: `gecko judge [--community]`
(`typing_time`, `engram`/`engram_formula` Engram, home row, effort, SFB, with cmini percentiles). Validity against 136M-keystroke timing, Engram comfort and
weight perturbations: [cost-function validity](../research/experiments/2026-10-22-cost-function-validity.md). Cross-checks: `gecko compare -p engram` (measured
comfort) and `gecko compare --corpora english,shai,reddit,french` (robustness, L05; add `writer,programmer,vim-programmer` for the full-symbol languages). The validation below also used the since-retired `english-cased` and `mt-450k` (`data/corpora/legacy/`).

**Community likes (cmini, `gecko layouts likes`).** 4121 cmini layouts → 1965 importable
(30-key core, all letters, standard or angle fingering) → 1904 after merging 55 duplicates and
6 mirrors; 303 have at least one like. Spearman ρ of GECKO goodness (−score) vs likes, ansi-30,
with cmini's angle-board layouts scored on `ansi-30-angle` (angle-mod fingering, see Fingering
below; `--no-angle` scores them with standard fingering):

| profile · corpus | all (1904) | liked only (303) | ortho (1504) | angle (358) | stagger (40) |
|---|---|---|---|---|---|
| default · english | +0.078 | +0.061 | +0.075 | +0.106 | −0.359 |
| default · shai | +0.077 | +0.071 | +0.073 | +0.111 | −0.386 |
| engram · english | +0.050 | +0.036 | +0.056 | +0.062 | −0.321 |
| default · english, `--no-angle` | +0.057 | +0.080 | +0.075 | +0.043 | −0.359 |
| default · english on ortho-30 (no angle board) | +0.052 | +0.076 | +0.074 | +0.030 | −0.364 |

Default rows: 2026-10-22 profile (now `balanced`). Liked layouts sit at a mean GECKO rank percentile of 45.0 % vs
50.9 % for unliked ones. Reading: likes barely track layout quality. They measure fame and age
(Graphite 77 likes, rank 15; the joke layout `inqwerted` 76 likes, rank 1822; Dvorak and
Colemak-DH ~20 likes, ranks 1198 and 1449), and 84 % of layouts have zero likes (ties). The
weak positive ρ among liked layouts is the usable signal; `cmini_likes_ranking` pins it
(ρ > 0.05, Graphite and Gallium in the top 10 % of liked layouts: 5th and 17th of 303). Angle
fingering lifts the angle group from +0.043 to +0.106 and the whole set from +0.057 to +0.078;
liked-only ρ drops (0.080 → 0.061) because angle layouts now outrank some liked ortho ones. Engram's comfort tables are per key position,
so its angle column does not change. The stagger group is small (40, 9 liked) and
noisy. Per-metric ρ vs likes is printed too (all |ρ| ≤ 0.08). Use these
layouts for C7 as preference pairs between liked layouts, not as raw like counts.

## Fingering (angle mod)

Fingering is part of the geometry. `ansi-30-angle` is `ansi-30` with the angle mod: the same
key positions, but the left bottom row is typed one finger over (Z ring, X middle, C V B
index; the left pinky has no bottom key). Re-fingered keys take their new finger's natural
bottom-row effort ([decision](decisions/scoring.md)). Reference layouts, `balanced` profile
(the default when measured), english, standard → angle fingering (lower is better; Colemak-DH angle is the ANSI angle
variant with the `xcdvz` bottom row):

| layout | ansi-30 | ansi-30-angle | Δ | largest changes |
|---|---|---|---|---|
| Graphite | 14.74 (1) | 14.87 (1) | +0.13 | SFB 1.16 → 1.28 %, movement −21 % |
| Sturdy | 17.67 (2) | 18.21 (2) | +0.54 | SFB 0.89 → 1.03 % |
| Semimak | 19.71 (3) | 19.31 (4) | −0.40 | movement −27 %, stretch down, SFB +0.13 |
| Canary | 19.93 (4) | 18.25 (3) | −1.68 | SFB 0.97 → 0.74 %, movement −32 % |
| Dvorak | 21.99 (5) | 22.26 (5) | +0.27 | SFB, stretch slightly up |
| Colemak-DH | 26.44 (6) | 26.89 (7) | +0.45 | SFB 1.54 → 1.64 % |
| Colemak-DH angle | 28.33 (9) | 26.12 (6) | −2.21 | SFB 1.72 → 1.50 %, stretch down |
| Colemak | 27.37 (7) | 27.69 (8) | +0.32 | as Colemak-DH |
| Halmak | 28.01 (8) | 28.41 (9) | +0.40 | SFB 2.75 → 2.99 % |
| Workman | 30.76 (10) | 30.10 (10) | −0.66 | SFB 3.00 → 2.85 % |
| QWERTY | 61.97 (11) | 60.77 (11) | −1.20 | SFB 6.63 → 6.42 %, stretch down |

Reading: angle fingering is worth about 2 points to layouts designed for it (Canary, the
Colemak-DH angle variant) and costs 0.1–0.5 to layouts designed for standard fingering, almost
all through SFB: the left index now covers C V B, so its bottom-row letters collide with its
home column. Movement (`movement`) and stretch (`stretch`) fall for most layouts because the bottom-left
keys are closer to their fingers. Score a layout on the fingering it was designed for;
compare fingerings with `gecko compare -g ansi-30` vs `-g ansi-30-angle`.

## Fingering search (C11)

Fingering can also be searched. A key may list other fingers that can press it, each with its
effort (`alt` in the board file; [representation](representation.md)). `gecko optimize
--fingering` runs the layout search, then a descent over single-key reassignments
(`fingering::search`): each candidate re-costs the scorer (`set_geometry` rebuilds only the
changed slot's rows and columns, ~1 ms) and runs a warm-started SA (`--fingering-evals`, default
300k swaps) plus a steepest descent from the current layout. The given fingering gets the same
SA first. The best improving reassignment is taken, until none improves. A changed fingering is
written to the run's `geometry.toml` and every result is re-scored on it. Home keys have no
options, and options are independent per key.

`ansi-30-alt` is ansi-30 with the angle mod's three reassignments as options, with the efforts
`ansi-30-angle` gives them (Z ring 2.8, X middle 2.4, C index 1.8). V keeps effort 1.8, where
ansi-30-angle raises it to 2.6. English / `balanced` (then the default), SA 20M evals, 4 threads, seeds 1–3
(`bench/fingering-2026-10-23/`):

| board | best | fingering |
|---|---|---|
| ansi-30 (fixed) | 12.154 (all seeds) | standard |
| ansi-30-angle (fixed, C10) | 11.757 (all seeds) | angle mod |
| ansi-30-alt + `--fingering` | 11.665 (seeds 2, 3), 12.077 (seed 1) | X → middle, C → index (seed 1: X only) |

- The search finds the angle-mod optimum's layout (`lmdgbjquo. rntsfpheai xvcwzky/;,`) with two
  of the three angle reassignments. It keeps Z on the pinky: on that layout Z holds `x`, and
  moving it to the ring finger costs +0.012 (11.665 → 11.677).
- The 0.09 gap to ansi-30-angle is V's effort (11.744 with V at 2.6). With angle-mod efforts the
  search's fingering is 0.012 better than the full angle mod.
- Seed 1 stops at a local optimum (X only, 12.077). The descent is greedy, so run several seeds.
- Runs: ansi-30 `20261023-152108-sa-195cfe32`, ansi-30-angle `20261023-152124-sa-fc9d060d`,
  `--fingering` `20261023-152214-sa-b9912990` (seed 2).

## Multi-objective

Besides the weighted sum: rank-sum selection over many results (Kühlmak) and Pareto
fronts (NSGA-II). Reports always show the per-metric breakdown.

## Adaptive keys (magic / repeat)

A layout with `[[magic]]` keys is scored on its corpus rewritten as typed with them
(`representation.md`, Corpus): every metric then sees the key as one more symbol, so an SFB
`c y` that becomes `c ★` disappears if ★ is on another finger, and the key's presses count in
effort and finger load. Nothing changes in the hot loop or the delta (rules are fixed during a
search; the key moves like a symbol). Total keystrokes are unchanged (one press replaces one),
so scores stay comparable with plain layouts, except that a symbol dropped to make room for
the key no longer counts. Not modeled: the learning cost of rules (only `gecko magic
--min-gain` limits their number). Rule search: `context/algorithms/magic-rules.md`.

## Layers (`layer_switch`, `symbol_bigram`, `brackets`)

- **`layer_switch`** (bigram table on the layered geometry, expanded per layer mode): typing `j` after
  `i` costs 100 per layer-key press in between, plus 100 for each press next to a different key
  on the same finger (same-finger into or out of the layer key). Presses: a toggle layer's key
  to leave `i`'s layer when `j` is elsewhere, then `j`'s layer key unless `j`'s layer is already
  active — one-shot layers always need it. Hold releases are free. So hold costs once per run
  of layer symbols, one-shot once per symbol, toggle twice per run (in and out). Reads
  "presses per 100 bigrams".
- **`symbol_bigram`** (own channel `fy`/`Cy`, `Order::SymBi`): frequencies are only bigrams of two ASCII
  punctuation symbols, normalized on their own; cost 100 when they are on one finger (two keys),
  on different layers, or the first is on a one-shot layer (a tap between them). Reads "% of symbol bigrams that are awkward". Adds one term to the
  pair delta only when weighted.
- The layer key's own keystroke: every layer symbol is typed with its layer key held, so `effort`
  adds the holder key's effort to each layer slot and finger loads (`finger_target`, `hand_balance`) count the
  keystroke on the holder's finger too (loads then sum to more than 100%).
- **`brackets`** (sparse list `Order::Pairs`, cost table `Cb`): each of `() [] {} <>` on the
  layout, weighted by the corpus frequency of its two symbols (weights sum to 1). Cost 0 when
  opener and closer are on one layer and adjacent (same hand and row, neighbouring columns,
  opener left) or mirrored (same row, finger and stretch flag on opposite hands, opener on the
  left hand); 50 for those shapes reversed; 100 otherwise. Reads "% of bracket use that is
  badly paired". The delta loops over ≤ 4 entries, and only when weighted.
- **`vim`** Vim motions (`Order::Pairs`, sparse pairs like `brackets`; block `t` in each `PairEntry`
  says which sparse table it reads): profile `vim` picks the key pairs, `row` = h–j, j–k, k–l;
  `pairs` = h–l, j–k; `fingers` = all six. For `row`/`pairs` a pair costs 0 when the keys are
  neighbours on one row and layer in order (first key left), 50 reversed, 100 otherwise; for
  `fingers` 0 on different non-thumb fingers of one hand and layer, else 100. Pairs weigh equally,
  so `vim` is "% of the policy broken" and needs no corpus. No `vim` = 0. QWERTY: row 0, pairs 50,
  fingers 16.7. [vim.md](vim.md).
- **`shortcuts`, `familiar`** (`Order::Place`, an n × n symbol-on-slot table `Cp` folded like a mono term: `full` sums
  `Cp[x, p(x)]`, a swap delta touches two entries, QAP terms add it to the mono matrix). Tables
  are built in the scorer (they need the symbols) and already carry their weights. The QWERTY
  reference is the 30 letter keys fitted onto the board (`metrics::qwerty_slots`); boards without
  30 open keys have no reference. **`shortcuts`** is per shortcut letter, not per keystroke: Z X C V
  weight 1, A S Q W weight ½, normalized to sum 1; cost 100 off the left hand, else
  `50 · min(d / 2, 1)` for distance `d` (key units) from the letter's QWERTY slot. **`familiar`** needs
  profile `familiar_to = "qwerty"` (else 0): each QWERTY symbol's keystroke share × 0 on its
  reference key, 50 on another key of its finger, 75 on another finger of its hand, 100 on the
  other hand. Sample profile: `data/profiles/familiar.toml` (scales are rough).
- **Shift** (layout has `⇧` keys and the corpus has case counts, `GKCORP3`): a layer-like hold
  on its own fixed key. Each press (one per run of shifted symbols) is typed with the best `⇧`
  for the shifted key y: not on y's finger, then the other hand, then lowest effort. Costs, per
  keystroke: `effort` adds that key's effort; `layer_switch` adds 100, +100 when Shift's finger is y's
  (held while pressing y), +100 when the key before is a different key on Shift's finger. Own
  channels: mono `fsh`/`Csh` and pair `fsb`/`Csb` (the latter compiled into the pair delta only
  when weighted). Without `⇧` on the layout Shift costs nothing, so 30-key layouts score as
  before. A one-shot Shift costs the same for single capitals; caps runs are scored as held.
- **Ctrl** (layout has `⎈` keys): profile `ctrl` presses per keystroke over the shortcut letters
  (`profile::CTRL_SHORTCUTS`), each costed like a Shift press before the letter (best `⎈` key's
  effort in `effort`, `layer_switch` 100 or 200 on the letter's finger). Folded into `Cp` with the `effort`/`layer_switch`
  weights, so full, delta and QAP terms need no new channel; `shift_value` reports it.
- **Shift pairs** (`Layout.shifted`): the corpus is keyed first (`representation.md`), so a pair
  key carries both symbols' n-grams and the shifted one's Shift presses.
- **`zones`** (`Order::Place`, count of keys): with profile `thumb_zone` (e.g. `␣⌫⏎`), a
  zone symbol off the base thumb keys or a letter, digit or punctuation symbol on one costs 1;
  with `base_letters`, a letter or zone symbol on a layer key costs 1. Keysets weight it 100, a
  constraint ([keysets.md](keysets.md#special-keys)).
- Not modeled yet: the Shift key's own keystroke in finger load,
  layer state beyond one bigram (toggle assumes each bigram starts from `i`'s layer). Profile `code` weights `layer_switch`/`symbol_bigram`/`brackets` (scales are rough estimates).

## Home-row mods (`hrm_misfire`)

- Bigram table: 100 when the first key is a home-row mod (`Geometry::hrm`) and the second is
  a different finger of the same hand, including its thumb. That roll presses the next key
  while the mod is still down, which is how tap-hold resolvers misfire. Same-finger pairs
  release first, rolls into a mod and other-hand pairs are free. Layer copies are never
  mods. Reads "% of bigrams at risk".
- No bundled profile weights it, because HRMs are opt-in. References on english/ansi-30: QWERTY
  9.9 %, Sturdy/Dvorak 16 %, Colemak 22 %, Workman 26 %, spread ≈ 4.25. With that scale, weight
  0.25 leaves the default optimum unchanged (15.1 %). Weight 1 cuts `hrm_misfire` to 5.6 % by moving
  letters off the home row (`home_row` 58 → 31 %), so it costs a lot. Runs `20261021-104858-sa-3817de78`
  and `20261021-104833-sa-be473459` vs default `20261021-104823-sa-8909360f` (SA, seed 1, 10 s).

## Thumbs (`thumb_roll`, `thumb_alt`, `thumb_load`)

- Trigrams with a thumb key get their own classes instead of "other" (which now means only a
  same-finger step or a repeated key, thumbs included). Only steps that touch a thumb decide:
  **`thumb_roll`** if any of them stays on the same hand (index → space on the left thumb),
  **`thumb_alt`** if all of them change hands. Both `max`, weight 0 in bundled
  profiles. On corne-36 / English with Space on a thumb they are ≈ 33–36% and ≈ 15–20% of trigrams.
- **`thumb_load`** (`Order::Load`): thumb share of keystrokes (%), scored like `hand_balance` as
  `(share − thumb_target)² / 10`. Profile `thumb_target` defaults to 18 (Space alone on
  English is 18.3%); raise it to ask for a letter on a thumb. Updated from the finger-load
  accumulators, so it adds nothing to the delta beyond `finger_target`/`hand_balance`'s load path.

## Relative finger effort

Optional profile `finger_effort` contains `typing`, `lateral`, `vertical` arrays in
LP LR LM LI LT RT RI RM RR RP order and `symmetric` (default true, left values mirrored).
Typing costs must be positive; movement costs nonnegative; all finite. Scorer construction
normalizes inverse typing effort to 100% over fingers present in the geometry, including
thumbs, replacing `finger_target`/`finger_peak` targets and deriving `thumb_load`'s combined thumb target.
Movement adds `lateral × |column − home column| + vertical × |row − home row|` to `effort`,
using the cheapest home key of that finger, including layer holders. No home key means
zero added movement. Logical rows/columns keep split rotation/tenting out of these axes.
These are reach costs, not velocity or hand mechanics; existing SFB settings still apply.
Web defaults (pinky/ring/middle/index/thumb): typing 2/1.5/1.1/1/1, lateral
2.5/1.8/1.4/0.7/0.6, vertical 1.5/1.1/0.9/0.8/0.8. They are editable heuristics.
Legacy profiles without this block retain their scores. Costs fold at scorer construction;
no allocation or extra work is added to the optimization hot loop.

## Validation (#140, 2026-10-23)

Commit `fed6641`, cmini `0db088e`, profile `default` (now `balanced`), ansi-30 standard fingering. Scripts and
outputs: `bench/validation-2026-10-23/` (`cmini_compare.py` → `numbers.json`, `ranking.py` →
`ranking-<profile>.json`, `compare-corpora.*`, `likes-shai.txt`).

**Numbers vs cmini (shai, 10 references, same key rows).** Every stat reconciles exactly: once
the differences below are applied to cmini's own analyzer code, it reproduces GECKO to < 1e-4 pp
(`gecko_full` vs `words` columns). Raw cmini vs GECKO (QWERTY, Graphite; max |Δ| over 10):

| Stat (GECKO) | QWERTY cmini / GECKO | Graphite | max Δ | Cause |
|---|---|---|---|---|
| SFB (`sfb`) | 6.52 / 6.70 | 1.18 / 1.20 | 0.18 | Denominator: cmini divides by all bigrams incl. digits, capitals' symbols and other off-layout grams; GECKO by bigrams on the layout. Same numerator. |
| SFS (`sfs`) | 10.31 / 11.19 | 5.57 / 6.59 | 1.03 | cmini: dsfb trigram classes (no space, no repeated key). GECKO: every skip-1 pair, incl. a repeated middle (`llo`) and across a space (`x␣z`, estimated from word ends/starts). |
| Alternate (`alternate`) | 19.14 / 21.15 | 33.33 / 37.40 | 4.26 | cmini puts every trigram with a repeated key, incl. first = third (`ere`, `eve`), in `sfR` (~9.8 % of trigrams); GECKO classifies first = third by finger. |
| Redirect (`redirect`) | 5.94 / 11.96 | 1.72 / 2.51 | 6.02 | As above, plus one-hand trigrams whose ends share a finger (cmini `dsfb-red`) are redirects in GECKO (bad without an index finger). |
| Bad redirect (`bad_redirect`) | 0.42 / 1.46 | 0.26 / 0.37 | 1.04 | Same. |
| SFT (`sft`) | 0.20 / 0.44 | 0.03 / 0.03 | 0.23 | First = third same-finger trigrams (`ded`) are SFT in GECKO, `sfR` in cmini. |
| Roll in/out, onehand, alt-SFS (`inroll`/`outroll`/`onehand`/`alt_sfs`) | 19.79 / 20.54 … | | 0.98 | Denominator only. |

Two further effects, both < 0.25 pp: `trigram_coverage` 0.99 (default) drops the rarest 1 % of
trigrams (≤ 0.22 pp on `alternate`, ≤ 0.09 on rolls); with 1.0 the match is exact. cmini's
`trigrams.json` equals the in-word trigrams of its `words.json`, which GECKO imports. GECKO's
ansi-30 references differ from some cmini entries off the 30-key core (cmini Graphite has
`,` beyond column 9); the comparison uses identical rows. Second source: the Layouts Doc's
published SFB (QWERTY 6.6, Workman 3.1, Dvorak 2.6, Colemak ≈ 1.6 %; its own English corpus)
vs GECKO `english` 6.63 / 3.00 / 2.58 / 1.54.

**Numbers vs Oxeylyzer** (`d015a16`, its own shai data, same rows, ortho/traditional fingering;
`oxey_compare.py` → `oxeylyzer.json`). Divided by the share of n-grams on the layout (Oxeylyzer
counts digits, `-`, `=` and its shift symbol `⇑`; 93–94 % of bigrams are on the layout), SFB,
rolls in/out, onehand, redirect (+ sfs), bad redirect (+ sfs) and alternate + alt-sfs match
GECKO within 0.03 pp on all 10. The rest:

- Alt-sfs (+1.3 to +3.1 pp): Oxeylyzer puts same-key ends (`ere`) in alt-sfs and GECKO puts
  them in `alternate`. The sum matches.
- SFT (+0.04 to +0.58 pp): Oxeylyzer's SFT also counts trigrams with an adjacent repeated key
  (`llo`). Without those, it matches to < 0.003 pp.
- SFS (−0.38 to +0.17 pp): this comes from skips across a space. GECKO estimates `x␣z` from word
  ends × starts (25 % of skip pairs). Oxeylyzer counts them from text, where capitals (`⇑`) and
  punctuation break many of them. Its values fall between GECKO's in-word-only SFS and GECKO's
  full `sfs`.

`trigram_precision` 1000 vs 1e6 changes nothing on shai.

**Ranking vs consensus.** No published community ranking exists, so two signals: tiers
(1 = Graphite, Gallium, Sturdy, Canary, Semimak, Rain, Maya, Recurva, Nerps, APTv3; 2 = Colemak-DH;
3 = Colemak; 4 = Dvorak, Workman, Halmak; 5 = QWERTY; sources in `ranking.py`) and cmini likes
on the same 16 layouts. Spearman ρ of GECKO score vs consensus (shai; range over english,
english-cased, shai, mt-450k, reddit):

| Profile | ρ tier | ρ likes | cross-tier pairs agreeing |
|---|---|---|---|
| default | +0.85 (0.82–0.85) | +0.80 (0.67–0.80) | 70/72 (69–70) |
| sfb | +0.87 (0.70–0.87) | +0.65 (0.38–0.76) | 72/72 (63 on mt-450k) |
| hand | +0.85 (0.84–0.86) | +0.80 (0.66–0.84) | 70/72 (69–71) |
| engram | +0.54 (0.48–0.62) | +0.54 (0.52–0.56) | 57/72 (55–60) |

- Main disagreement (default, every corpus): **Dvorak above Colemak and Colemak-DH.** Dvorak
  has twice the SFB (2.79 vs 1.38 %, +5.1 weighted) but wins on pinky–ring pairs (`pinky_ring` 1.21 vs
  3.36 %, −3.0), alternation, redirects (2.45 vs 9.17 %), SFS and row jumps. The community
  ranks SFB first and values Colemak's QWERTY-familiar shortcuts; likes, though, agree with
  GECKO (Dvorak 20, Colemak-DH 19). The `sfb` profile orders every tier pair correctly.
- mt-450k (word list, every word once) also puts Dvorak above Rain; rare words raise
  everyone's SFB and flatten tier-1 differences.
- Within tier 1, GECKO vs likes is weak (ρ +0.53 shai, +0.10 mt-450k): these layouts differ by
  taste (rolls vs alternation), which no single objective settles.
- Over all 1904 cmini layouts ρ vs likes is only +0.08 (`likes-shai.txt`): likes there track
  exposure, memes (`inqwerted`, 76 likes, rank 1816) and near-duplicates, not quality. See
  `research/experiments/2026-10-21-profile-ranking.md`.
- `engram` disagrees most: Halmak and Dvorak rank 6th–7th, above Sturdy, Canary and Colemak-DH.

**Corpus robustness** (`gecko compare --corpora english,english-cased,shai,mt-450k,reddit`):
the 10 references keep their rank on every corpus except tier 1 (Graphite/Semimak/Canary/Sturdy
swap within ranks 1–4; Semimak is first on mt-450k). Ranks 5–10 never move.
