#!/bin/sh
# Second programming grid, from the best full-cal layout (layouts/best.toml, variant.py).
cd "$(dirname "$0")"
G=../../target/release/gecko
P=profiles/prog-cal.toml
python3 -I seeds.py $G digits-row layouts/best-digits-row.toml programmer-typed $P 6 40000000 1234567890 > seeds-digits-row.json
python3 -I seeds.py $G oneshot    layouts/best-oneshot.toml    programmer-typed $P 4 > seeds-oneshot.json
python3 -I seeds.py $G long       layouts/best.toml            programmer-typed $P 3 160000000 > seeds-long.json
for l in rust python js; do python3 -I seeds.py $G lang-$l layouts/best.toml code-$l-typed $P 3 > seeds-lang-$l.json; done
