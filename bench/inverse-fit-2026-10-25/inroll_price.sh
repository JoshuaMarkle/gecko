#!/bin/sh
# The price of inward rolls: the optimum under `default` with the inroll weight ×1…×16
# (ansi-30, prose, the GECKO optimum's apos symbols), 4 seeds × BLS 25M each.
set -e
cd "$(dirname "$0")"
G=../../target/release/gecko
mkdir -p price
for k in 1 2 4 8 16; do
  for s in 1 2 3 4; do
    $G --json optimize --algo bls --evals 25000000 --seed $s -g ansi-30 -c prose \
      -p profiles-inroll/inroll-x$k.toml --from "raw:cwdlxjyou.gstnvpheiafbmrqzk'/," --top 1 > price/x$k-s$s.json
  done
done
