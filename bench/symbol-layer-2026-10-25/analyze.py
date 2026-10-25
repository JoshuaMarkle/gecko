"""Tables from study.py score output.

usage: python3 -I analyze.py scores.json
Per frame: every layout's free-profile score, its gap to the frame's best layout (points, % of
typing time, % per Sym-layer keystroke), and the human rules it breaks (brackets, sym_groups,
measured under the human profile). One point ≈ 0.043 % typing time per keystroke (README).
"""
import json, subprocess, sys
from pathlib import Path

PCT = 0.043  # % typing time per point (programming-layout README)
ROOT = Path(__file__).resolve().parents[2]
ASCII = "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~"
ON_BASE = {"us": "',./\"<>?", "ltgt": "',./\"?"}


def sym_share(tag):
    """% of keystrokes typed on the Sym layer: the frame's layer symbols (and ⏎ in Vim frames)."""
    corpus = "vim-programmer-typed" if tag.endswith("vim") else "programmer-typed"
    info = json.loads(subprocess.run([str(ROOT / "target/release/gecko"), "--json", "corpus", "info", corpus],
                                     check=True, capture_output=True, text=True).stdout)
    pm = {e["symbol"]: e["share"] for e in info["symbols_per_mille"]}
    syms = [c for c in ASCII if c not in ON_BASE[tag.split("-")[0]]] + (["⏎"] if tag.endswith("vim") else [])
    return sum(pm.get(c, 0) for c in syms) / 10

res = json.load(open(sys.argv[1]))
frames = sorted({v["tag"] for v in res.values()})
for f in frames:
    rows = [(k, v) for k, v in res.items() if v["tag"] == f]
    best = min(v["free"]["score"] for _, v in rows)
    share = sym_share(f)
    print(f"\n## {f}  (best free score {best:.2f}; Sym layer {share:.1f} % of keystrokes)\n")
    print("| layout | free | gap | % time | % per sym key | brackets broken | groups broken | layer presses | sym bigrams bad |")
    print("|---|---|---|---|---|---|---|---|---|")
    for k, v in sorted(rows, key=lambda kv: kv[1]["free"]["score"]):
        s = v["free"]["score"]
        hm = v["human"]["metrics"]
        gap = s - best
        print(f"| {k} | {s:.2f} | {gap:+.2f} | {gap * PCT:.3f} | {gap * PCT / share * 100:.1f} | "
              f"{hm['brackets'][0]:.0f} | {hm['sym_groups'][0]:.0f} | {hm['layer_switch'][0]:.2f} | {hm['symbol_bigram'][0]:.1f} |")
