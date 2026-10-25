# Inverse fits of 20 English layouts and the case for inward rolls, 2026-10-25

Write-up: `docs/papers/inverse-fit/` (`.md` + PDF). ansi-30, `prose`, profile `default`;
`gecko fit-layout` (squared hinge, tolerance 0.05, bound 4.6 = ×100, 12 rounds, BLS 4 M × 2 seeds
per round, 600 swap neighbours, all cores).

| file | what |
|---|---|
| `layouts.json` | the 20 layouts of the English study (classic, modern, GECKO `prose` optima per symbol set) |
| `grid.sh` → `fits-sigma{0.5,1.0}.{json,txt}`, `profiles-sigma*/` | inverse fit of every layout at prior σ 0.5 and 1.0; cross matrix; each fit as a profile |
| `roll_timing.py` → `roll_timing.json` | inward vs outward rolls: Aalto pair times (practice-adjusted, `~/projects/gecko-workspace/costfit/timing30.npz`), third keystroke after a roll, Engram measured choices |
| `profiles-inroll/`, `inroll_price.sh` → `price/` | the optimum under `default` with the rolls-in weight ×1, 2, 4, 8, 16 (4 BLS seeds × 25 M each, from the GECKO apos optimum) |
| `analyze.py` → `results.json` | fit summaries, modern-layout mean multipliers, price table |
| `judges.py` → `judges.json` | held-out timing judges J_B, J_B+l (cost-function study) on every layout and price optimum |

## Results

No community layout is self-consistent; Galite 0.53 %, Graphite 0.56 %, Dhorf 0.94 %, Gallium 1.18 %
under their own fits (σ 0.5). Modern layouts' mean multipliers: Engram ×0.14, hand alternation ×0.28,
pinky–ring ×0.25, SFS ×5.0, rolls in ×1.42, rolls out ×1.08. Inward roll pairs 4.7 % slower than
outward (CI +0.2 to +10.1 %); Engram P(inward) 0.51. Details in the paper.

The first grid (logistic likelihood) was replaced: it moved the self-consistent GECKO optimum's
weights (Engram ×3) for nothing; the squared hinge keeps it at ×1 (`context/decisions/scoring.md`).
