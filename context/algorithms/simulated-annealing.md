# Simulated annealing

**Idea:** random swaps; always accept improvements, accept a worsening Δ with
probability `exp(−Δ/T)`. T falls over time so the search moves from exploring to refining.

**Schedule:** exponential `T(i) = T0 · exp(−k·i/N)` (carpalx). Set T0 from data: sample
random swaps at start, choose T0 so ~50–80% of worsening moves are accepted. End
temperature so acceptance of worsening moves is < 0.1%. Finish with a steepest-descent
polish.

**Parameters:** `T0` (or `accept0`), `T_end` (or `accept_end`), `steps`, `restarts`,
`reheat` (optional restart from best at higher T), `neighborhood`.

**Pitfalls:** T0 must be scaled to the score range (carpalx author's main warning); too
short a schedule = a noisy hill climb. Many independent runs beat one long run for
finding diverse good layouts (Kühlmak).

**Variants to try:** adaptive cooling (keep acceptance rate on a target curve), parallel
tempering (replicas at several temperatures exchange states).

## Results

Implemented: `optimize/anneal.rs`. `steps = 0` (default) = auto, ≈ 4 restarts per worker in the budget; T updated every 256 steps. Best under the current default profile (spread units, B14, S02, E05): 15.704,
`jkndfzquo, rthcpxseai lmbwvyg/;.` (20 s, seeds 1 and 2 agree). Ranks 1st on english/shai/mt-10k,
2nd on mt-450k, and edges out Graphite on measured comfort (E05 5.22 vs 5.25). The previous
profile's optimum (8.396 then) ranked last of all layouts on E05.

Leaderboard 2026-10-20 (`gecko bench --time 5s --seeds 3`, 20 threads, i9-13900H, default
profile, english corpus, ansi-30; lower is better; Semimak 11.85, Graphite 12.26, QWERTY 53.31):

| algo | mean | best | stdev | M evals/s |
|---|---|---|---|---|
| hc | 8.603 | 8.539 | 0.077 | 3.2 |
| sa | 8.416 | 8.408 | 0.005 | 3.2 |
| ga | 8.890 | 8.566 | 0.249 | 0.9 |

