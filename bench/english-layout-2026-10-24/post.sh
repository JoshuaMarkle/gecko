#!/bin/sh
# After grid.sh: weight sweep from the prose optimum, sub-problem certificate, then gather.
cd "$(dirname "$0")"
G=../../target/release/gecko
RUN=$(python3 -c "import json;d=json.load(open('seeds-prose-apos.json'));print(min(d['rows'],key=lambda r:r['score'])['run'])")
OPT=$(python3 -c "import json;d=json.load(open('seeds-prose-apos.json'));print(min(d['rows'],key=lambda r:r['score'])['chars'])")
$G --json sweep -c prose --from "raw:$OPT" --draws 40 --evals 10000000 --seed 1 --out sweep.json > /dev/null
$G --json certify run:$RUN --free 18 --kind full --time 600s > certify-full.json
python3 -I gather.py > gather.txt
