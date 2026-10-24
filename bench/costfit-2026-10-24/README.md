# Cost-function optimization (2026-10-24)

Data and code behind `research/experiments/2026-10-24-cost-function-optimization.md` and the
`evidence` weights, `data/profiles/default.toml` since 2026-10-24 (runs here record the profile
under its old name `evidence`; `gecko verify` uses the embedded copy). GECKO `dcc3cd1`, ansi-30,
`english` unless stated.

| Path | What |
|---|---|
| `costfit/` | Research CLI on `gecko-core`: `corpus` (n-gram counts as JSON), `tables` (every metric's cost table), `features` (unit metric contributions per layout, stdin = layout specs), `opt` (SA/BLS under weight vectors, stdin = JSONL `{"id","weights","scale"?}`) |
| `scripts/iki4.py`, `iki5.py` | Aalto 136M scans: 30-key pair and triple timing of touch typists, two typist halves (iki5 adds AZERTY/QWERTZ/Dvorak typists, mapped to physical keys) |
| `scripts/ept.py`, `judge.py` | Pair means, practice control, and the held-out time judge (`J_B`) |
| `scripts/s1_pairs.py`, `s1b_resid.py` | Study 1: key-pair time on GECKO's pair tables; what the metrics miss |
| `scripts/s2_trigrams.py`, `s2b_explore.py` | Study 2: third-keystroke context on trigram and skipgram tables |
| `scripts/lib4.py`, `s4_eval.py`, `s5_sweep.py` | Studies 4–5: fitted profiles, comfort-share sweep, optimum judged |
| `scripts/s6_cma.py`, `s7_pool.py` | Studies 6–7: CMA-ES over weights (bilevel), pooled re-evaluation |
| `scripts/s8_judges.py`, `seeds.py` | Study 8: judges of the chosen optimum; seeds × BLS with `gecko optimize -p` |
| `results/` | One JSON per study (`s1_pairs_<practice>.json`, `final_coefs.json`, `s4_eval.json`, `s5_sweep.json`, `s6_<run>_best.json` and `.log.json`, `s7_pool.json`, `s8_judges.json`), `seeds-evidence-<board>-<corpus>.json`, `sweep-evidence.json` |
| `runs/` | The four winning 25 M BLS runs; each passes `gecko verify` |

## Rerun

Scripts ran from `~/projects/gecko-workspace/costfit/` (absolute paths inside; edit `W` in
`lib4.py` / `ept.py` to move). Python: `/usr/bin/python3` with numpy, scipy.

```
cargo build --release --manifest-path costfit/Cargo.toml   # binary into <W>/target (CARGO_TARGET_DIR)
costfit corpus > data/corpus_english.json; costfit corpus -c shai > data/corpus_shai.json
costfit tables > data/tables_ansi30.json
costfit features < refs.txt > data/refs.jsonl     # also cmini.txt, random.txt, tiers.txt (16 tier layouts)
python3 -I iki4.py <Keystrokes/files> <metadata_participants.txt> timing30.npz     # ~3 min, 16 cores
python3 s1_pairs.py lin|quad|letters|none; python3 s2_trigrams.py
python3 s4_eval.py; python3 s5_sweep.py
python3 s6_cma.py 0.5 0.8 40 a50; … 0.25 0.8 40 a25; … 0.75 0.8 40 a75; … 0.5 0.0 40 a50free   # ~12 min each
python3 s7_pool.py; python3 s8_judges.py
python3 -I seeds.py <gecko> evidence <board> <corpus> > results/seeds-evidence-<board>-<corpus>.json
gecko sweep -p evidence --draws 30 --evals 10000000 --out results/sweep-evidence.json
gecko compare -p evidence --robust
```

The keystroke data (Kaggle `noelsmathew/keystrokes`, CC BY-NC-SA 4.0; Dhakal et al., CHI 2018)
is not redistributed; `timing30.npz` (per-pair sums) stays in the workspace, and only fitted
coefficients and judge values are here.
