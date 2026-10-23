#!/bin/sh
# C11 fingering search vs fixed fingerings (ansi-30, ansi-30-angle), english / default.
set -e
G=./target/release/gecko
for seed in 1 2 3; do
  for g in ansi-30 ansi-30-angle; do
    $G --json optimize -g $g --evals 20000000 --threads 4 --seed $seed > bench/fingering-2026-10-23/$g-s$seed.json
  done
  $G --json optimize -g ansi-30-alt --fingering --evals 20000000 --threads 4 --seed $seed > bench/fingering-2026-10-23/alt-s$seed.json
done
python3 -I bench/fidelity-2026-10-23/slim.py bench/fingering-2026-10-23/*.json
