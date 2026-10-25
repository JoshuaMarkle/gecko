# The best programming layout with a symbol layer, 2026-10-25

Write-up: `docs/papers/programming-layout/` (`.md` + PDF) and
`research/experiments/2026-10-25-programming-layout.md`. corne-36, BLS 40 M evaluations,
4 threads, 6 seeds per cell.

| file | what |
|---|---|
| `shift.py`, `shift.json` | Aalto 136M scan: what a held Shift costs in typing time (56,358 QWERTY touch typists). Run from the costfit workspace: `python3 -I shift.py <Keystrokes/files> <metadata_participants.txt> shift.json` (~10 min, 16 cores) |
| `profiles/prog-old.toml`, `prog-cal.toml` | `code` plus the thumb zone (␣ ⌫ ⏎ on base thumbs) and letters on the base layer (`zones` 100); `layer_switch` at the old weight (1) or calibrated from `shift.json` (26.8) |
| `refs.py` → `layouts/` | Miryoku, Getreuer v2 and a US-style symbol layer on a Colemak-DH base, common thumbs `⏎ ␣ ⇧ │ ① ⌫ ②` |
| `seeds.py`, `grid.sh` | seeds × BLS per cell → `seeds-<cell>.json`: `full-cal`, `full-old`, `layer-cal` (Colemak-DH letters pinned), `full-code` (typed code only), `full-raw` (code files) |
| `variant.py`, `grid2.sh` | from the best `full-cal` layout: `digits-row` (1–0 on the layer home row, pinned), `oneshot`, `long` (3 × 160 M), `lang-{rust,python,js}` |
| `analyze.py` → `results.json` | every cell's best and the references scored on 7 corpora, plus per-term costs |
| `runs/` | `config.json` + `best.json` of the reported runs (progress logs dropped; `gecko verify run:<id>` re-runs from the config): `20261025-043842-bls-a679c37e` (optimum, `long`), `20261025-040445-bls-0aec6d68` (`full-cal`), `20261025-042045-bls-33240900` (`digits-row`) |

## Calibration of `layer_switch`

A held Shift is the one layer-like key the Aalto data measures. Capital after a space vs the same
letter in lowercase: +0.578 log interval (×1.78), from 120k capitals; letter → `?` vs letter →
`.`: +0.590. An SFB pair is +0.157 (+17 %) on the same scale, and `default` weights SFB at
2.867 / 0.7908 = 3.626 points per SFB %. `layer_switch` counts presses per 100 bigrams, so a
press worth 0.578 / 0.157 = 3.68 SFBs is 13.4 points per unit: weight 26.8 at scale 2 (was 1).
The release (letter after a capital, +0.40) is left out: names after a capital confound it.

## Results

`results.json`; write-up in the paper. Optimum 145.36 (`long`); 40 M `full-cal` 145.75; digits in a row 146.51;
optimized layers on Colemak-DH 150.66; Getreuer 151.36; US-style 161.88; Miryoku 172.08; old layer cost
188.64; one-shot 190.17. One point ≈ 0.043 % typing time per keystroke.
