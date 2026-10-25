# Goals

GECKO finds the best possible keyboard layouts, and later the best physical keyboards.
It is mostly a data-analysis system (judging layouts thoroughly and fast) with
optimization on top.

## Principles

1. **Judge thoroughly.** A large, growing metric set; every score is explainable per metric.
2. **Fast.** Optimize the swap-delta, not the full evaluation. Target ≥10M swap
   evaluations/sec across cores for 30 keys.
3. **Reproducible.** Same inputs + seed + code version → same result.
4. **Extensible.** A new metric or algorithm doesn't touch unrelated code or slow the hot loop.
5. **Two front ends, one core.** CLI for agents, web UI for the human.

## Phases

Current implementation includes layout optimizers, symbol layers, the web workflow,
2D/3D parametric bilevel search, hand IK, collision repair and manufacturability checks.
The phase 2 “beats the best known layouts” check is written up in [evaluation.md](evaluation.md) (#141).
Remaining scheduled work and its priority live in [roadmap.md](roadmap.md); the phase
list below describes the project's scope, not an execution queue.

| # | Phase | Done when |
|---|---|---|
| 1 | **Scorer MVP:** 30 main keys, standard row-stagger board, English corpus, P0 metrics, score known layouts | Rankings of known layouts match community consensus; SFB numbers match published ones |
| 2 | **Optimizers:** hill climb, SA, GA, tabu, memetic; algorithm leaderboard | Finds layouts that beat the best known layouts under our profile |
| 3 | **Full metric set:** P1/P2 metrics, profiles, rank-sum and Pareto selection | |
| 4 | **Web UI:** run browser, compare, interactive editor, 3D viewer | |
| 5 | **Symbol layers:** symbols, layers, modifiers, code corpora | Thumb keys, hold/one-shot/toggle layers, Shift, paired brackets (`brackets`), HRM misfires (`hrm_misfire`), number/navigation layers, adaptive keys, `code` corpus. |
| 6 | **2D geometry:** optimize key x, y, rotation on a plane, no overlaps | |
| 7 | **3D geometry:** add z, tilt, tent; hand model | |
| 8 | **Free-form 3D:** curved keyboards, full 6-DoF keys, thumb clusters, no intersections | |
| 9 | **Learned costs:** timing models, personal data, learned weights | Long-term direction; no scheduled timing-model or personal-calibration issue |

Basic keyboards must be excellent before geometry work starts. Geometry phases
must reuse the same scorer (the hand model only changes the precomputed cost tables).

## Non-goals

Firmware generation, typing tutor, hosted service, accounts.
