# Focus topics

Questions GECKO is built to answer, in the order they become answerable. For each: the question,
what we already know from the repo (with source), the hypothesis, the experiment, the figures that
would make the result convincing, and what blocks it. "Have" means a measured number exists today;
everything under **Hypothesis** is a prediction, not a result.

| # | Topic | Core claim to test | Ready? | Paper |
|---|---|---|---|---|
| T1 | Objective sensitivity | The "best layout" is a region, not a point; a few weights decide which | Yes | [P1](drafts/P1-objective-sensitivity.md) |
| T2 | Certified optimality | Optimizers are near-optimal; bounds can say how near, at least on sub-problems | Mostly | [P2](drafts/P2-certified-optimality.md) |
| T3 | Landscape structure | Layout space is a big-valley with few funnels; most good layouts share a skeleton | Yes | [P3](drafts/P3-landscape.md) |
| T4 | Trade-off atlas | Every popular layout sits on a small Pareto/QD front; the front is mapped | Yes | [P3](drafts/P3-landscape.md) |
| T5 | Metric gaming and generalization | Optimizing a proxy beats real comfort measures unless the proxy is held out | Yes | P1 §Goodhart |
| T6 | Board geometry co-design | Which board features (stagger, splay, thumb arc, tent) buy the most comfort | Needs #54, #59 | [P4](drafts/P4-geometry-codesign.md) |
| T7 | Hand-size robustness | A layout/board tuned for one hand fails on others; robust designs cost little | Needs #103 | P4 |
| T8 | Learned costs | Timing and preference data calibrate the objective better than hand weights | Needs #31, #61 | [P5](drafts/P5-learned-costs.md) |
| T9 | Symbols, layers, code | Layer and adaptive-key designs change the optimum for programmers | Partly | new |
| T10 | Search benchmark | On a QAP with trigram terms, which metaheuristic wins at which budget | Yes | software paper §7 |
| T11 | Community agreement | Which metrics predict what ~4,000 community layouts keep and like | Yes | P5 / P1 |
| T12 | Idea → printable board | An end-to-end hand → layout → board → STL pipeline | Far (J4, J6) | demo |

---

## T1 · Objective sensitivity: is there a best layout?

**Question.** Given a fixed metric set, how much does the optimum move as the weights move?
Which weights are decisive and which are irrelevant?

**Have.** SA reaches the same optimum from every seed in 10–30 s (`context/algorithms/simulated-annealing.md`),
so the search is not the bottleneck; the objective is. Changing the profile changed the winner:
the earlier optimum (8.396) ranked last of all references on the independent comfort judge E05
once the profile changed. B11 at weight 1 swaps Halmak and Colemak in the reference order
(`context/objective.md`).

**Hypothesis.** (H1) Optima form a small number of clusters (roll-heavy, alternation-heavy,
pinky-averse); weights move you between clusters, rarely within one. (H2) Two or three weights
(SFB, scissor/stretch, trigram redirect) account for most of the variation in the optimum.
(H3) A layout optimal for one random weight vector stays within X % of optimal for most others
(the "robust core"), where X is measured, not assumed.

**Experiment.**
1. Sample N = 1,000 weight vectors (Latin hypercube over log-scaled weights, spread-normalized).
2. For each, run SA (30 s, 3 seeds) → optimum; or archive once with `gecko explore` and re-rank
   under every weight vector without new search (the archive stores raw metric vectors).
3. Cluster optima by Hamming / finger distance; fit a sensitivity model (rank correlation of
   each weight with embedding coordinates; Sobol indices on score and on layout distance).
4. Report the consensus layout (slots where >90 % of optima agree) and the contested slots.

**Figures.**
- F1: 2-D embedding (MDS/UMAP on finger distance) of all optima, coloured by cluster, reference
  layouts overlaid. *Shows the clusters and where Graphite/Sturdy/Semimak/Canary fall.*
- F2: Sobol / rank-correlation bars per weight (which weights matter).
- F3: Consensus keyboard: each key coloured by the share of optima putting the same symbol there.
- F4: Regret matrix: layout optimal for weight vector *i* scored under weight vector *j*.
- F5: Weight-sweep ribbon for one weight: layout "phase diagram" (which clusters appear when).

**Blockers.** A weight-sweep driver (a loop over `gecko explore` / `gecko profile` today; a small
`gecko sweep` command would make it reproducible). Everything else exists.

**Venue.** Workshop or arXiv + blog; the main selling point is the figure set and the open tool.

---

## T2 · Certified optimality

**Question.** How far from optimal are the layouts that heuristics find?

**Have.** Gilmore–Lawler bound with dual ascent, reported after every `optimize` run
(`context/algorithms/bounds.md`). Measured gaps on ansi-30 / english / default:

| free symbols | total gap | pair-part gap |
|---|---|---|
| 30 | 90.3 % | 70.3 % |
| 16 | 24.5 % | 12.1 % |
| 10 | 12.9 % | 1.7 % |

So: the bound is useless for the full 30-key problem, informative with ≤ 16 free keys, and the
pair-part is certified within 1.7 % at 10 free keys. This is an honest negative-then-positive
result, and the paper should say exactly that.

**Hypothesis.** (H1) With a MIP/CP oracle (#43) on windows of ≤ 12 free keys, SA's optimum is
exactly optimal in nearly all windows. (H2) "Window-wise optimality" (no window of k free keys
improves) implies near-global quality: the k-swap neighbourhood is a strong certificate even
without a global bound. (H3) Where SA is not optimal, the miss is in trigram-dominated terms.

**Experiment.**
1. From the best layout of many runs, free every k-subset of keys (k = 6..12) and solve exactly
   (branch & bound with the GL bound; MIP oracle for larger k). Count improvements.
2. Compare the certified gap against the 10-free-key and 16-free-key rows; extend to k = 20 with
   the oracle.
3. Tighter relaxations (RLT-2 / SDP) for the pair part only, to see how far the full bound can be
   pushed. (Currently not planned; the paper can mark it as future work.)

**Figures.** Gap vs number of free keys (log scale); fraction of windows where SA is optimal;
bound convergence per dual-ascent iteration (data exists: 4.49 → 5.81 over 50 iterations);
scatter of heuristic vs exact on random sub-problems.

**Blockers.** #43 (MIP oracle). Without it, only the brute-force/B&B windows k ≤ 10 are exact.

**Venue.** Optimization / EvoCOP-style short paper; the QAP-with-sparse-trigrams structure is
the reusable contribution.

---

## T3 · Landscape structure

**Question.** What does the space of keyboard layouts look like to a local search?

**Have.** `gecko explore` writes an atlas: random-sample score distribution, random-walk
autocorrelation / correlation length, a local-optima network from Breakout Local Search with
exclusion restarts, per-optimum raw metric vectors (`context/algorithms/explore.md`). The space
is 30! ≈ 2.65 × 10³² layouts, 435 swap neighbours each (`research/studies/search-space-visualization.md`).

**Hypothesis.** (H1) The landscape is a big valley: fitness-distance correlation among local
optima is strongly positive. (H2) There are a handful of funnels (a few "families" of near-optimal
layouts that differ in which hand or finger takes the vowels). (H3) The number of distinct local
optima within 1 % of the best is much larger than the number of reference layouts people use.
Report the measured values; some may fail.

**Experiment.** Run `gecko explore` for 10 min × 10 seeds per (geometry, corpus, profile). Compute
fitness-distance correlation, funnel decomposition (basin sizes via kick-edge graph), distance
distribution between best optima, mirror-symmetry fraction, and how pins change the structure
(pin ETAOIN-like sets).

**Figures.**
- F1: Local-optima network (nodes = optima, size = hits, colour = score, layout by force-directed
  with Hamming distance).
- F2: Fitness-distance plot against the best optimum, with the reference layouts as marked points.
- F3: Random-score density vs optimum scores (how rare good layouts are).
- F4: Correlation-length / autocorrelation curve of the random walk.
- F5: Consensus keyboard and "contested slot" map across the top 100 distinct optima.

**Blockers.** None. Web Explore view (#100) is a bonus for interactive supplements.

---

## T4 · Trade-off atlas (quality diversity and Pareto)

**Question.** What can you get if you refuse to pay a given cost? What does "more rolls" or "less
index load" cost?

**Have.** MAP-Elites over behaviour descriptors (`gecko qd`, roll ratio, index/pinky load, hand
balance, any metric) and NSGA-II (`context/algorithms/nsga2.md`, `map-elites.md`).

**Hypothesis.** (H1) The Pareto front between SFB and roll ratio is short and convex; most
reference layouts lie behind it. (H2) The cost of a 5-point shift in index load is small at the
front and grows sharply near the extremes. Numbers pending.

**Experiment.** MAP-Elites at 16×16 on (roll_ratio, index_load); NSGA-II on (comfort, speed
proxy, symbol-friendliness) with a hypervolume-vs-budget curve; place all references and the
~4,000 cmini community layouts on the same map.

**Figures.** QD heatmap (score per cell) with references; Pareto fronts per objective pair;
"what it costs" curves; community density over the front (are people where the front is?).

**Blockers.** None for the scorer side. Web views F1 (#38) are optional.

---

## T5 · Metric gaming and generalization (Goodhart in layout design)

**Question.** When you optimize a proxy, how much of the gain is real?

**Have.** Optima under one profile can rank last under an independent judge (above). The default
profile now includes a measured-comfort term (E05); a held-out-corpus score exists (L06, #44)
and ranking tests cover reference layouts.

**Hypothesis.** (H1) The gap between proxy score and judge score grows with search effort
(longer search → more exploitation) for profiles lacking the judge term. (H2) Optimizing on one
corpus and evaluating on another (English → French, English → code) loses far less for
corpus-robust metrics (geometry-based) than for trigram metrics.

**Experiment.** For a fixed budget grid (1 s … 10 min) optimize profile A, evaluate profiles B, C,
and judge E05 on the result; repeat with train / held-out corpus pairs across english, shai,
reddit, mt-10k, french, code.

**Figures.** Proxy-vs-judge scatter along the search trajectory (does the judge turn around while
the proxy still falls?); cross-corpus transfer matrix; per-metric "overfit" bars.

**Blockers.** None.

---

## T6 · Board geometry co-design

**Question.** Which physical board features buy the most comfort, and do they change which layout
is best?

**Have.** Parametric boards (stagger, splay, split spread/angle, thumb arcs), CMA-ES for
continuous parameters (G5), cost tables rebuilt in 0.65 ms per moved key (`gecko perf`), a hand
model with finger IK for reach, press angle, wrist posture (`context/hand-model.md`). Boards:
ansi-30, ortho-30, corne-36, sweep-34, lily58, kyria, ergodox, dactyl-manuform.

**Hypothesis.** (H1) Most of the comfort gain over ANSI comes from a few features (column stagger,
thumb cluster), and the rest are small. (H2) A jointly optimized layout+board beats the best
layout on the best stock board by a measurable margin that sequential optimization (layout first,
then board) misses.

**Experiment.** Ablation: start from ortho-30; add one feature at a time (column stagger → splay →
split → thumbs → tent), re-optimizing layout each time. Then bilevel search (#54) vs sequential.
Compare results across stock boards at equal budget.

**Figures.** Ablation waterfall (cost vs feature); before/after board overlay; layout×board
interaction table (does the best layout change with the board?); Pareto of comfort vs
manufacturability when #73 lands.

**Blockers.** #54 bilevel, #59 3D params, #66 3D collision, #63/#52 already done.

---

## T7 · Hand-size robustness

**Question.** How much does the optimum depend on hand size, and can one design serve a range?

**Have.** Hand model takes dimensions or a size preset in the profile (`[hand]`); G01 / G03 / G05
change with hand.

**Hypothesis.** A layout/board tuned for the median hand loses noticeably on the 5th and 95th
percentile; a minimax design over a hand-size range costs little on the median.

**Experiment.** Optimize per hand-size percentile; cross-evaluate (regret matrix); minimax and
mean-optimized designs (#103).

**Figures.** Regret matrix; score vs hand size for each design; parametric board morph by hand
size (the visual selling point).

**Blockers.** #103, and anthropometric data cited properly (`research/OPEN-QUESTIONS.md`).

---

## T8 · Learned costs from human data

**Question.** Can public timing and preference data replace hand-tuned weights?

**Have.** Engram pairwise comfort (E05, fit with Bradley–Terry), cmini likes (`gecko profile fit`:
held-out pairwise accuracy 53.78 % vs default profile 51.96 % vs E05 49.40 %; `research/experiments/2026-10-21-profile-ranking.md`).
That result is small, and the write-up calls it so.

**Hypothesis.** Timing-based costs (E01, from the 136M keystrokes data, #31) will predict
inter-key intervals far better than the hand-set pair costs; combining timing and preference
shrinks disagreement between the two.

**Experiment.** Regress IKI on geometric pair features (hand, finger pair, dx, dy, row change)
per typist group; evaluate on held-out typists; compare to hand-tuned cost and E05; use the
model as a profile and re-run T1/T5.

**Figures.** Predicted vs observed IKI by finger pair (heatmap pair); weights learned vs default;
reliability of pairwise accuracy with bootstrap CIs (needed: current accuracy has no CI).

**Blockers.** #31, #35 (done), #61, #46 (tapping test for personal data).

---

## T9 · Symbols, layers and code

**Question.** What changes in the best layout when you type code, use layers and adaptive keys?

**Have.** Thumb tokens, hold / one-shot / toggle layers, Shift, paired brackets (Y03), home-row
mod misfire risk (Y04), `code` corpus, magic / repeat keys (`context/algorithms/magic-rules.md`).

**Hypothesis.** Code-aware profiles move bracket and operator symbols off the pinkies and pair
them; a repeat/magic key lowers SFB by a measurable fraction for a modest cost. Numbers pending.

**Experiment.** Optimize 36-key boards with and without layers on `code` vs `english`; sweep the
magic-key budget; report per-metric deltas.

**Figures.** Layer view of the best layouts; SFB vs number of magic rules; cross-corpus table.

**Blockers.** Combos / chords (#91), number/navigation layer (#30 done), HRM misfire calibration.

---

## T10 · Search benchmark

**Question.** Which metaheuristic is best for a QAP whose cost has sparse trigram terms, and how
does the answer depend on the time budget?

**Have.** `gecko bench` with SA, BLS, tabu, memetic, HC, GA; 10-seed leaderboard
(`bench/memetic-2026-10-21/leaderboard.json`): at 2 s on 4 threads, BLS 17.612 mean (best 17.561),
SA 17.787, memetic 18.027, tabu 18.960. Neighbourhood study: bigger moves give better optima per
descent but 3–60× more evaluations (`context/algorithms/README.md`).

**Experiment.** Budget grid (0.5 s … 5 min), 30 seeds, time-to-target, performance profiles
(Dolan–Moré), plus an instance family (corne-36, lily58 with symbols and layers) to test whether
rankings carry over.

**Figures.** Performance profile; time-to-target ECDF; anytime curves with confidence bands.

**Blockers.** None; belongs in the software paper unless it grows.

---

## T11 · Community agreement

**Question.** Which metrics line up with what the community keeps and likes?

**Have.** ~4,100 cmini layouts with likes, importers for Oxeylyzer / genkey / cmini formats,
rank-agreement tooling (`context/algorithms` + `community` module).

**Hypothesis.** Single metrics predict likes weakly; the total score predicts them better than any
one metric but modestly; popularity is confounded by exposure and age.

**Experiment.** Rank correlation of likes with each metric and with profile totals, with
bootstrap CIs; layout-duplicate clusters (mirror groups); time/author controls if available.

**Figures.** Metric-vs-likes forest plot; density of community layouts over the T3/T4 maps.

**Blockers.** Confidence intervals and a pre-registered analysis plan (fix the test before looking).

---

## T12 · Idea → printable board (demo paper)

Hand → layout → parametric board → collision → STL, one reproducible walkthrough (#77). Only
worth writing after #66, #73, #75. Save for a system demo track.

---

## What makes a result convincing here

- **Effect sizes with intervals**, over seeds and over corpora, not a single best number.
- **A baseline people trust**: published reference layouts (Graphite, Sturdy, Semimak, Canary,
  Colemak-DH, Dvorak, Workman, Halmak, QWERTY) scored in the same pipeline, plus the published
  SFB numbers as a validity check (QWERTY 6.63 % SFB today).
- **An independent judge** that was not optimized (E05 from Engram; later E01 timing, and a user
  study).
- **Exactness where possible** (small windows solved to optimality), honesty where not (large gaps).
- **Open reproduction**: every figure regenerates from `runs/` + a seed; `gecko verify` passes.
- **Negative and small results reported**, with the reasons.

## Gaps to fill before submission

1. A real user study: even 10–20 participants, a typing test on two or three layouts, to see
   whether the score ordering shows up in comfort or speed. Without this every claim stays
   "predicted comfort".
2. Confidence intervals on pairwise accuracy and rank correlations.
3. A `gecko sweep` (weight sweeps) and a figure-generation script per paper.
4. Related-work coverage: the 2023–2026 survey (`research/studies/literature-survey-2023-2026.md`)
   found the literature thin; the related-work sections must cite what exists and say what is
   not covered.
