"""Summarize the fidelity runs: per board and top-K, mean wall time and best exact score."""
import glob, json, os, sys
from collections import defaultdict
d = os.path.dirname(os.path.abspath(__file__))
rows = defaultdict(list)
for f in sorted(glob.glob(os.path.join(d, '*.json'))):
    name = os.path.basename(f)[:-5]
    board, k, seed = name.rsplit('-', 2)
    r = json.load(open(f))['run']
    fid = r.get('fidelity') or {}
    rows[(board, int(k[1:]))].append((seed, r['elapsed_ms'] / 1000, r['best'][0]['score'], r['id'], fid.get('levels'), fid.get('final_level')))
for (board, k), v in sorted(rows.items()):
    t = sum(x[1] for x in v) / len(v)
    s = sum(x[2] for x in v) / len(v)
    print(f"{board:12} K={k:<5} time {t:6.2f}s  mean best {s:.4f}  " + "  ".join(f"{x[0]}:{x[2]:.4f}" for x in v) + f"  levels {v[0][4]} final {[x[5] for x in v]}")
    for x in v:
        print("   ", x[3])
