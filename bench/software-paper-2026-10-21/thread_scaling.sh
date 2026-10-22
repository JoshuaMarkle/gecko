#!/usr/bin/env bash
# Search throughput against thread count (software paper, Fig. 4): SA, ansi-30, english, default profile,
# 4 s per point, 3 seeds. Run on an idle machine. Writes thread_scaling.json beside this script.
set -euo pipefail
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
gecko=$here/../../target/release/gecko
out=$(mktemp -d); trap 'rm -rf "$out"' EXIT
echo '[' > "$here/thread_scaling.json"; first=1
for t in 1 2 4 8 12 16 20; do
  for seed in 1 2 3; do
    r=$("$gecko" optimize --algo sa --time 4s --threads "$t" --seed "$seed" --json 2>/dev/null)
    eps=$(printf '%s' "$r" | python3 -I -c 'import json,sys; d=json.load(sys.stdin); print(d["run"]["evals_per_sec"])')
    [[ $first == 1 ]] || echo ',' >> "$here/thread_scaling.json"; first=0
    printf '{"threads": %s, "seed": %s, "evals_per_sec": %s}' "$t" "$seed" "$eps" >> "$here/thread_scaling.json"
    echo "threads=$t seed=$seed evals/s=$eps"
  done
done
echo ']' >> "$here/thread_scaling.json"
