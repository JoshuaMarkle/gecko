# Finding the Best Keyboard Layout: Search, Bounds and Certificates

*Skeleton. Focus paper 3. Topics [T2, T10](../topics.md). Status: eight optimizers and bounds exist;
exact windows (#43) and the final "best layout" runs do not.*

## Abstract (draft)
We ask what the best layout is under a fixed cost function, and how sure we can be. We compare
eight optimizers (hill climbing, SA, breakout local search, tabu, GA, memetic, MAP-Elites,
NSGA-II) under equal budgets on a quadratic assignment problem with sparse trigram terms, give a
Gilmore–Lawler lower bound with dual ascent (90.3 % gap at 30 free keys, 1.7 % on the bigram part
at 10 free keys), and certify solutions window by window with exact search [TODO]. We report the
best layouts found for [TODO: ANSI-30, Corne-36, with layers] with their per-metric breakdowns,
compare them with Graphite, Sturdy, Semimak and Canary, and state how far from optimal they can be
shown to be.

## Research questions
1. RQ1 Which optimizer reaches the best layout fastest, by budget (anytime curves, performance
   profiles, time-to-target over 30 seeds)?
2. RQ2 How large is the provable gap; how does it shrink with fewer free keys?
3. RQ3 Are heuristic optima window-optimal (every k-key subset, k = 6..12, solved exactly)?
4. RQ4 What is the best layout per board/corpus/profile, and how does it differ from the
   references on each metric?
5. RQ5 Does the answer change with symbols, layers and magic keys (code corpus, 36-key board)?

## Hypotheses
H1 BLS and SA tie at long budgets, BLS leads at short ones (matches the 2 s leaderboard).
H2 ≥ 95 % of windows with k ≤ 10 are exactly optimal for the best heuristics. H3 misses are
trigram-dominated. H4 the global best equals the multi-seed consensus best. All can fail.

## Method
- Algorithms under the `Optimizer` trait; `gecko bench --seeds 30` over a budget grid.
- Bound: QAP form (`Scorer::qap_terms`), trigrams relaxed to pairs, GL plus 20 dual-ascent
  iterations; pins as constants.
- Exact: branch and bound with the bound; MIP/CP-SAT oracle (#43) for larger windows.
- Reproducibility: every cited layout comes from a run verified with `gecko verify`.

## Existing evidence
2 s leaderboard (10 seeds): BLS 17.616 ± 0.081, SA 17.787 ± 0.145, memetic 18.027 ± 0.352, tabu
18.960 ± 0.256 (`bench/memetic-2026-10-21/`); SA 20 s optimum 15.704 under the current profile with
seeds agreeing; bound table and ascent convergence 4.49 → 5.81 (`context/algorithms/bounds.md`);
neighbourhood study (`context/algorithms/README.md`).

## Figures
F1 anytime curves with CIs · F2 performance profile · F3 gap vs free keys · F4 window-optimality
share by algorithm and k · F5 best layout keyboard + per-metric waterfall vs Graphite · F6 best
layouts across boards and corpora.

## Threats
"Best" is relative to the cost function (see [paper 2](02-cost-function.md)); weak bound at full
size; machine-dependent timings; one language unless the multilingual runs are added.

## Needed
#43 oracle; window driver; 30-seed budget grid; final runs saved as curated `bench/` directories.
