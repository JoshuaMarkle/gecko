---
group: analyzers
kind: paper
read: 2026-10-20
status: verified
feeds: [E04, E05, C7]
take: Crowdsourced comfort → Bradley–Terry → reusable pair-score CSV
---

# Engram and the bigram typing-preference study (Arno Klein)

Paper: Klein, "Optimizing comfortable keyboard layouts using human typing preferences and
language-dependent n-grams: the Engram Study", *Int. J. Human–Computer Interaction* (2026),
doi:10.1080/10447318.2026.2665409. Earlier preprint (2021): https://preprints.org/manuscript/202103.0287

Repos (all by binarybottle):
- https://github.com/binarybottle/engram — layouts (Engram-en v2025.10.31: `byou dngv / hiae trsc / kjxw mlfp`).
- https://github.com/binarybottle/bigram-typing-comfort-experiment — web experiment +
  analysis (Apache-2.0), studies 1–9 in `bigram_tables/`.
- https://github.com/binarybottle/typing_preferences_to_comfort_scores — Bayesian preference
  model (Stan) → comfort scores (MIT).
- https://github.com/binarybottle/optimize_layouts — multi-objective optimizer (MIT).
- https://github.com/binarybottle/engram-2021 — older scoring model.

## Key ideas

- **Measure comfort directly.** Crowdsourced participants (Prolific + volunteers) type two
  bigrams repeatedly and pick the more comfortable one. Conclusion: **typing speed is a weak
  proxy for comfort.**
- **Bradley–Terry** on same-key bigrams (AA vs QQ) gives per-key comfort strengths with
  bootstrap CIs; pairwise choices give row-separation (same row > reach > hurdle) and
  column-separation (adjacent > distant) preferences, and index column 4 vs 5.
- **Bayesian preference learning** (Stan, participant random effects, feature selection with
  participant-aware CV) maps bigram features → latent comfort, extended to all 30 keys.
- **Active data collection:** next bigram pairs chosen by max–min distance in PCA feature space
  (`extra/pair_recommender.py`).
- **Optimizer:** item-pair (letter bigram) frequency × position-pair score per objective;
  objectives `engram_key_preference, engram_outside, engram_row_separation,
  engram_same_finger, engram_same_row` (+ a trigram `engram_order`). Pareto front by
  exhaustive search for ≤10 items, branch-and-bound for ≥12. Letters placed in phases
  (most frequent first), then the rest.

## Data worth reusing

- `optimize_layouts/input/engram_2key_scores.csv`: 1024 position pairs over 32 QWERTY-named
  positions, normalized scores per objective. Not a free-form measured table: it is a rule
  (`keyboard_layout_scorers/prep/prep_keypair_engram_scores.py`) whose constants come from
  the study. Converted for GECKO in `data/comfort/` (see below).
- `engram_3key_order_scores.csv`: 32,768 position triples.
- `typing_preferences_to_comfort_scores/data/processed_bigram_data.csv`: the choice data.

## How to use in GECKO

- Add an **"engram-comfort" profile**: map QWERTY position names to `ansi-30` slots, load the
  CSV as a pair-cost table (`1 − score`), weight by bigram frequency. Zero hot-loop cost.
- Use it as an **independent judge**: layouts tuned on GECKO's hand-written metrics should also
  score well here. Disagreement points at mis-weighted metrics.
- Reuse the method for **personal calibration** (phase 9): pairwise comfort choices in the
  web UI → Bradley–Terry / logistic model on GECKO's pair features → personal C2 weights.
## Mapping to GECKO (`data/comfort/`)

Files: `engram-2key.csv` (`a_row,a_col,b_row,b_col,score,row_separation,same_row,same_finger,outside,key_preference`,
900 ordered pairs, `a` typed first) and `engram-1key.csv` (`row,col,score`). Each has a
provenance header (source commits, MIT licence). All values 0..1, **higher = more comfortable**.

- **Numbering.** Engram names positions by QWERTY key. QWERTY row r, col c of the 3×10 block →
  ansi-30 row r, col c (row 0 top, col 0 left pinky). Engram's fingering (cols 4–5 index,
  T/G/B and Y/H/N as index stretch) matches ansi-30's. The two extra keys `[` and `'` are dropped.
- **`score` = Engram's `avg4`** = mean of four transition criteria:
  `row_separation` (same row 1, adjacent row 0.588, row skip 0), `same_row` (same row: inward
  1 / outward 0.779, ×0.75 if columns not adjacent; different rows 0.5; same finger 0),
  `same_finger` (0 if same finger), `outside` (1 / 0.846 / 0.716 for 0 / 1 / 2 keys in the
  center columns). `key_preference` (mean of the two keys' 1-key tiers) is listed but not
  in `score`; use `engram-1key.csv` as a separate mono term instead. Range of `score`: 0.179–1.0.
- **Mirroring.** All constants were measured on the **left hand** and applied mirror-symmetrically
  to the right (columns counted from the pinky). Same for the 1-key tiers.
- **Cross-hand pairs** are 1.0 on every transition criterion, so `score` = 1.0 unless a center
  column key lowers `outside`. Use the file as is: cross-hand = most comfortable (cost 0 after
  `1 − score`). This is Engram's model, not a measurement of alternation.
- **Same-key repeats** (`a == b`) count as same finger (score 0.5). GECKO reports repeats
  separately (B02), so a loader should probably zero the diagonal.
- **Center columns** (cols 4–5) were outside the study: 1-key tier 0 and an `outside` penalty only.
  Treat them as unmeasured, not as "worst".
- Verified: all 900 kept rows equal the published rule to 3 decimals. Spot checks: `df`/`kj`
  (inward roll, home row) 1.0; `fd` 0.945 (outward); `fr`/`qa` same-finger reach 0.397;
  `qz` same-finger row skip 0.25; `ex` scissor 0.625; `fj` 1.0.

Caveats: participants typed on their own (mostly row-staggered laptop/desktop) keyboards with
QWERTY habits, left hand only, so it fits ansi-30 reasonably. It is coarse (few distinct
values, no pinky-specific transition cost except via 1-key tiers, no distance term), and less
valid on column-staggered or ortho boards, where "row skip" and "outside" mean different reaches.

Recommended use: (1) an independent **judge profile**: cost = `1 − score` as a C2 pair table
(plus optionally `1 − tier` as C1), never tuned against GECKO's own metrics; report where it
disagrees with the default profile. (2) Later, a **fit target** for spread-normalized weights of
GECKO's geometric pair features. For finer data, fit to the raw choice data
(`typing_preferences_to_comfort_scores/data/processed_bigram_data.csv`) instead.

## Fitted table (`data/comfort/engram-bt.csv`)

A *measured* alternative to `engram-2key`, fitted from the raw choices
(`typing_preferences_to_comfort_scores@d8978d4`, `data/processed_bigram_data.csv`, MIT).
Reproduce: `python3 -I research/scripts/fit_engram_bt.py <choices.csv> data/comfort/engram-2key.csv data/comfort/engram-bt.csv`.

- **Data:** 4490 choices, 61 participants, 165 distinct bigram pairs over 112 distinct bigrams on
  12 left-hand keys (`qwer asdf zxcv`; no center columns). Each pair was shown twice per person.
- **Model:** Bradley–Terry, `P(a over b) = logistic(s_a − s_b)`, one strength per ordered bigram,
  L2 λ = 1 (picked by participant-grouped 5-fold CV log-loss; no participant random effects).
- **Held-out (participant-aware 5-fold):** accuracy 0.736, log-loss 0.517. The avg4 formula
  predicts 0.592 (34 % of test pairs tie under avg4, counted as 0.5; ≈ 0.64 on non-ties). In-sample
  0.767 vs a 0.771 ceiling (share of choices agreeing with each pair's majority), so the data are
  noisy and BT captures nearly all of the consistent signal.
- **Scale:** strengths z-scored and matched to the mean/std of avg4 over the measured bigrams, then
  clipped to 0..1, so fallback rows (avg4) and fitted rows share one scale. Higher = more comfortable.
- **Coverage on ansi-30:** 200 of 420 same-hand ordered off-diagonal pairs are measured (100 per
  hand, right mirrored); the rest (center columns, unseen bigrams) and all cross-hand pairs fall back
  to avg4 with `measured = 0`. Extra columns: `measured`, `bt_strength`, `n` (choices involving the
  bigram), `avg4`. The GECKO loader only reads the first five columns.
- **Agreement with avg4:** Spearman 0.39 over the 100 measured off-diagonal bigrams (0.91 over all
  900 rows, dominated by the shared cross-hand and fallback rows).
- **What differs:** one-row same-finger reaches with the middle or index finger are rated
  comfortable (`de` 0.89 with n = 264, `ed` 0.89 with n = 112, `fr` 0.83, `rf` 0.90; avg4 0.40), while pinky/ring
  transitions are worse than the formula says (`wz` 0.30, `xq` 0.22 vs 0.63). Home-row inward rolls
  stay at the top (`df` 1.0, `ds` 1.0, `sd` 0.95); `qz` is the worst (0.18).

Caveats: comfort when typing a bigram repeatedly at leisure is not speed, so "SFBs on strong fingers
are comfortable" does not contradict the SFB metric; use this as a judge next to it, not instead of it.
Strengths are relative to the comparison graph: bigrams compared only against bad pinky bigrams can be
inflated (e.g. `ce`, middle-finger row skip, 0.91 from n = 26). Left hand only, mirrored; QWERTY-trained
participants on their own (mostly row-staggered) keyboards. A model with participant effects and GECKO's
geometric pair features (the next step for weight fitting) would generalize to unmeasured pairs.
