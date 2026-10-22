# Tabu search

**Idea:** steepest descent that is allowed to take the best non-improving move, with a
short-term memory (tabu list) forbidding recently reversed moves so it doesn't cycle.
Robust tabu search (Taillard) is a classic strong method for QAP.

**Details:**
- Move = swap (a, b). Mark "put a back in slot s" tabu for `tenure` iterations.
- Aspiration: a tabu move is allowed if it gives a new best.
- Randomized tenure in a range (robust tabu) avoids cycling.
- Keep the full Δ matrix of all swaps and update it incrementally after each move
  (standard QAP trick) instead of recomputing all swaps.

**Parameters:** `tenure_min`, `tenure_max`, `iterations`, `restarts`, diversification
interval.

**Pitfalls:** incremental Δ-matrix updates are fiddly with trigram terms; verify with the
delta == full property test.

**Implementation** (`optimize/tabu.rs`, `--algo tabu`): Taillard's RoTS. A move is tabu
only if *both* symbols would return to slots they left within their tenure (drawn per move
from `[0.9 m, 1.1 m]`, m = movable symbols). Aspiration: a new best for the restart, or both
placements unused for `5 m²` iterations (forced diversification). Restart from a random
layout after `100 m` iterations without improvement; each restart's best is polished by
steepest descent. Params (`TabuParams`): `tenure_min`, `tenure_max`, `diversify`,
`stagnation`, `refresh`. The Δ matrix holds the linear part of each swap delta; the finger-load
part is added during the scan (see [decisions](../decisions/search.md) 2026-10-21). One iteration on ansi-30 ≈ 0.26
ms on one thread (≈ 3× cheaper than a full rescan); evals = swaps scanned.

## Results

Leaderboard 2026-10-21 (`gecko bench --algos sa,bls,tabu --seeds 10 --time 5s`, 20 threads,
i9-13900H, default profile, english corpus, ansi-30; lower is better; hits = seeds reaching
the best):

| algo | mean | best | stdev | hits | M evals/s |
|---|---|---|---|---|---|
| sa | 15.766 | 15.704 | 0.065 | 5/10 | 4.8 |
| bls | 15.714 | 15.704 | 0.030 | 9/10 | 4.8 |
| tabu | 16.104 | 15.704 | 0.203 | 1/10 | 16.7 (swaps scanned) |

**Cross-check:** tabu independently reaches SA's optimum, the same layout
`jkndfzquo, rthcpxseai lmbwvyg/;.` = 15.704 (once in the 5 s bench; `--time 20s`, run
`20261021-044457-tabu-aa71398b`). No method has found anything better, so it is very likely
the optimum for this profile.

Tabu is the weakest of the three at 5 s: it makes one move per iteration, ~20 k iterations
per worker in 5 s, while SA makes millions of cheap random moves. Parameter variants (tenure
0.3–0.6 m or 1.5–2 m, no restarts, restart after 20 m or 300 m, no diversification) were all
no better than the defaults (3 seeds: defaults 15.88, variants 15.90–16.47). Not a default candidate; useful
as an independent check. A faster trigram update in `delta_shift` would help most.
