---
group: analyzers
kind: tool
read: 2026-10-20
status: read
feeds: [B-class, M03, B8, search]
take: Full same-hand bigram classes; disjoint bigrams; threshold annealing with adaptive fidelity
---

# Kühlmak (Felix Kühling)

Rust analyzer + optimizer, MIT. https://github.com/fxkuehl/kuehlmak. Compared with the other tools in
[tool-comparison](../studies/tool-comparison.md); this file adds what the code shows.

## Key ideas (beyond the existing summary)

- **Complete same-hand bigram classification:** a 15×15 matrix per hand (picture in
  `doc/bigrams.png`, sheet `doc/bigrams.ods`). Every same-hand bigram gets a class
  (SFB, scissor, WLSB, down-roll, up-roll); hand changes score 0.
- **Disjoint bigrams:** the same classification applied to skipgrams across the other hand
  or space ("dSFB", "dRoll"). Weighted lower than the direct version.
- **Ring-finger rule:** rolls away from the ring finger are easy, toward it or skipping over it
  are hard (finger independence, see `hager-ross-finger-independence.md`).
- **WLSB:** lateral stretch weighted by 1/finger distance; pinky→index stretch counts 1/3 stretch
  and 2/3 roll.
- **Travel estimate from n-grams:** home→key for each press, corrected by SFB/dSFB
  (home→last replaced by first→last), weighted ×4 for SFB, ×2 for dSFB.
- **Balance by RSS:** per-finger and per-hand totals are squared, summed, rooted, normalized so a
  balanced layout equals the plain sum.
- **Space affinity:** `space_thumb = Any | L | R`. With a fixed space thumb, space joins
  same-hand sequences (rolls, redirects); with Any it breaks them.
- **Annealer (`src/anneal.rs`) is threshold accepting with adaptive fidelity, not Metropolis:**
  - accept a neighbour if `score < best + noise`; reset to best if `score > best + 100·noise`;
  - noise shrinks when no improvement for `steps_per_iter`, and grows by the improvement
    size when a new best is found ("exothermic" reheating);
  - candidates are scored at **partial precision** (only the most frequent n-grams); precision
    rises when a cheap score disagrees with the exact score. Multi-fidelity search.
- **Rank-sum selection** across many runs, **soft targets**, **constraints** (zxcv, homing keys,
  allowed keys per row, similarity to a reference layout with a threshold).

## How to use in GECKO

- Multi-fidelity is the main new idea: evaluate deltas on the top-K n-grams, confirm on the full
  table before accepting a new best. GECKO's `trigram_coverage` is a fixed version of this; an
  adaptive version gives speed early and exactness late.
- Add "disjoint" variants of B-metrics as skipgram tables (S04 generalization).
- `space_thumb` matters once space is on the layout (phase 5).
- Row-allowed constraints (`top_keys` etc.) are useful for uniform keycap sets.
