# Cost-function validity: timing, comfort, weights

| | |
|---|---|
| Date | 2026-10-22 |
| Question | Is the default profile right: does it match measured typing cost, are its conclusions robust to its weights, and do optimized layouts improve on independent judges? |
| Answer | Valid at the coarse level for touch typists (timing partial ρ 0.65); fine structure (SFS, B14, spread units, the optimum's identity) is convention, not evidence |
| Command | `bench/cost-validity-2026-10-22/README.md` (costlab tool + scripts) |
| Data | ansi-30 / english / default; 10 references, 1,904 cmini layouts, 3,000 random; Aalto 136M mirror (68k touch typists); Engram BT comfort |
| Artifacts | `bench/cost-validity-2026-10-22/results.json`; paper `docs/papers/cost-validity/cost-validity.pdf` |

## Results

- **Timing.** Folded pair cost vs median relative IKI, controlling log bigram frequency: partial ρ
  0.65 (touch ≥ 70 WPM, 18,504 people), 0.50 (touch, 68,063), 0.26 (1–4 fingers, control).
  First-letter-grouped CV R² 0.27 → 0.69 (fast touch). SFB costs +0.65 median IKI; scissors
  +0.11, pinky–ring +0.08, adjacent finger +0.08; in vs out rolls equal; SFB distance adds nothing.
- **Trigrams.** Third-keystroke time, controlling its bigram: SFS +0.5 % (95 % CI −1.5 to +2.6),
  rolls −5.7 % (−7.2 to −4.2), bad redirects +3.8 % (+0.1 to +8.2) vs alternation.
- **Comfort (Engram, 200 measured pairs).** Non-E05 pair cost ρ 0.49; effort −0.72, row jump −0.51,
  stretch −0.42, SFB −0.06.
- **Time judge** (relational timing model on English bigrams): Spearman with score 0.94 random,
  0.84 cmini, 0.93 references, 0.44 good layouts. Default optimum ≈ Graphite on time (2.1377 vs
  2.1370) despite a 4.1-point score lead. NNLS weights reproducing it (ρ 0.997): SFB 25 % (default
  55 %), E04 15, B09 12, B05 11, T01 11, B10 7, B14 0.3 (default 13).
- **Decomposition.** SFB explains 80 / 70 / 61 % of score variance (random / cmini / references);
  among good layouts B14 35 %, E05 14 %, SFB 12 %; SFB–B14 r = −0.64; participation ratio 3.4 of 16.
- **Robustness.** Every classic beats QWERTY in > 99 % of log-normal weight draws; every modern
  beats every classic in 97 % (σ 0.5), 73 % (σ 1), 6 % with equal expected weights. Only 2 of 90
  reference orderings are Pareto-dominant. Scales computed on good layouts keep 6 of the top 20.
- **Sweep.** Two optimum families 0.004 points apart (`xvndqjguo.`… SFB 0.51 / B14 2.38;
  `qkndfzjuo.`… 0.64 / 1.88). Only dropping SFB costs much (11.4 points); B03, M04, S01, S02, T01, T06
  are inactive at the optimum. Corpus transfer among prose corpora ≤ 0.43 points; mt-450k 2.9–4.9.
- **Wider sweep.** 150 draws at σ 0.5: 73 distinct optima, median 15 keys moved, regret 0.19
  (p90 1.18); 100 at σ 1.0: 94 distinct, 25 keys, 1.34 (3.64). None is more than 0.2 % faster on
  the time judge than the default optimum.
- **Profiles.** Time-fitted profile (with S01/S02 kept): fastest on the time judge (0th
  percentile of cmini), home row 61.4 %, default score 19.35 (Graphite 21.68). `learned` is
  exploited by its optimum: home row 8 %, SFB 5.4 %, default score 68.3. Full table: paper Table 4.

## Implications

Candidate profile changes (not applied; need a decision and the ranking test): weight B05, B09,
B10, T01; lower B14; drop B03; treat S01/S02 as comfort assumptions; compute scales over good
layouts. Present optima as near-tie families. Open: a skipgram comfort measurement
([open questions](../OPEN-QUESTIONS.md)).
