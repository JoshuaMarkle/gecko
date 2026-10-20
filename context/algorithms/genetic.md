# Genetic algorithm

**Idea:** a population of layouts evolves through selection, crossover, and mutation.

**Encoding:** permutation (`sym[slot]`), pins excluded from all operators.

**Operators:**
- Crossover for permutations: order (OX), partially mapped (PMX), cycle (CX). CX keeps
  every symbol in a slot from one parent, which suits position-based fitness.
- Mutation: swap (rate per child), occasionally 3-cycle.
- Selection: tournament (size 2–4). Elitism: keep the top few unchanged.
- Diversity: reject duplicates by layout hash.

**Parameters:** `pop_size`, `generations` or time, `crossover`, `crossover_rate`,
`mutation_rate`, `tournament`, `elitism`.

**Pitfalls:** plain GA on permutation problems is usually weaker than SA or tabu per
CPU second. Its value is as a comparison and as the base of the memetic algorithm.
KLO found GA best for improving an existing layout while staying close to it.

## Results

Implemented: `optimize/genetic.rs` (OX on movable slots, tournament, elitism, duplicate rejection, steepest polish of the top 3 at the end). Weakest per CPU second, as expected.

Leaderboard 2026-10-20 (`gecko bench --time 5s --seeds 3`, 20 threads, i9-13900H, default
profile, english corpus, ansi-30; lower is better; Semimak 11.85, Graphite 12.26, QWERTY 53.31):

| algo | mean | best | stdev | M evals/s |
|---|---|---|---|---|
| hc | 8.603 | 8.539 | 0.077 | 3.2 |
| sa | 8.416 | 8.408 | 0.005 | 3.2 |
| ga | 8.890 | 8.566 | 0.249 | 0.9 |

