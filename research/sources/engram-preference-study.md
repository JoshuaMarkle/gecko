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
  positions, normalized scores per objective. A ready **empirical C2 table**.
- `engram_3key_order_scores.csv`: 32,768 position triples.
- `typing_preferences_to_comfort_scores/data/processed_bigram_data.csv`: the choice data.

## How to use in GECKO

- Add an **"engram-comfort" profile**: map QWERTY position names to `ansi-30` slots, load the
  CSV as a pair-cost table (`1 − score`), weight by bigram frequency. Zero hot-loop cost.
- Use it as an **independent judge**: layouts tuned on GECKO's hand-written metrics should also
  score well here. Disagreement points at mis-weighted metrics.
- Reuse the method for **personal calibration** (phase 9): pairwise comfort choices in the
  web UI → Bradley–Terry / logistic model on GECKO's pair features → personal C2 weights.
- Caveat: the bigram tables are left-hand (`*_LH.csv`) and were typed on participants' own
  (mostly row-staggered) keyboards. Check the paper for how right-hand and cross-hand scores
  are derived before using it on column-staggered boards **(verify)**.
