# Software paper experiments

2026-10-21 (run ids are UTC, so `20261022-…`), i9-13900H (6 performance + 8 efficiency cores),
ansi-30 / english / default unless stated. Used by `docs/papers/software/` (figures:
`docs/papers/software/figures.py`). Time-budget runs are not deterministic, so `gecko verify`
refuses them; they are kept for provenance and plotting only.

| File | Command | Used for |
|---|---|---|
| `metrics_scaling.py` → `metrics_scaling.json` | `python3 -I bench/software-paper-2026-10-21/metrics_scaling.py` (idle machine) | swap-delta rate vs metrics enabled: 4.8 M (bigram) → 2.4 M (far-skipgram channel) → 2.1 M (load terms) → 0.64 M (trigram) |
| `thread_scaling.sh` → `thread_scaling.json` | `bench/software-paper-2026-10-21/thread_scaling.sh` | SA evals/s at 1–20 threads, 3 seeds: 0.57 M → 5.0 M (8.7×); the 8-thread points ran under background load |
| `runs/20261022-031427-sa-8b3885fd` | `gecko optimize --time 10s --seed 7 --holdout shai` | best 17.561; holdout keeps 99.6 % of the gain, top-10 rank ρ 0.16 |
| `runs/20261022-031438-sa-633849eb` | `gecko optimize -g corne-36 --pin '{space}{enter}{bspc}{tab}{esc}' --time 10s --seed 3` | Corne with pinned thumbs: 9.229, 4.7 M evals/s, bound gap 91 % |
| `runs/20261022-031516-qd-af3fe56a` | `gecko qd --time 20s --seed 1 --axes roll_ratio,index_load --bins 12` | MAP-Elites map: 92 of 144 cells, 107 M evals |
| `graphite-score.json` | `gecko score graphite --json` | per-metric trade-off figure |
| `compare-references.json` | `gecko compare --json` | reference ranking |
| `magic-sturdy-compare.json` | `gecko compare sturdy magic/magic-sturdy --json` | Sturdy 21.831 → Magic Sturdy 19.995 |
| `landscape.json` | `gecko landscape --samples 2000 --walks 32 --seed 1 --json` | random baseline 98.4 ± 17.5, correlation length 8.5 swaps |
