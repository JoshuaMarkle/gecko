# Cost-function validity (2026-10-22)

Data and code behind `docs/papers/cost-validity/` and
`research/experiments/2026-10-22-cost-function-validity.md`. GECKO commit 94a22f3, ansi-30,
`english`, default profile unless stated.

| Path | What |
|---|---|
| `results.json` | Every number and figure input of the paper (aggregates only) |
| `timefit.toml` | Profile with weights fitted to the time judge (S01/S02 kept); optimize with `costlab opt -p timefit.toml` |
| `costlab/` | Research CLI on `gecko-core`: `features` (unit-weight metric contributions per layout, stdin = layout specs), `tables` (per-metric pair tables + slots), `opt` (SA under weight vectors, stdin = JSONL `{"id","weights"}`) |
| `scripts/lib.py`, `a1`–`a13` | One analysis each: score decomposition, rank robustness, timing (bigram, trigram), Engram pairs, Goodhart check, sweep, scales, SFB–stretch trade-off, time judge and time-fitted weights, profile judges |
| `scripts/paper_data*.py`, `merge.py` | Recompute `results.json` from the analysis outputs |
| `scripts/a14_skipgrams.py`, `skipgram_timing.json` | SFS timing by context, finger, distance and outcome (`research/experiments/2026-10-22-skipgram-timing.md`) |
| `scripts/iki.py`, `iki2.py`, `iki3.py` | Scans of the Aalto 136M keystroke mirror: all-typist bigram means; bigram medians by typist group (relative IKI); trigram third-keystroke timing |

## Rerun

Scripts were run from `~/projects/gecko-workspace/costlab/` (paths in them are absolute; edit
`lib.py` and the CSV paths to rerun elsewhere). Python: `/usr/bin/python3` with numpy, scipy.

```
cargo build --release --manifest-path bench/cost-validity-2026-10-22/costlab/Cargo.toml
costlab features < layouts.txt > data/refs.jsonl            # also cmini, random, -c shai …
costlab tables > data/tables_ansi30.json
costlab opt --evals 20000000 --seeds 2 < data/w_sweep.jsonl > data/sweep.jsonl
python3 iki2.py <Keystrokes/files> <metadata_participants.txt> out2   # ~20 min, 16 cores
python3 iki3.py <Keystrokes/files> <metadata_participants.txt> out2
python3 paper_data.py && python3 paper_data2.py && python3 paper_data3.py && python3 merge.py
MPLCONFIGDIR=/tmp/gecko-mpl python3 -I docs/papers/cost-validity/figures.py
docs/papers/templates/build_pdf.sh docs/papers/cost-validity/cost-validity.tex
```

The keystroke data (Kaggle `noelsmathew/keystrokes`, CC BY-NC-SA 4.0; Dhakal et al., CHI 2018)
is not redistributed; only summary statistics derived from it are in `results.json`.
Weight vectors: default; each weight dropped, halved, doubled; 150 log-normal draws σ = 0.5 and
100 at σ = 1.0 (numpy seed 42). SA: 20M evaluations, seeds 1 and 2, best kept.
