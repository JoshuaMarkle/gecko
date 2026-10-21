# Hill climbing

**Idea:** repeatedly apply improving swaps until none exist (a local optimum), then restart.

- **First-improvement:** try swaps in random order, take the first improving one. Fast per step.
- **Steepest-descent:** evaluate all N(N−1)/2 swaps (435 for 30 keys), take the best. With
  delta evaluation this is cheap and deterministic.
- **Random restarts** across threads; keep the top-K distinct local optima.

**Parameters:** `mode` (first|steepest), `restarts`, `neighborhood`.

**Pitfalls:** gets stuck in the first local optimum; quality depends almost entirely on
restarts. Useful as the local-search step inside memetic and as a final polish after SA.

## Results

Implemented: `optimize/hill_climb.rs` (steepest descent; restart 0 starts from the start layout).
`--moves cycle3,depth2` improves the 5 s bench mean 16.02 → 15.90 (within ~1σ); see
[README.md](README.md#neighbourhoods-vs-plain-swap-2026-10-21-issue-8).

Leaderboard 2026-10-20 (`gecko bench --time 5s --seeds 3`, 20 threads, i9-13900H, default
profile, english corpus, ansi-30; lower is better; Semimak 11.85, Graphite 12.26, QWERTY 53.31):

| algo | mean | best | stdev | M evals/s |
|---|---|---|---|---|
| hc | 8.603 | 8.539 | 0.077 | 3.2 |
| sa | 8.416 | 8.408 | 0.005 | 3.2 |
| ga | 8.890 | 8.566 | 0.249 | 0.9 |

