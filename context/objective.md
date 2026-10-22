# Objective

Full metric list with IDs: `context/metrics-catalog.md`. Implemented now (`gecko list metrics`):
B01 SFB, S01 SFS, S02 far SFS (gaps 2–3, decayed), S03 SFS distance, S04/S05 skip stretch (B14 on skipgrams), B03 finger speed,
B04 LSB, B05 scissors, B06 half scissors, B07 bigram rolls in, B09 pinky–ring pairs, B10 row jump,
B14 geometric stretch (Oxeylyzer 2), B11 finger-pair movement (KLO), M01 effort, M02 finger-load error,
M03 finger balance (RSS), M07 peak finger load, M04 hand balance, M09 thumb load, M05 home row, M06 center columns,
B08 alternation, T01–T08 and T12–T13 (thumb) trigram classes, T09 contortions, T10 irregular trigrams,
E04/E05 Engram key-pair discomfort (formula / measured; geometry `[comfort]` tables), G01 reach,
G05 press angle and G03 wrist posture (hand model, profile `[hand]`; `context/hand-model.md`),
B02 repeats (info), Y01 layer switches (and Shift), Y02 code-symbol bigrams, Y03 paired brackets, Y04 home-row-mod misfires. Profile `[sfb]` sets B03/S03's distance transform (linear, squared, fitts),
lateral multiplier and per-finger factors, and S02's `skip_decay`. Profile `row_offset`
(pinky ring middle index × [above, below] home) adds per-finger row costs to M01's geometry effort.

## Hand model (G01, G03, G05)

Mono tables from `gecko_core::hand` (`context/hand-model.md`): each hand is placed on its home
keys and every key solved by finger IK (planar fingers, 4-DoF thumb). G01 = joint deviation from
rest + splay + fingertip shortfall; G05 = press misalignment; G03 = wrist deviation, extension
and pronation of the resting hand against the forearm (elbows from `[hand.body]`), plus the
deviation of sideways reaches. Layer slots add the holder's cost (like M01). Profile `[hand]`
(`size` or `dims`, optional `body`) sets the hand; without it G01/G03/G05 use the average hand and
arms and B14's relaxed spread stays 1.35 u (with it: × span / 200 mm). Bundled `default` weights
them 0; `hand` weights G01 1.0, G05 0.5, G03 1.0 and halves M01. Scales (stdev over references,
english / ansi-30): G01 0.198, G05 0.060; G03 has none (its spread over layouts is ~0.004: a
board metric, weighted in natural units). Folded at construction; no hot-loop cost.

## B11 finger-pair movement

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

Findings (english, ansi-30): B11 spread over the references = 18.3. It agrees with B14 on the
optimum (20 s SA, seed 7: B11 at 0, 0.5, 1 gives near-identical layouts, B14 1.87–1.88, E05
5.22–5.27) but re-sorts references with ring-up scissors: Semimak (`bl`) 93.7 and Halmak (`ed`)
97.1 are above QWERTY (85.9); Sturdy 41.5 and Graphite 46.1 are the lowest. Weight 1 swaps
Halmak and Colemak; 0.5 keeps the reference order. Default 0.5.

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

Adding a linear metric to a channel that is already on costs nothing at search time; switching a
channel on does. Measured 2026-10-21 (one thread, `bench/software-paper-2026-10-21/metrics_scaling.py`):
swap deltas run at 4.8 M/s with bigram and gap-1 skipgram metrics only, 2.4 M/s once a far-skipgram
metric (S02) is on, 2.1 M/s with the load terms (M02, M03, M07), and 0.64 M/s once any trigram
metric is on; the rate is flat within each step.

## Delta evaluation

Swapping symbols a and b changes only terms that involve a or b:
bigrams O(S), trigrams O(S²). Nonlinear terms (M02, M03, M04, M07) update from per-finger accumulators.
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

## Geometry rebuild (G2)

Geometry search needs a scorer per candidate board. `Scorer::set_geometry(board)` (or
`with_geometry`) re-costs a scorer for a board with keys moved but the same slots, keeping
everything that comes from the corpus (frequency tables, sparse trigram lists, brackets) and
the weights. Per slot it keeps two masks: **logical** (any key field but `pos`/`rot`/`size`
changed) and **moved** (the key or its `metric_points` entry changed). Pair/triple tables that
read metric points (`metrics::reads_points`: B03 S03 B14 S04 S05 B10 Y03 T09) recompute the rows
and columns of moved slots; every other pair/triple table only those of logical changes, so a
pose-only move touches none of the trigram-class, SFB or scissor tables. Mono and place tables
(O(N); the hand model and home-row lookups are board-wide) are rebuilt in full, sharing one
`metrics::Prep` (metric points, hand model computed once, not per metric). Changed entries are
refolded as `new` folds them (0, then `+ w·t` in `METRICS` order), so the result equals
`Scorer::new` on the moved board bit for bit (`tests/rebuild.rs`). A changed layer key or
comfort table marks every slot; a different slot structure is an error. On a 3D board moving one
key re-fits its hand's plane, so all of that hand's keys are "moved" (geometric tables only).
States must be `refresh`ed afterwards.

Measured 2026-10-21 (i9-13900H, english / default, `gecko perf`, one key nudged per rebuild):

| board | `Scorer::new` | one-key rebuild | speed-up |
|---|---|---|---|
| ansi-30 | 3.1 ms | 0.64 ms | 4.8× |
| corne-36 | 4.6 ms | 0.78 ms | 5.9× |
| dactyl-manuform-5x6 (3D) | 19.5 ms | 2.2 ms | 9× |

Criterion (`tables/build`, `tables/rebuild_one_key`, ansi-30): 2.80 ms vs 0.66 ms. The floor on
flat boards is the hand model (G01/G03/G05, ≈ 0.5 ms for 30 keys), which is board-wide (hand
placement on the home keys, elbow midline) and so recomputed on every rebuild.

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
- `gecko profile fit --target likes` learns non-negative weights with pairwise logistic loss
  on unique liked cmini layouts, holding out 20% of whole layouts before computing scales.
  The opt-in `learned` profile gets 53.8% held-out pair accuracy vs default 52.0% and
  Engram 49.4% (seed 1); weak popularity evidence, not a replacement for comfort weights.
  Method, hashes and exact fit: `research/experiments/2026-10-21-profile-ranking.md`.
- Profiles are TOML in `data/profiles/` (`default`, `sfb`, `hand` = default + hand model, `engram` = independent judge, E05 only):
  weights, soft targets, constraints. Profiles are versioned; runs record the hash.

## Calibration

The default profile must rank known layouts sensibly (modern layouts > Colemak-DH and
Dvorak > QWERTY; Colemak-DH vs Dvorak is left open, see [decisions](decisions/scoring.md)) and reproduce published SFB percentages within tolerance. A
regression test pins these rankings (`ranking_sanity_on_english`). On the bundled corpus
QWERTY SFB = 6.63% (published ≈ 6.6%). Cross-checks: `gecko compare -p engram` (measured
comfort) and `gecko compare --corpora english,shai,mt-10k,mt-450k,reddit,french` (robustness, L05).

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

## Adaptive keys (magic / repeat)

A layout with `[[magic]]` keys is scored on its corpus rewritten as typed with them
(`representation.md`, Corpus): every metric then sees the key as one more symbol, so an SFB
`c y` that becomes `c ★` disappears if ★ is on another finger, and the key's presses count in
effort and finger load. Nothing changes in the hot loop or the delta (rules are fixed during a
search; the key moves like a symbol). Total keystrokes are unchanged (one press replaces one),
so scores stay comparable with plain layouts, except that a symbol dropped to make room for
the key no longer counts. Not modeled: the learning cost of rules (only `gecko magic
--min-gain` limits their number). Rule search: `context/algorithms/magic-rules.md`.

## Layers (Y01, Y02, Y03)

- **Y01** (bigram table on the layered geometry, expanded per layer mode): typing `j` after
  `i` costs 100 per layer-key press in between, plus 100 for each press next to a different key
  on the same finger (same-finger into or out of the layer key). Presses: a toggle layer's key
  to leave `i`'s layer when `j` is elsewhere, then `j`'s layer key unless `j`'s layer is already
  active — one-shot layers always need it. Hold releases are free. So hold costs once per run
  of layer symbols, one-shot once per symbol, toggle twice per run (in and out). Reads
  "presses per 100 bigrams".
- **Y02** (own channel `fy`/`Cy`, `Order::SymBi`): frequencies are only bigrams of two ASCII
  punctuation symbols, normalized on their own; cost 100 when they are on one finger (two keys),
  on different layers, or the first is on a one-shot layer (a tap between them). Reads "% of symbol bigrams that are awkward". Adds one term to the
  pair delta only when weighted.
- The layer key's own keystroke: every layer symbol is typed with its layer key held, so M01
  adds the holder key's effort to each layer slot and finger loads (M02, M04) count the
  keystroke on the holder's finger too (loads then sum to more than 100%).
- **Y03** (sparse list `Order::Brackets`, cost table `Cb`): each of `() [] {} <>` on the
  layout, weighted by the corpus frequency of its two symbols (weights sum to 1). Cost 0 when
  opener and closer are on one layer and adjacent (same hand and row, neighbouring columns,
  opener left) or mirrored (same row, finger and stretch flag on opposite hands, opener on the
  left hand); 50 for those shapes reversed; 100 otherwise. Reads "% of bracket use that is
  badly paired". The delta loops over ≤ 4 entries, and only when weighted.
- **L01, L02** (`Order::Place`, an n × n symbol-on-slot table `Cp` folded like a mono term: `full` sums
  `Cp[x, p(x)]`, a swap delta touches two entries, QAP terms add it to the mono matrix). Tables
  are built in the scorer (they need the symbols) and already carry their weights. The QWERTY
  reference is the 30 letter keys fitted onto the board (`metrics::qwerty_slots`); boards without
  30 open keys have no reference. **L01** is per shortcut letter, not per keystroke: Z X C V
  weight 1, A S Q W weight ½, normalized to sum 1; cost 100 off the left hand, else
  `50 · min(d / 2, 1)` for distance `d` (key units) from the letter's QWERTY slot. **L02** needs
  profile `familiar_to = "qwerty"` (else 0): each QWERTY symbol's keystroke share × 0 on its
  reference key, 50 on another key of its finger, 75 on another finger of its hand, 100 on the
  other hand. Sample profile: `data/profiles/familiar.toml` (scales are rough).
- **Shift** (layout has `⇧` keys and the corpus has case counts, `GKCORP3`): a layer-like hold
  on its own fixed key. Each press (one per run of shifted symbols) is typed with the best `⇧`
  for the shifted key y: not on y's finger, then the other hand, then lowest effort. Costs, per
  keystroke: M01 adds that key's effort; Y01 adds 100, +100 when Shift's finger is y's
  (held while pressing y), +100 when the key before is a different key on Shift's finger. Own
  channels: mono `fsh`/`Csh` and pair `fsb`/`Csb` (the latter compiled into the pair delta only
  when weighted). Without `⇧` on the layout Shift costs nothing, so 30-key layouts score as
  before. A one-shot Shift costs the same for single capitals; caps runs are scored as held.
- Not modeled yet: the Shift key's own keystroke in finger load,
  layer state beyond one bigram (toggle assumes each bigram starts from `i`'s layer). Profile `code` weights Y01/Y02/Y03 (scales are rough estimates).

## Home-row mods (Y04)

- Bigram table: 100 when the first key is a home-row mod (`Geometry::hrm`) and the second is
  a different finger of the same hand, including its thumb. That roll presses the next key
  while the mod is still down, which is how tap-hold resolvers misfire. Same-finger pairs
  release first, rolls into a mod and other-hand pairs are free. Layer copies are never
  mods. Reads "% of bigrams at risk".
- No bundled profile weights it, because HRMs are opt-in. References on english/ansi-30: QWERTY
  9.9 %, Sturdy/Dvorak 16 %, Colemak 22 %, Workman 26 %, spread ≈ 4.25. With that scale, weight
  0.25 leaves the default optimum unchanged (15.1 %). Weight 1 cuts Y04 to 5.6 % by moving
  letters off the home row (M05 58 → 31 %), so it costs a lot. Runs `20261021-104858-sa-3817de78`
  and `20261021-104833-sa-be473459` vs default `20261021-104823-sa-8909360f` (SA, seed 1, 10 s).

## Thumbs (T12, T13, M09)

- Trigrams with a thumb key get their own classes instead of "other" (which now means only a
  same-finger step or a repeated key, thumbs included). Only steps that touch a thumb decide:
  **T12 thumb roll** if any of them stays on the same hand (index → space on the left thumb),
  **T13 thumb alternate** if all of them change hands. Both `max`, weight 0 in bundled
  profiles. On corne-36 / English with Space on a thumb they are ≈ 33–36% and ≈ 15–20% of trigrams.
- **M09 thumb load** (`Order::Load`): thumb share of keystrokes (%), scored like M04 as
  `(share − thumb_target)² / 10`. Profile `thumb_target` defaults to 18 (Space alone on
  English is 18.3%); raise it to ask for a letter on a thumb. Updated from the finger-load
  accumulators, so it adds nothing to the delta beyond M02/M04's load path.

## Relative finger effort

Optional profile `finger_effort` contains `typing`, `lateral`, `vertical` arrays in
LP LR LM LI LT RT RI RM RR RP order and `symmetric` (default true, left values mirrored).
Typing costs must be positive; movement costs nonnegative; all finite. Scorer construction
normalizes inverse typing effort to 100% over fingers present in the geometry, including
thumbs, replacing M02/M07 targets and deriving M09's combined thumb target.
Movement adds `lateral × |column − home column| + vertical × |row − home row|` to M01,
using the cheapest home key of that finger, including layer holders. No home key means
zero added movement. Logical rows/columns keep split rotation/tenting out of these axes.
These are reach costs, not velocity or hand mechanics; existing SFB settings still apply.
Web defaults (pinky/ring/middle/index/thumb): typing 2/1.5/1.1/1/1, lateral
2.5/1.8/1.4/0.7/0.6, vertical 1.5/1.1/0.9/0.8/0.8. They are editable heuristics.
Legacy profiles without this block retain their scores. Costs fold at scorer construction;
no allocation or extra work is added to the optimization hot loop.
