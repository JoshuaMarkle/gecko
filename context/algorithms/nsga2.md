# NSGA-II (multi-objective)

**Idea:** optimize several objectives at once (e.g. SFB, rolls, effort, familiarity) and
return the **Pareto front**: layouts where no objective can improve without another
getting worse. The user picks the trade-off instead of guessing weights.

**Details:** non-dominated sorting into fronts, crowding distance for diversity,
binary tournament on (front, crowding). Same permutation operators as `genetic.md`;
add local search on each objective's weighted combination for a memetic variant.

**Alternative (cheaper):** run many single-objective SA runs with random weight vectors,
then apply rank-sum selection (Kühlmak) or extract the non-dominated set.

**Parameters:** `objectives` (metric IDs), `pop_size`, `generations`, operator rates.

**Output:** the front saved in the run; the web UI shows it as a scatter/parallel-coordinates plot.

## Usage

```sh
./target/release/gecko optimize --algo nsga2 --objectives sfb,inroll,effort --time 30s
./target/release/gecko optimize --algo rwsa --objectives sfb,inroll,effort --evals 50000
```

## Results

NSGA-II and RWSA discover non-dominated Pareto sets across conflicting metrics (such as `sfb` vs `inroll` vs `effort`). Each layout on the front records both raw metric values and normalized minimization costs; verified replays reproduce identical fronts deterministically.

