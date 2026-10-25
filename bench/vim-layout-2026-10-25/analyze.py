"""Score the Vim cells' optima and references → results.json; print the tables.
Usage: python3 -I bench/vim-layout-2026-10-25/analyze.py   (after grid.sh)
The typing cost of a policy is its optimum's score under `vim-free` (no h j k l term).
"""
import json, subprocess, tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
G = str(ROOT / "target/release/gecko")
FREE = str(HERE / "profiles/vim-free.toml")
CORPORA = ["vim-programmer-typed", "programmer-typed", "vimshare-11", "vimshare-33", "vim"]
KEYS = ["layer_switch", "symbol_bigram", "sfb", "typing_time", "engram", "hand_switch", "vim"]
FINGER = ["L pinky", "L ring", "L middle", "L index", "L index", "R index", "R index", "R middle", "R ring", "R pinky"]
THUMB = ["L outer thumb", "L middle thumb", "L inner thumb", "R inner thumb", "R middle thumb", "R outer thumb"]
ROW = ["top", "home", "bottom"]
LAYER = ["base", "layer 1", "layer 2"]
WATCH = "⎋:/hjkl⏎wbe"
TOK = {"{enter}": "⏎", "{space}": "␣", "{bspc}": "⌫", "{shift}": "⇧", "{layer1}": "①", "{layer2}": "②", "{esc}": "⎋"}

layouts = {"ref:us-layer-esc": str(HERE / "layouts/us-layer-esc.toml"), "ref:prog-esc": str(HERE / "layouts/prog-esc.toml")}
cells = {}
for f in sorted(HERE.glob("seeds-*.json")):
    try:
        d = json.load(open(f))
    except json.JSONDecodeError:  # a cell still running
        continue
    best = min(d["rows"], key=lambda r: r["score"])
    cells[d["tag"]] = {"best": d["best"], "at_best": d["at_best"], "n": len(d["rows"]),
                       "scores": sorted(r["score"] for r in d["rows"]), "run": best["run"], "rows": best["rows"],
                       "corpus": d["corpus"], "profile": Path(d["profile"]).stem}
    layouts[d["tag"]] = "run:" + best["run"]
# Each policy's optimum: the better of its 40 M cell (vim-<p>) and its 160 M cell (long-<p>).
best_of = {}
for v in ("free", "pairs", "row", "fingers", "qwerty"):
    tags = [t for t in (f"vim-{v}", f"long-{v}") if t in cells]
    if tags:
        best_of[v] = min(tags, key=lambda t: cells[t]["best"])


def feats(spec, corpus, profile=FREE):
    out = subprocess.run([G, "features", "-g", "corne-36", "-p", profile, "-c", corpus], input=spec + "\n",
                         capture_output=True, text=True, cwd=ROOT, check=True).stdout
    return json.loads(out)


def where(rows, ch):
    """Layer, row and finger of a symbol in 4-rows-per-layer corne-36 rows."""
    for i, r in enumerate(rows):
        if ch in r:
            layer, row, col = i // 4, i % 4, r.index(ch)
            return f"{LAYER[layer]}, " + (THUMB[col] if row == 3 else f"{ROW[row]} {FINGER[col]}")
    return "—"


res = {}
for name, spec in layouts.items():
    r = res[name] = {"spec": spec, "scores": {}, "costs": {}}
    for c in CORPORA:
        f = feats(spec, c)
        r["scores"][c] = f["score"]
        r["costs"][c] = {k: f["costs"].get(k, 0.0) for k in KEYS}
    rows = cells[name]["rows"] if name in cells else None
    if rows is None:  # a reference TOML
        t = tomllib.load(open(spec, "rb"))
        rows = t["rows"] + [x for ly in t.get("layer", []) for x in ly["rows"]]
        for k, v in TOK.items():
            rows = [x.replace(k, v) for x in rows]
    if rows:
        r["rows"] = rows
        r["where"] = {ch: where(rows, ch) for ch in WATCH}
json.dump({"cells": cells, "best_of": best_of, "layouts": res}, open(HERE / "results.json", "w"), indent=1)

base = res[best_of["free"]]["scores"]["vim-programmer-typed"] if "free" in best_of else None
print(f"{'layout':18s} {'vim-prog':>9s} {'vs free':>8s} {'prog':>8s} {'layer%':>7s} {'sfb':>5s} {'time':>5s}")
for n, r in sorted(res.items(), key=lambda kv: kv[1]["scores"]["vim-programmer-typed"]):
    s = r["scores"]["vim-programmer-typed"]
    c = r["costs"]["vim-programmer-typed"]
    print(f"{n:18s} {s:9.2f} {100 * (s / base - 1) if base else 0:+7.1f}% {r['scores']['programmer-typed']:8.2f}"
          f" {c['layer_switch']:7.2f} {c['sfb']:5.2f} {c['typing_time']:5.2f}")
print()
for n, r in res.items():
    if "where" in r:
        print(n, "; ".join(f"{k} {v}" for k, v in r["where"].items()))
print()
for t, c in cells.items():
    print(f"{t:12s} {c['corpus']:22s} best {c['best']:.3f} at best {c['at_best']}/{c['n']} spread {c['scores'][-1] / c['scores'][0] - 1:.1%}")
