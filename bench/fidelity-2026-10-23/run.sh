#!/bin/sh
# B8 adaptive fidelity: exact vs top-K trigram search on the code corpus / code profile.
# Same --evals budget per run (SA, 4 threads); compare wall time and the exact best score.
set -e
G=./target/release/gecko
O=bench/fidelity-2026-10-23
for seed in 1 2 3; do
  for k in 0 200 1000; do
    $G --json optimize -c code -p code --evals 20000000 --threads 4 --seed $seed --fidelity $k > $O/ansi30-k$k-s$seed.json
    $G --json optimize -g corne-36 -c code -p code --from data/layouts/layered/corne-code.toml \
      --evals 20000000 --threads 4 --seed $seed --fidelity $k > $O/corne-code-k$k-s$seed.json
  done
done
# About equal time on corne-code: K=1000 gets twice the evaluations (exact re-run alongside).
for seed in 1 2 3; do
  $G --json optimize -g corne-36 -c code -p code --from data/layouts/layered/corne-code.toml \
    --evals 40000000 --threads 4 --seed $seed --fidelity 1000 > $O/corne-code2x-k1000-s$seed.json
  $G --json optimize -g corne-36 -c code -p code --from data/layouts/layered/corne-code.toml \
    --evals 20000000 --threads 4 --seed $seed > $O/corne-code2x-k0-s$seed.json
done
python3 -I bench/fidelity-2026-10-23/slim.py bench/fidelity-2026-10-23/*.json
