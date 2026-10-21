# Algorithms

All discrete optimizers implement one trait in `gecko-core::optimize`:

```rust
trait Optimizer {
    fn init(&mut self, scorer: &Scorer, start: Option<&Layout>, rng: &mut Rng);
    fn step(&mut self, scorer: &Scorer, rng: &mut Rng) -> StepInfo; // a batch of work
    fn best(&self) -> &[Scored<Layout>];                              // top-K, de-duplicated
}
```

Implemented: HC, SA, GA, BLS, tabu, Explore, MAP-Elites (`gecko-core/src/optimize/`, `explore/`). The runner handles time budgets, threads/restarts, progress events, and persistence,
so algorithms contain only search logic. All moves respect pins.

| File | Kind | Status | Role |
|---|---|---|---|
| [hill-climbing.md](hill-climbing.md) | Local search | Phase 2, first | Baseline + inner local search for others |
| [simulated-annealing.md](simulated-annealing.md) | Local search | Phase 2 | Main workhorse (carpalx, KLO, Kühlmak use it) |
| [genetic.md](genetic.md) | Population | Phase 2 | Comparison; staying near a start layout |
| [tabu-search.md](tabu-search.md) | Local search | Done | Strong for QAP; cross-checks SA's optimum |
| [memetic.md](memetic.md) | Hybrid | Phase 2–3 | GA + local search, island model |
| [nsga2.md](nsga2.md) | Multi-objective | Phase 3 | Pareto fronts |
| [explore.md](explore.md) | Local search + archive | Done | BLS; explore = probe + BLS with exclusion restarts, writes an atlas of local optima |
| [map-elites.md](map-elites.md) | Quality diversity | Done | Grid over behaviour axes (roll ratio, index load, …), best layout per cell; `gecko qd` |
| [geometry-search.md](geometry-search.md) | Continuous | Phase 6+ | Key positions/rotations; bilevel with layout |

Shared neighborhoods (`optimize/moves.rs`): every `steepest_descent` runs swaps to a local
optimum, then, if enabled in `Params.moves` (`--moves columns,cycle3,depth2|all`), tries the
best same-finger column exchange, then the best 3-cycle, then the best pair of swaps (depth 2,
first swap among the `depth2_width` = 32 least-worsening, 0 = all); any improvement resumes
the swap descent. All moves are swap sequences between movable symbols (pins respected,
delta-tracked, no allocation). Default: swap only (bench below). Hand-half mirroring: later.
Every algorithm is compared with `gecko bench` before it becomes a default. Record
leaderboard results at the bottom of the algorithm's file.

### Neighbourhoods vs plain swap (2026-10-21, issue #8)

ansi-30, default profile, english, 20 threads. `gecko bench --algos hc,sa,bls --moves
"swap/columns/cycle3/depth2/all" --time 5s` (3 seeds; hc/bls rerun with 5 seeds):

| algo | swap | columns | cycle3 | depth2 (w 32) | cycle3+depth2 |
|---|---|---|---|---|---|
| hc | 16.017 ± 0.080 | = swap | 16.033 ± 0.138 | 15.940 ± 0.158 | 15.899 ± 0.185 |
| sa | 15.737 ± 0.048 | 15.704 ± 0 | 15.737 | 15.737 | 15.737 |
| bls | 15.724 ± 0.040 | 15.737 | 15.724 | 15.758 ± 0.068 | 15.889 ± 0.049 |

Per descent from 200 random layouts (single thread): swap 20.50 mean / 12 k evals; cycle3
19.68 / 34 k; depth2 w 8 20.22 / 20 k, w 32 19.58 / 56 k, w 0 18.43 / 728 k; columns never
improved a swap optimum (ansi-30 has only the two index-finger column pairs). Larger moves
give better optima per descent but cost 3–60× more, so: HC gains a little (within ~1σ);
SA's polish is unaffected (identical within noise); BLS loses because fewer kicks fit in the
budget, and its kicks already provide the escape. Defaults stay swap-only; revisit for
memetic (#13), where descents from crossover children are the inner loop.
