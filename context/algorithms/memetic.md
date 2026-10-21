# Memetic algorithm (GA + local search), island model

Implemented in `optimize/memetic.rs`; `gecko optimize --algo memetic`, `gecko bench
--algos sa,bls,tabu,memetic`, and the Search tab. Every island evolves a small population
with GA's shared tournament selection, order crossover on movable slots and swap mutation.
Every child receives bounded local refinement before duplicate rejection and insertion.
The starting layout is refined too; generations retain elites. Pins and fixed layer/Shift
keys remain excluded from every operator.

**Refinement:** `hc` uses shared swap descent, then configured `--moves` neighbourhoods
when swap-optimal; `local_steps` caps improving moves (default 12). `tabu` uses robust tabu's
existing incremental delta matrix, randomized tenure and aspiration for the given number
of iterations, returning its best visited layout. Neither refiner worsens its input. Child
scoring and local-search candidate evaluations count toward the evaluation budget; there
is no final, uncounted descent.

**Islands:** stable RNG stream per island; worker threads evaluate disjoint groups in
parallel, then the coordinator exchanges snapshots around a directed ring. Every
`migration_interval` generations the best `migrants` distinct individuals go to the next
island, filling vacancies or replacing worse individuals. Snapshots prevent immediate
multi-hop forwarding. Duplicate immigrants are ignored. Three consecutive generations
below one-quarter population restart the island with fresh refined random children,
retaining its elite. Child attempts are capped at 3× population size, so small search spaces
and converged populations cannot get stuck trying to fill an impossible population.

**Parameters:** `params.memetic` contains `ga` (same operator fields as GA, default
population 24 / elitism 2), `local_search` (`hc|tabu`), `local_steps` (12), `islands`
(0 = threads), `migration_interval` (5; 0 disables), `migrants` (2), and
`collapse_generations` (3; 0 disables). CLI: `--memetic-pop`, `--local-search`,
`--local-steps`, `--islands`, `--migration-interval`, `--migrants`; remaining parameters
are available through the server's `memetic` object / saved configuration.

**Budgets and reproducibility:** pause, cancel and budgets are checked between complete
parallel generations; the last generation can exceed the requested time/evaluation
budget. No workers wait on a barrier after another worker exits. Explicit island count
makes `--evals` results and evaluation counts identical across worker counts; otherwise
changing threads also changes the default number of islands. All effective parameters
are stored in runs. Fixed layouts return immediately. Thread creation happens once per
generation; benchmark before choosing very small populations with no refinement.

## Results

2026-10-21, ansi-30 / english / default, 4 threads, 10 seeds × 2 s, swap-only.
Raw leaderboard: `bench/memetic-2026-10-21/leaderboard.json` (same source revision for all
algorithms; background builds/tests were running, so throughput is indicative).

| algo | mean | best | stdev | M evals/s |
|---|---:|---:|---:|---:|
| SA | 17.7869 | 17.5654 | 0.1454 | 0.500 |
| BLS | 17.6161 | 17.5612 | 0.0812 | 1.116 |
| tabu | 18.9602 | 18.2805 | 0.2561 | 4.132 |
| memetic (HC defaults) | 18.0269 | 17.6502 | 0.3522 | 1.116 |

The default short-budget memetic search trails SA/BLS, with more variability; BLS remains
the stronger short-run option. Tabu's candidate-scan counts include its cached delta
matrix, so its evaluation rate is not directly comparable to independent swap deltas.
Population refinement has useful search diversity but is not a new default.

Two deterministic 2M-evaluation examples use seed 13, 4 islands/workers, population 12,
8 local steps and migration every generation. Both find 18.421767, retaining distinct
alternative layouts; HC uses 2,058,279 candidate evaluations, tabu 2,224,284 (generation
overshoot). Both pass `gecko verify`. Full configs, input hashes, results and progress are
curated in `bench/memetic-2026-10-21/runs/`; see its README for replay commands.

A longer default-population HC run (seed 3, 4 islands/workers, 8M requested evaluations,
8,309,769 actual, about 7 s) finds **17.561220**, matching the best BLS score above.
Curated run `20261021-142140-memetic-0d7f9b24` verifies exactly. The larger initial
refinement cost pays back over a longer budget; more seeds are needed for a long-budget
comparison.
