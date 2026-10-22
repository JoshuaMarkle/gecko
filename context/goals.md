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

Status 2026-10-21: phases 1 and 4 done (web UI with tabs per design step); phase 2's optimizers
built (HC, SA, GA, tabu, memetic, NSGA-II, MAP-Elites, CMA-ES, Gilmore–Lawler bound), its
"beats the best known layouts" check not yet written up; phase 3 mostly
done (C1–C9: finger-pair table, P1/P2 metrics, learned profile weights, held-out corpora);
phase 5 mostly done (combos/chords left); phase 6 started (parametric 2D boards, fast table
rebuild, collision, geometric costs); phase 7 started (hand model, IK, reach, press angle, wrist
posture). Next: bilevel geometry + layout search (#54), then 3D parameters and collision.

| # | Phase | Done when |
|---|---|---|
| 1 | **Scorer MVP:** 30 main keys, standard row-stagger board, English corpus, P0 metrics, score known layouts | Rankings of known layouts match community consensus; SFB numbers match published ones |
| 2 | **Optimizers:** hill climb, SA, GA, tabu, memetic; algorithm leaderboard | Finds layouts that beat the best known layouts under our profile |
| 3 | **Full metric set:** P1/P2 metrics, profiles, rank-sum and Pareto selection | |
| 4 | **Web UI:** run browser, compare, interactive editor, 3D viewer | |
| 5 | **Symbol layers:** symbols, layers, modifiers, code corpora | Thumb keys, hold/one-shot/toggle layers, Shift, paired brackets (Y03), HRM misfires (Y04), number/navigation layers, adaptive keys, `code` corpus. Left: combos and chords (#91) |
| 6 | **2D geometry:** optimize key x, y, rotation on a plane, no overlaps | |
| 7 | **3D geometry:** add z, tilt, tent; hand model | |
| 8 | **Free-form 3D:** curved keyboards, full 6-DoF keys, thumb clusters, no intersections | |
| 9 | **Learned costs:** timing models, personal data, learned weights | |

Basic keyboards must be excellent before geometry work starts. Geometry phases
must reuse the same scorer (the hand model only changes the precomputed cost tables).

## Non-goals

Firmware generation, typing tutor, hosted service, accounts.
