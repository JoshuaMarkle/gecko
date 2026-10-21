# Memetic benchmark and reproducible runs

2026-10-21, issue #13 implementation on base `fd55a73`, ansi-30 / english / default.
The inputs are versioned under `data/`; each run records their hashes, start layout,
seed and all optimizer parameters. Background builds/tests ran during the time-based
leaderboard; throughput is indicative.

```
gecko --json bench --algos sa,bls,tabu,memetic --time 2s --seeds 10 --threads 4
```

`leaderboard.json` contains every seed's score. See `context/algorithms/memetic.md`
for the comparison and interpretation.

| Run | Refinement | Population | Local steps | Migration interval | Seed | Requested evals | Actual evals | Best |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| `20261021-142007-memetic-eeabc4a8` | HC | 12 | 8 | 1 | 13 | 2,000,000 | 2,058,279 | 18.421767 |
| `20261021-142009-memetic-6b25a149` | tabu | 12 | 8 | 1 | 13 | 2,000,000 | 2,224,284 | 18.421767 |
| `20261021-142140-memetic-0d7f9b24` | HC | 24 | 12 | 5 | 3 | 8,000,000 | 8,309,769 | 17.561220 |

All use four islands and four threads. The final run reaches the short leaderboard's
best BLS score after roughly seven seconds. Generation boundaries explain the evaluation
overshoot. All three runs pass `gecko verify` with this implementation.

To replay one from the repository root, copy its curated directory into `runs/`, then
verify it (use a different ID from the table for the other examples):

```
cp -R bench/memetic-2026-10-21/runs/20261021-142140-memetic-0d7f9b24 runs/
./target/release/gecko verify run:20261021-142140-memetic-0d7f9b24
```
