#!/bin/sh
# Inverse fits of the English-study layouts on ansi-30 / prose under `default`, two priors.
set -e
cd "$(dirname "$0")"
G=../../target/release/gecko
SPECS=$(python3 -I -c "import json; print(' '.join('raw:'+x['chars'] for x in json.load(open('layouts.json'))))")
NAMES=$(python3 -I -c "import json; print(','.join(x['name'] for x in json.load(open('layouts.json'))))")
for s in 0.5 1.0; do
  $G fit-layout $SPECS -g ansi-30 -c prose -p default --sigma $s --bound 4.6 --rounds 12 --seed 1 --names "$NAMES" \
    --out fits-sigma$s.json --profiles profiles-sigma$s > fits-sigma$s.txt 2>/dev/null
done
