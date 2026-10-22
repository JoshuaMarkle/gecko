# Existing tools compared

One row per tool: how it judges a layout, how it searches, what GECKO took. Details and
numbers live in the linked source summaries; this page is only the comparison.
Written 2026-10-20, trimmed 2026-10-21.

| Tool | Lang | Judging model | Search | Take for GECKO | Summary |
|---|---|---|---|---|---|
| carpalx | Perl | Triad effort: base + penalty + stroke path; no SFB term | SA, exponential cooling | Effort profile (T11); T0 scaled to score range | [carpalx](../sources/carpalx.md) |
| Oxeylyzer 1/2 | Rust | SFB/SFS by distance and finger, geometric stretch, trigram classes, heatmap, max finger use | Greedy / depth-N / SA, pins, top-1000 trigrams | B14 stretch, trigram weights, depth-2 swaps | [oxeylyzer](../sources/oxeylyzer.md) |
| genkey | Go | Finger speed (SFB + skipgram decay, per-finger KPS), LSB, index balance | Random starts → greedy → multi-swap | Skipgram decay, personal dexterity, worst words | [genkey-semimak](../sources/genkey-semimak.md) |
| Keyboard Layout Optimizer | Rust | ArneBab/Neo lineage: movement-pattern table, key-cost matrix, trigram irregularity, learnability, word metrics | SA, GA | B11 table, scissor direction rule, modifier n-gram expansion | [klo-dariogoetz](../sources/klo-dariogoetz.md) |
| evolve-keyboard-layout (ArneBab) | Python | Neo community metrics | Evolutionary | Origin of KLO metrics; nothing separate to port | — |
| Kühlmak | Rust | Same-hand bigram classes, disjoint bigrams, WLSB, RSS balance, soft targets, constraints | Threshold SA with adaptive fidelity; rank-sum selection | RSS balance, soft targets, rank-sum, adaptive fidelity | [kuehlmak](../sources/kuehlmak.md) |
| keynergy | Rust lib | Library for custom metrics and n-gram stats | n/a | API ideas only | — |
| Halmak GA | JS | Simulated typing under an effort budget | GA | Cautionary example of a thin objective | [halmak](../sources/halmak-keyboard-genetics.md) |
| Engram / optimize_layouts | Python | Measured pairwise comfort tables, Pareto over 5 objectives | Exhaustive / branch-and-bound over phases | Independent judge profile; calibration method | [engram](../sources/engram-preference-study.md) |
| Feit / Oulasvirta / Karrenbauer | — | Assignment-problem objectives: ergonomics, familiarity, mnemonics | Integer programming, lower bounds | Certificates; symbol-layer model (French standard) | [integer-programming](../sources/integer-programming-bounds.md), [dellamico](../sources/dellamico-single-finger-qap.md) |
| Cosmos, Dactyl-Manuform, ergogen | TS / Clojure / JS | Physical geometry generation, no layout judging | Manual / parametric | Geometry parameters, hand scans, IK, collision checks | [cosmos](../sources/cosmos-keyboards.md), [ergogen-dactyl](../sources/ergogen-dactyl.md) |

## Gaps among the tools read

- None scores a layout on **arbitrary 3D geometry** with a hand model; the analyzers assume a
  fixed 2D board with a fixed finger map. This is GECKO's niche (`context/goals.md`).
- None optimizes geometry and layout together (bilevel): Cosmos and ergogen generate geometry
  without judging typing, the analyzers judge typing without generating geometry.
- Only the Aalto integer-programming work certifies optimality; GECKO has Gilmore–Lawler
  bounds (`context/algorithms/bounds.md`).

## Unverified

- carpalx default `k` values come from a mirror's config files (see
  [carpalx](../sources/carpalx.md)); primary site unreachable.
- `keynergy` and `evolve-keyboard-layout` were not read in depth; their rows rest on docs.rs and README summaries.
