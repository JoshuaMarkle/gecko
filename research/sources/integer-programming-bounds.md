---
group: optimization
kind: paper
read: 2026-10-20
status: secondary
feeds: [B6, B7]
take: Exact bounds to certify optima; AZERTY standard (MPI pages unreachable)
---

# Exact methods and lower bounds: Karrenbauer, Oulasvirta, Feit, John

- Karrenbauer & Oulasvirta, "Improvements to keyboard optimization with integer programming",
  UIST 2014, doi:10.1145/2642918.2647382. Project: https://resources.mpi-inf.mpg.de/keyboardoptimization/UIST2014
- John & Karrenbauer, MOTOR 2019, iterative lower bounds for sparse QAP (exact title **verify**):
  https://resources.mpi-inf.mpg.de/keyboardoptimization/MOTOR2019/MOTOR2019.pdf
- Feit, "Assignment Problems for Optimizing Text Input", PhD thesis, Aalto 2018:
  https://aaltodoc.aalto.fi/items/5c7322e4-b490-419d-9a1a-d848f5a851d6
- Feit, Nancel, John, Karrenbauer, Weir, Oulasvirta, "AZERTY amélioré: computational design on
  a national scale", CACM 2021 (the French standard NF Z71-300) **(verify citation details)**.

(MPI pages were blocked from our env; details below are from abstracts and the thesis record.)

## Key ideas

- Write letter assignment as an integer program (linearized QAP) and solve with
  branch-and-bound. Gains: better layouts than random search on their cases, and **certified
  bounds** on how far a solution is from optimal.
- Sparse QAP (most symbol pairs have tiny frequency): an iterative method gives a hierarchy of
  lower bounds plus heuristic solutions.
- Objectives as assignment-problem terms: motor performance, ergonomics, learnability
  (familiarity with a reference layout), mnemonics (related symbols nearby). The French
  standard optimized placement of ~100+ special characters this way.
- The thesis concludes that existing models do not capture the diverse finger strategies of
  real typists (`feit-how-we-type.md`), so "the optimal physical layout" is still open.

## How to use in GECKO

- **Certify results:** for the bigram-only part of a profile, a Gilmore–Lawler bound or an LP
  relaxation tells whether SA's best is near-optimal. GECKO's SA now finds the same best
  layout from different seeds (`context/algorithms/simulated-annealing.md`), so a bound is the
  remaining way to prove it.
- Run a MIP solver offline (HiGHS, CP-SAT) on small sub-problems: e.g. fix the vowel block,
  solve the consonant assignment exactly. Useful as a test oracle for optimizers.
- Familiarity and mnemonic terms are linear/quadratic assignment terms (L02, L03), so they fold
  into C1/C2 like any other metric.
- Symbol layers (phase 5) are the French-standard problem: many symbols, sparse frequencies,
  strong grouping constraints. Read the CACM paper before designing Y-metrics.
