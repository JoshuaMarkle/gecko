---
group: optimization
kind: paper
read: 2026-10-20
status: secondary
feeds: [B4, B5, G5]
take: CMA-ES, NSGA-II, MAP-Elites, parallel tempering
---

# General search methods for GECKO's later phases

Canonical references for methods GECKO plans or should consider. Not keyboard-specific.

| Method | Reference | Use in GECKO |
|---|---|---|
| CMA-ES | Hansen, "The CMA Evolution Strategy: A Tutorial", arXiv:1604.00772; libs: `cma` (Python), `cmaes` crate | Outer loop over continuous geometry params (phase 6+). Popsize 4+3·ln(n); restarts with growing population (IPOP) for multimodal problems |
| NSGA-II | Deb et al., *IEEE TEC* 6(2), 2002 | Pareto fronts over metrics (`context/algorithms/nsga2.md`) |
| MAP-Elites (quality diversity) | Mouret & Clune, arXiv:1504.04909, 2015 | Keep the best layout per cell of a behaviour grid (e.g. roll ratio × hand balance × index load). Gives users a **map of good, different layouts** instead of near-duplicates. Cheap with delta evaluation |
| Parallel tempering | Swendsen & Wang 1986; Earl & Deem 2005 review | Replicas at several T swap states; robust to a bad T schedule; good use of many cores |
| Threshold accepting | Dueck & Scheuer, *J. Comput. Phys.* 1990 | What Kühlmak actually uses (`kuehlmak.md`). Deterministic, no exp() |
| Late Acceptance Hill Climbing | Burke & Bykov, *EJOR* 2017 | One parameter (history length); compare against SA in `gecko bench` |
| Bayesian optimization | Shahriari et al., *Proc. IEEE* 2016; Liao's multi-objective BO for input devices (`ml-approaches.md`) | When each evaluation is expensive: geometry candidates scored by an inner layout search, or user-in-the-loop studies |
| Differential evolution | Storn & Price 1997 | Alternative to CMA-ES for geometry with bounds |

## Notes

- Bilevel (geometry outer, layout inner): use **common random numbers** and warm starts in the
  inner SA so the outer optimizer sees a smooth signal (`context/algorithms/geometry-search.md`).
- For many objectives, cheap alternatives to NSGA-II: random weight vectors + rank-sum
  (Kühlmak), or MAP-Elites with the score as quality and 2–3 metrics as behaviour.
