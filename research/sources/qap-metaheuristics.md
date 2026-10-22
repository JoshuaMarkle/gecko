---
group: optimization
kind: paper
read: 2026-10-20
status: read
feeds: [B1, B3, search]
take: Robust tabu, BLS, BMA memetic
---

# QAP metaheuristics: robust tabu, breakout local search, memetic, ILS

Keyboard layout = QAP (+ cubic trigram terms). The QAP literature has well-tested methods.
Benchmark library: QAPLIB (https://coral.ise.lehigh.edu/data-sets/qaplib/).

## Methods

| Method | Source | Idea | Notes |
|---|---|---|---|
| Robust Tabu Search (RoTS) | Taillard, *Parallel Computing* 17, 1991 | Best non-tabu swap each step; tabu = putting a symbol back in a recent slot; **random tenure** (0.9n–1.1n, restated in a secondary source, arXiv:1010.0157; Taillard 1991 itself not read; later right-skewed in [0, max]); aspiration if new best; long-term diversification forces moves unused for long | Reference implementation and parameters: HeuristicLab docs (https://dev.heuristiclab.com/trac.fcgi/wiki/Documentation/Reference/Robust%20Taboo%20Search) |
| Breakout Local Search (BLS) | Benlic & Hao, *Appl. Math. Comput.* 2013 (https://leria-info.univ-angers.fr/~jinkao.hao/papers/BenlicHaoQAP2012.pdf) | Steepest descent to a local optimum, then a perturbation whose **strength adapts** (grows when it returns to the same optimum) and whose **type** is chosen adaptively: directed (tabu-based) or random | Matched best-known results on all but 2 QAPLIB instances. Code: https://leria-info.univ-angers.fr/~jinkao.hao/BLS_code.html |
| BMA (memetic) | Benlic & Hao, *ESWA* 42(1), 2015 | Population + BLS as local search + uniform crossover + fitness-and-distance pool update + adaptive mutation | Best-known on 133 of 135 QAPLIB instances |
| Iterated Local Search | Stützle, *EJOR* 2006 | Local search + perturbation (random k-swaps) + acceptance (better / restart) | Simple, strong baseline |
| Fast Ant System (FANT) | Taillard 1998; best in Dell'Amico 2009 keyboard study | Pheromone matrix symbol×slot, build a solution, local search, reinforce best | See `dellamico-single-finger-qap.md` |
| Hybrid GA | Drezner, *INFORMS J. Comput.* 2003 | GA with tabu-improved children and a merging crossover | |

## How to use in GECKO

- **Tabu next:** RoTS with the Δ-matrix trick (keep Δ for all 435 swaps; after a move only
  swaps touching the moved symbols change a lot). With trigrams, recompute rows for the two
  moved symbols and verify with the delta == full test.
- **BLS is likely the best single-thread improvement on SA** for permutation problems of this
  size; it needs only `delta_swap` and a tabu list.
- **Memetic (BMA-style)** for the island model in `context/algorithms/memetic.md`: uniform
  crossover = keep slots where both parents agree, fill the rest randomly; then BLS.
- Use pool **distance** (number of differing slots) in replacement to keep diversity.
- Report "found by N/M seeds" with results; repeated identical optima from different seeds
  are the practical stopping signal.
