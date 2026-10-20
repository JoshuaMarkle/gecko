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

## Results

_None yet._
