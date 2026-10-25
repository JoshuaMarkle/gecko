"""Score references and cell optima on every corpus and profile → results.json; print the tables.
Usage: python3 -I bench/programming-layout-2026-10-25/analyze.py   (after grid.sh and grid2.sh)
"""
import json, subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
G = str(ROOT / "target/release/gecko")
P = {"cal": str(HERE / "profiles/prog-cal.toml"), "old": str(HERE / "profiles/prog-old.toml")}
CORPORA = ["programmer-typed", "programmer", "code-typed", "code-rust-typed", "code-python-typed", "code-js-typed",
           "prose"]
KEYS = ["layer_switch", "symbol_bigram", "brackets", "sfb", "typing_time", "effort", "hand_switch", "zones"]

layouts = {f"ref:{n}": str(HERE / f"layouts/{n}.toml") for n in ("us-layer", "miryoku", "getreuer")}
cells = {}
for f in sorted(HERE.glob("seeds-*.json")):
    try:
        d = json.load(open(f))
    except json.JSONDecodeError:  # a cell still running
        continue
    best = min(d["rows"], key=lambda r: r["score"])
    cells[d["tag"]] = {"best": d["best"], "at_best": d["at_best"], "n": len(d["rows"]),
                       "scores": sorted(r["score"] for r in d["rows"]), "run": best["run"], "rows": best["rows"],
                       "corpus": d["corpus"], "profile": Path(d["profile"]).stem, "evals": d["evals"]}
    layouts[d["tag"]] = "run:" + best["run"]


def feats(spec, corpus, profile):
    out = subprocess.run([G, "features", "-g", "corne-36", "-p", profile, "-c", corpus], input=spec + "\n",
                         capture_output=True, text=True, cwd=ROOT, check=True).stdout
    return json.loads(out)


res = {}
for name, spec in layouts.items():
    r = res[name] = {"spec": spec, "scores": {}, "costs": {}}
    for c in CORPORA:
        f = feats(spec, c, P["cal"])
        r["scores"][c] = f["score"]
        r["costs"][c] = {k: f["costs"][k] for k in KEYS}
    r["old"] = feats(spec, "programmer-typed", P["old"])["score"]
json.dump({"cells": cells, "layouts": res}, open(HERE / "results.json", "w"), indent=1)

print(f"{'layout':16s} {'cal':>8s} {'old':>7s} {'layer%':>7s} {'symbi':>6s} {'brk':>5s} {'sfb':>5s} {'time':>5s}")
for n, r in sorted(res.items(), key=lambda kv: kv[1]["scores"]["programmer-typed"]):
    c = r["costs"]["programmer-typed"]
    print(f"{n:16s} {r['scores']['programmer-typed']:8.2f} {r['old']:7.2f} {c['layer_switch']:7.2f} {c['symbol_bigram']:6.1f}"
          f" {c['brackets']:5.1f} {c['sfb']:5.2f} {c['typing_time']:5.2f}")
print()
for t, c in cells.items():
    print(f"{t:12s} {c['corpus']:18s} best {c['best']:.3f} at best {c['at_best']}/{c['n']} spread {c['scores'][-1] / c['scores'][0] - 1:.1%}")
