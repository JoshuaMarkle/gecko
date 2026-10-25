#!/bin/sh
# Second Vim grid: 3 seeds × 160 M evaluations from each policy's best 40 M layout (variant.py),
# because 40 M seeds spread 4–5 %, more than the policies differ.
cd "$(dirname "$0")"
G=../../target/release/gecko
P=profiles
SEEDS=../programming-layout-2026-10-25/seeds.py
best() { python3 -c "import json;d=json.load(open('seeds-$1.json'));print(min(d['rows'],key=lambda r:r['score'])['run'])"; }
for v in free pairs row fingers qwerty; do
  python3 -I ../programming-layout-2026-10-25/variant.py "$(best vim-$v)" layouts/best-$v.toml > /dev/null
done
# all five cells at once (15 searches × 4 threads; the evaluation budget, not time, ends a run)
for v in free pairs row fingers; do
  python3 -I $SEEDS $G long-$v layouts/best-$v.toml vim-programmer-typed $P/vim-$v.toml 3 160000000 > seeds-long-$v.json &
done
python3 -I $SEEDS $G long-qwerty layouts/best-qwerty.toml vim-programmer-typed $P/vim-free.toml 3 160000000 hjkl > seeds-long-qwerty.json &
wait
