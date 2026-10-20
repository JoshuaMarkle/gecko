# Memetic algorithm (GA + local search), island model

**Idea:** a GA where every child is improved by local search (hill climb, short SA, or
short tabu) before it joins the population. Population search finds good regions;
local search finishes them.

**Island model:** several populations on separate threads, exchanging their best
individuals every `migration_interval` generations. Keeps diversity and uses all cores.

**Parameters:** GA parameters + `local_search` (hc|sa|tabu) and its budget,
`islands`, `migration_interval`, `migrants`.

**Pitfalls:** local search dominates CPU time; keep its budget small and the population
small (10–50). Watch for premature convergence (all individuals identical); restart
islands that collapse.

## Results

_None yet._
