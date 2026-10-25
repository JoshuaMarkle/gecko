#!/bin/sh
# Programming grid on corne-36 (see README). Starts: layouts/us-layer.toml (Colemak-DH base).
cd "$(dirname "$0")"
G=../../target/release/gecko
S=layouts/us-layer.toml
P=profiles
L=abcdefghijklmnopqrstuvwxyz
python3 -I seeds.py $G full-cal   $S programmer-typed $P/prog-cal.toml > seeds-full-cal.json
python3 -I seeds.py $G full-old   $S programmer-typed $P/prog-old.toml > seeds-full-old.json
python3 -I seeds.py $G layer-cal  $S programmer-typed $P/prog-cal.toml 6 40000000 $L > seeds-layer-cal.json
python3 -I seeds.py $G full-raw   $S programmer       $P/prog-cal.toml > seeds-full-raw.json
python3 -I seeds.py $G full-code  $S code-typed       $P/prog-cal.toml > seeds-full-code.json
