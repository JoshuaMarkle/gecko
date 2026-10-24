# Optimizing the cost function: weights from timing, comfort and a bilevel search

| | |
|---|---|
| Date | 2026-10-24 |
| Question | What are the best weights for GECKO's cost function, judged by what the optimized layout is like (speed, comfort), not only by how known layouts rank? |
| Answer | The 2026-10-22 default profile (now `balanced`) is Pareto-dominated. A bilevel search found weights (the `evidence` weights, `default` since 2026-10-24) whose optimum beats the default optimum on all 7 independent judges: −18 % measured discomfort, slightly faster, ranked 1st or 2nd on every corpus. Speed hardly separates good layouts (≤ 0.3 % per keystroke), so comfort and the trade-off between them decide the weights. |
| Command | `bench/costfit-2026-10-24/README.md` (costfit tool + scripts) |
| Data | Aalto 136M mirror re-scanned on all 30 keys (49,293 QWERTY touch typists in two halves); Engram BT comfort; 16 community-tier layouts; 10 references, 1,904 cmini, 3,000 random layouts; ansi-30 / english |
| Artifacts | `bench/costfit-2026-10-24/results/`, profile `data/profiles/default.toml` (was `evidence.toml`), runs in `bench/costfit-2026-10-24/runs/`, paper `docs/papers/cost-optimization/` |

Builds on the [cost-function validity study](2026-10-22-cost-function-validity.md), which fit
weights so that known layouts rank like a time model. This study asks the harder question: when
the optimizer exploits the weights, is the layout it finds actually better?

## Method

Three steps, each checked on data the step did not see.

1. **Calibrate on measurements, at the level the metrics are defined.** Each bigram metric is a
   table over key pairs, so measured key-pair time can be regressed directly on GECKO's tables
   (study 1). Trigram metrics are regressed on third-keystroke context (study 2). The
   coefficients are in "log time per keystroke" and convert to profile weights exactly:
   `weight / scale = K · β` (÷ 100 for % metrics).
2. **Judge optima, not rankings.** For each candidate weight vector: optimize a layout (SA,
   10 M evaluations, 2–3 seeds), then score that optimum on judges the weights never saw. These
   are the time model refit on the other typist half (T_A, T_B), the empirical pair-time judge
   (J_B, two practice models), E01, Engram measured comfort (E05) and Engram's formula (E04).
3. **Search the weights (bilevel).** CMA-ES over the log-weights of all 21 default metrics plus
   B08, B01 fixed (scale invariance), 16 candidates × 40 generations per run. Objective:
   `(1−α)·T_B/sd + α·E05/sd` of the optimum. Penalties keep the community's ranking of 16 tier
   layouts (Spearman ρ ≥ 0.8; default 0.85) and the default optimum's finger-load error. The top
   candidates of all runs were then re-optimized at 3 seeds × 10 M (winner's-curse guard).

## Data: 30-key timing, two halves

`iki4.py` re-scans the raw keystrokes. It takes QWERTY touch typists (9–10 fingers) and only
sections typed exactly with no Backspace. It records every interval between two of the 30
ansi-30 keys (punctuation included), as log interval ÷ the typist's median, plus the
third-keystroke interval of every triple. Participants are split by id parity: 24,653 / 24,640.

- **Reliability is high:** split-half r = 0.98 over the 415 letter pairs with n ≥ 30;
  typist-split ceiling R² 0.986 (pairs), 0.918 (triples).
- **Coverage is the problem:** 289 of 900 key pairs never occur. Good layouts put **35–45 %** of
  their bigram mass on pairs QWERTY typists type fewer than 30 times (QWERTY itself: 0.5 %).
  Measured timing alone cannot score a good layout; a model has to carry it.
- Other typist layouts don't fill the gap. Dvorak-labelled typists (74 usable) disagree with
  QWERTY on shared pairs (r ≈ 0; their letter field is often blank). AZERTY (r 0.73) and QWERTZ
  (r 0.88) add only 44 pairs.

## Study 1: key-pair time on GECKO's pair tables

Target: mean log relative interval per ordered letter pair (n ≥ 30, 401 pairs). Punctuation
pairs are excluded because `.` and `,` carry sentence-boundary pauses (`n.` +0.42, `e.` +0.30
residuals). Practice is a nuisance term: log pair count, which by itself explains R² ≈ 0.4 (a
pair typed 100× more often is ~40 % faster). Sign-constrained least squares; 90 % CIs from 200
bootstraps over unordered pairs. Linear practice, quadratic practice and letter-frequency
controls give the same coefficients within ±0.02. With no practice control the fit collapses
(R² 0.21).

| metric | β (log time / unit) | effect | 90 % CI | default weight → this fit |
|---|---:|---|---|---|
| B01 SFB | 0.159 | +17 % per SFB pair | 0.11 – 0.21 | |
| B08 alternation (bigram) | 0.085 | **−8 %** per alternating pair | 0.05 – 0.12 | not weighted → 3.5× SFB in spread units |
| B09 pinky–ring | 0.079 | +8 % | 0.006 – 0.18 | |
| B05 scissors | 0.061 | +6 % | 0 – 0.17 | |
| B14 stretch | 0.050 | +5 % per stretch unit | 0.003 – 0.10 | |
| B10 row jump | 0.037 | +4 % per key of vertical travel | 0.001 – 0.07 | |
| M01 key effort | 0.031 | +3 % per effort unit | 0.019 – 0.044 | |
| B06 half scissors | 0.012 | ~0 | 0 – 0.09 | |
| E05 Engram comfort | 0.000 | none on time | 0 – 0.06 | |
| B04, B07, B11, M05, M06, G05 | 0 | none beyond the above | | |

- Fit: R² 0.755 in sample. Beyond practice, the metrics explain 0.40 in sample and 0.36 in
  10-fold pair-grouped CV, the same across typist halves. Extrapolating from frequent pairs
  (n ≥ 1000) to rare ones explains only 0.11 (ρ 0.32): the rare-pair regime that optimized
  layouts live in is the least predictable.
- **What the metrics miss: the key itself.** Adding one effect per second key raises pair-CV R²
  0.39 → 0.56. Bottom-row keys (`x c v b`, +0.10–0.17) and the center column (`g`, `j`) are
  slower than M01's effort table says. Middle/ring top-row keys (`e i o`) are as fast as home
  (KLO's `row_offset` already leans this way). A timing-derived effort table is the largest
  untapped improvement, but key and letter are confounded in QWERTY data (`q`, `x` look slow
  partly because they are rare letters), so it needs a design that separates them.
- Engram's hand-built pair formula (E04) is the best single predictor (CV 0.72 with practice).
  E04 + B14 + B08 match all 17 metrics together.

## Study 2: trigrams

Target: the third interval of i→j→k minus the mean of j→k (context effect), 2,212 letter
triples with n ≥ 30; practice = log trigram count − log pair count.

| metric | β | effect on 3rd keystroke | 90 % CI |
|---|---:|---|---|
| T07 bad redirect | 0.036 | +3.7 % | 0.003 – 0.065 |
| S04 skip stretch | 0.019 | +1.9 % per unit | 0.006 – 0.028 |
| S01 SFS | 0.015 | +1.5 % | 0 – 0.026 |
| T04 roll out | 0.014 | −1.4 % | 0.003 – 0.025 |
| T03 roll in | 0.003 | −0.3 % | 0 – 0.014 |
| T01 alternate, T06 redirect | 0 | no extra credit beyond the bigram | |

Unconstrained, alternation is *slower* in context (+3 % vs rolls; the bigram already credits
it). A repeated first and third key (`ere`) costs +3 %. The classes explain R² 0.22 of a
signal with ceiling 0.92, so most trigram context is unmodelled.

## The empirical judge and its limit

`judge.py` builds a time judge independent of GECKO's metric definitions. It uses one typist
half's practice-adjusted pair means, shrunk toward a finger-pair × row-pair + key-effect model
fit on that half, plus the trigram context term. Halves agree (ρ 0.98–0.99 over cmini).
**It cannot rank whole layouts, though.** Controlling letter frequency (beside pair frequency)
moves QWERTY from last to 2nd of 10 references: a judge trained on QWERTY typists confounds
practice with motor cost. So both variants are reported, as one check among several, never as
the objective.

## Study 4–5: time-only profiles, and the speed/comfort trade-off

Converting the fitted coefficients into a profile (`time`) makes hand alternation the largest
term (B08, 3.5× SFB per spread). Its optimum (`,uoyjqwcgv iaehklnstd x;/.zmrfbp`) is
Dvorak-like. On the fitted time model it is only **0.29 % per keystroke** faster than the
default optimum. All modern references lie within ~1 % of each other; QWERTY is ~6 % slower.
Tier agreement drops to 0.55. **Speed barely separates good layouts.** What ranks them is
comfort and how much comfort is worth.

Sweeping the comfort share α (time model + α · E05, light load terms) traces the frontier.
Pure comfort drives SFB to ~3 %: Engram's isolated-bigram ratings barely punish SFB.

| α | optimum | Δ time vs default opt | E05 | SFB | tier ρ |
|---|---|---:|---:|---:|---:|
| 0 | `pchwxjyou. dsnrmkteai vfblqzg/;,` | −0.2 % | 4.54 | 1.09 | 0.50 |
| 0.5 | `cgnwvqyou. dsrtmkheia bfpljx,z/;` | −0.2 % | 3.76 | 1.61 | 0.51 |
| 1.0 | `,uoyqkdncl aihezptsrg x;j./bfwvm` | +0.3 % | 3.39 | 2.83 | 0.53 |

Evidence-only weights rank the known layouts unlike the community at every α, mainly by
putting Dvorak and Halmak higher. That is why the bilevel search keeps the tier constraint.

## Study 6–7: bilevel search

Four CMA-ES runs (α 0.25 / 0.5 / 0.75 with ρ ≥ 0.8; α 0.5 unconstrained), 640 inner
optimizations each. The landscape is rugged: the runs settled in different layout families,
and the α = 0.5 run did not reach the family the α = 0.75 run found. So the selection pools the
top 6 candidates of every run plus each run's CMA mean, and re-optimizes all of them. The
Pareto set (time vs Engram comfort), all with tier ρ ≥ 0.8:

| weights from | optimum | Δ time vs default opt | E05 | SFB | SFS | ρ |
|---|---|---:|---:|---:|---:|---:|
| α 0.25 run | `mflbzjyou, dsrnpgteai cvwhqxk/;.` | −0.23 % | 4.31 | 0.83 | 7.49 | 0.80 |
| **α 0.75 run, #3 → `evidence`** | `wgncvzuo,q dtrsfyheia pblmjxk/;.` | −0.13 % | **4.04** | 0.95 | 8.53 | 0.83 |
| α 0.5 run | `,uoyqjchwx ieatgpdnsr ;/.kzbfmvl` | +0.01 % | 3.89 | 0.74 | 7.06 | 0.83 |
| α 0.75 CMA mean | `cwdlxqyou, gstnvpheia fbmrjzk/;.` | −0.06 % | 3.85 | 1.00 | 8.45 | 0.80 |
| default | `lwdgbzquo. rntsfpheai jxmcvky/;,` | 0 | 4.91 | 0.89 | 6.43 | 0.85 |

Every row dominates the default on both axes. The chosen vector scores best on the α = 0.5
objective (1.502 vs default 1.976) with margin on the tier constraint; α 0.75 run #2 scores
1.496 but sits exactly at ρ 0.80 with SFB 1.08.

## The best cost function: `evidence` (now `default`)

`data/profiles/default.toml` since 2026-10-24 (spread units, the previous scales plus B08's 5.13). In this page "default" means the 2026-10-22 profile, now `balanced`:

| metric | default | evidence | reading |
|---|---:|---:|---|
| B01 SFB | 2.867 | 2.867 | fixed (scale) |
| E05 measured discomfort | 0.622 | **2.08** | comfort becomes the second-largest term |
| B09 pinky–ring | 1.255 | 1.176 | kept: timing +8 % |
| B08 alternation | — | **1.003** | new: timing −8 % per alternating pair |
| T04 / T03 rolls out / in | 0.606 / 0.314 | 0.715 / 0.305 | kept: rolls are fast in context |
| T07 bad redirects | 0.260 | 0.567 | ×2: +3.7 % in context |
| B14 stretch | 0.531 | 0.505 | kept |
| M04 / M02 load | 0.089 / 0.061 | 0.418 / 0.190 | ×3–5 (the search stabilizes finger load with them) |
| M01 effort | 0.163 | 0.321 | ×2: timing +3 % per effort unit |
| B10 row jump | 0.368 | 0.285 | |
| S01 / S02 / S04 SFS | 0.514 / 0.181 / 0.082 | 0.039 / 0.064 / 0.018 | ~dropped: no comfort or timing case beyond B01 |
| T01 alternates, T06 redirects | 0.549 / 0.401 | 0.064 / 0.006 | ~dropped: no context effect; B08 carries alternation |
| B05 / B06 scissors | 0.405 / 0.270 | 0.092 / 0.024 | down: scissor cost is mostly in B14 and B10 |
| B11, M05 | 0.248 / 0.109 | 0.086 / 0.037 | down: no timing effect |

In words: keep SFB and pinky–ring, value hand alternation per bigram and rolls per trigram, and
take measured comfort seriously. Stop paying for SFS, trigram alternation and redirects, which
show no measurable cost here.

## Analysis of the best cost function

**Its optimum** (ansi-30 / english, BLS 25 M, 10/10 seeds identical, 8.074):

```
w g n c v z u o , q
d t r s f y h e i a
p b l m j x k / ; .
```

All vowels and `y h` on the right hand, consonant columns on the left. SFB 0.95 %, SFS 8.5 %,
alternation 69.7 %, rolls 45.1 %, home row 61.4 %.

**Judged against the default optimum and the best known layouts** (`s8_judges.json`; lower is
better except where noted):

| layout | T_A Δ% | T_B Δ% | J_B | J_B+letters | E01 | E05 | E04 | SFB | SFS | M02 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| evidence optimum | **−0.38** | **−0.20** | **0.1736** | **0.0436** | **6.54** | **4.04** | **4.38** | 0.95 | 8.53 | 2.47 |
| default optimum | −0.29 | −0.08 | 0.1792 | 0.0451 | 6.64 | 4.91 | 4.79 | 0.89 | 6.43 | 2.66 |
| galite-reference | −0.11 | +0.08 | 0.1764 | 0.0368 | 6.82 | 5.19 | 5.00 | 1.12 | 6.74 | 2.41 |
| Graphite | −0.11 | +0.09 | 0.1773 | 0.0377 | 6.84 | 5.25 | 5.02 | 1.16 | 6.71 | 2.39 |
| Sturdy | +0.66 | +0.88 | 0.1822 | 0.0371 | 7.60 | 6.54 | 6.18 | 0.89 | 6.21 | 2.53 |
| QWERTY | +5.56 | +5.78 | 0.2064 | 0.0303 | 13.06 | 12.23 | 13.73 | 6.63 | 12.11 | 4.10 |

(T: fitted time model on typist half A / B, % per keystroke, relative to the model's
intercept, so only differences mean anything.)

- **It beats the default optimum on all seven judges.** That includes the four it was never
  tuned toward (T_A, both J_B variants, E01) and Engram's formula E04 (−9 %). The time gain is
  tiny (0.1 %); the comfort gain is large (E05 −18 %).
- **Not a table exploit.** 84 % of its same-hand bigram mass falls on pairs Engram actually
  measured (default optimum 73 %, Graphite 72 %). It moves toward the measured part of E05, not
  the formula fill.
- **What it pays:** SFS 6.4 → 8.5 %, SFB +0.06 pp, home row −2 pp, redirects +0.25 pp. If SFS
  later shows a comfort cost, this is where the profile will move.
- **Search is reliable:** 10/10 seeds at best on ansi-30 english and shai, 9/10 on corne-36
  english, 10/10 on corne-36 shai (`seeds-evidence-*.json`).
- **Corpus robustness is better than default's.** Both evidence winners rank 1st–2nd on all five
  corpora (english, english-cased, shai, mt-450k, reddit). Phase 2's default english winner was
  5th of 6 on mt-450k.
- **Margin over known layouts:** under `evidence` the best cmini layout with the same symbols
  (galite-reference, 11.85) is 32 % behind (default: 17 %). On corne-36 the evidence winner beats
  every known layout under the *default* profile too (10.76 vs Gallium 11.75).
- **Weight robustness** (`gecko sweep`, 30 draws σ 0.5 + one-at-a-time): 60 of 94 vectors land
  in the base family and 25 in the near-tie `cwdlxqyou, gstnvpheia fbmrjzk/;.` family (default:
  55 / 111). Median regret 0.55 (6.8 % of the score; default 1.10, 9 %). Only B01, T04, T03, B09,
  E05 and B14 move the optimum when dropped or doubled. M04, M05, S01, S02, T01 and T06 are
  inactive.
- **Reference ranking** (`compare --robust`): Graphite 1st in 100 % of draws; Sturdy, Semimak,
  Canary, Dvorak share 2–5; QWERTY last. The change from default: **Halmak moves above
  Colemak-DH and Colemak** (25.2 vs 27.2). The community puts Halmak in tier 4. This is the
  price of valuing alternation and comfort; tier ρ stays 0.83.

## Limits

- **Practice confound.** Timing comes from QWERTY typists. Pair coefficients survive three
  practice models, but whole-layout empirical judging does not (QWERTY flips from last to 2nd).
  Coefficients are skilled-typing speed for short English sentences, not fatigue or learning.
- **Comfort rests on one small study.** Engram: 61 participants, 112 left-hand bigrams over 12
  keys, mirrored, formula elsewhere. E05 now carries the second-largest weight, so a larger
  comfort data set is the most valuable next measurement (see [open questions](../OPEN-QUESTIONS.md)).
- **α is a value judgement.** "One spread of time = one spread of comfort" is a convention. The
  Pareto table shows the alternatives; α 0.25 gives a faster optimum with lower SFB (0.83 %).
- **Rugged weight landscape.** Four CMA runs found different families. The chosen vector is the
  best found, not a certified optimum; families within ~0.15 points are ties.
- **ansi-30 only** for the time model (QWERTY key positions). corne-36 results reuse the weights
  without board-specific timing.

## Recommendations

1. Done 2026-10-24: the `evidence` weights replaced `default`; the old default is `balanced`,
   and Phase 2 numbers in `context/evaluation.md` are `balanced` results
   ([decision](../../context/decisions/scoring.md)).
2. Fit a timing-derived key effort table, separating key from letter, e.g. with QWERTZ/AZERTY
   swaps or a letter-frequency design (largest remaining CV gain: +0.17 R²).
3. Measure comfort on all 30 keys, including center columns and SFS, before raising E05 further.
4. Drop or retire inactive weights (S01, S02, T01, T06, M05) only after (3); SFS is the one open
   comfort question.
