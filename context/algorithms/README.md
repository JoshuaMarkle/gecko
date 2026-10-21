# Algorithms

All discrete optimizers implement one trait in `gecko-core::optimize`:

```rust
trait Optimizer {
    fn init(&mut self, scorer: &Scorer, start: Option<&Layout>, rng: &mut Rng);
    fn step(&mut self, scorer: &Scorer, rng: &mut Rng) -> StepInfo; // a batch of work
    fn best(&self) -> &[Scored<Layout>];                              // top-K, de-duplicated
}
```

Implemented: HC, SA, GA, BLS, Explore (`gecko-core/src/optimize/`, `explore/`). The runner handles time budgets, threads/restarts, progress events, and persistence,
so algorithms contain only search logic. All moves respect pins.

| File | Kind | Status | Role |
|---|---|---|---|
| [hill-climbing.md](hill-climbing.md) | Local search | Phase 2, first | Baseline + inner local search for others |
| [simulated-annealing.md](simulated-annealing.md) | Local search | Phase 2 | Main workhorse (carpalx, KLO, Kühlmak use it) |
| [genetic.md](genetic.md) | Population | Phase 2 | Comparison; staying near a start layout |
| [tabu-search.md](tabu-search.md) | Local search | Phase 2 | Strong for QAP |
| [memetic.md](memetic.md) | Hybrid | Phase 2–3 | GA + local search, island model |
| [nsga2.md](nsga2.md) | Multi-objective | Phase 3 | Pareto fronts |
| [explore.md](explore.md) | Local search + archive | Done | BLS; explore = probe + BLS with exclusion restarts, writes an atlas of local optima |
| [magic-rules.md](magic-rules.md) | Greedy (lazy) | Done | Rules of an adaptive (magic) key on a fixed layout |
| [geometry-search.md](geometry-search.md) | Continuous | Phase 6+ | Key positions/rotations; bilevel with layout |

Shared neighborhoods: swap (default), 3-cycle, swap two columns on the same finger, swap
hand halves (mirroring). Every algorithm is compared with `gecko bench` before it becomes
a default. Record leaderboard results at the bottom of the algorithm's file.
