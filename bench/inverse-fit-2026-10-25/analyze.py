"""Summarize the inverse fits (fits-sigma*.json) and the inward-roll price runs (price/*.json)
into results.json and a printed report."""
import json
import subprocess
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
G = str(ROOT / "target/release/gecko")
LAY = json.load(open(HERE / "layouts.json"))
KIND = {x["name"]: x["kind"] for x in LAY}


def feats(chars_list, corpus="prose"):
    out = subprocess.run([G, "features", "-g", "ansi-30", "-p", "default", "-c", corpus],
                         input="\n".join("raw:" + c for c in chars_list) + "\n",
                         capture_output=True, text=True, cwd=ROOT, check=True).stdout
    return {json.loads(l)["chars"]: json.loads(l) for l in out.splitlines()}


def summarize(path):
    d = json.load(open(path))
    fits = d["fits"]
    ids = [w["id"] for w in fits[0]["weights"]]
    rows = []
    for f in fits:
        m = {w["id"]: w for w in f["weights"]}
        rows.append({
            "name": f["layout"], "kind": KIND.get(f["layout"], "?"), "chars": f["chars"],
            "base_gap_pct": f["base"]["gap_pct"], "base_keys": f["base"]["keys"],
            "fit_gap_pct": f["fitted"]["gap_pct"], "fit_keys": f["fitted"]["keys"],
            "self_consistent": f["self_consistent"], "converged": f["converged"], "rounds": len(f["rounds"]),
            "log_distance": f["log_distance"],
            "mult": {k: m[k]["multiplier"] for k in ids},
            "lo": {k: m[k]["lo"] for k in ids}, "hi": {k: m[k]["hi"] for k in ids},
            "informative": {k: m[k]["sd_ratio"] <= 0.8 for k in ids},
            "at_bound": [k for k in ids if m[k]["at_bound"]],
            "best_found": f["fitted"]["best"],
        })
    return {"ids": ids, "rows": rows, "cross": d["cross"]}


res = {}
for s in ["0.5", "1.0"]:
    p = HERE / f"fits-sigma{s}.json"
    if p.exists():
        res[s] = summarize(p)

# Roll and SFS levels of every layout (natural units), to set beside the fitted weights.
allc = [x["chars"] for x in LAY]
F = feats(allc)
for s, r in res.items():
    for row in r["rows"]:
        c = F[row["chars"]]["costs"]
        row["values"] = {k: -c[k] if k in ("inroll", "outroll", "hand_switch", "alternate", "home_row") else c[k]
                         for k in ("inroll", "outroll", "sfb", "sfs", "hand_switch", "engram", "effort")}

for s, r in res.items():
    ids = r["ids"]
    print(f"\n=== prior sigma {s} ===")
    print(f"{'layout':22} {'kind':8} {'base%':>7} {'fit%':>6} {'keys':>4} {'ok':>3} {'dist':>5}  inroll  outroll  in/out  sfs     sfb    engram")
    for x in r["rows"]:
        m = x["mult"]
        print(f"{x['name']:22} {x['kind']:8} {x['base_gap_pct']:7.1f} {x['fit_gap_pct']:6.2f} {x['fit_keys']:4} "
              f"{'Y' if x['self_consistent'] else ('c' if x['converged'] else '-'):>3} {x['log_distance']:5.2f}  "
              f"{m['inroll']:6.2f}  {m['outroll']:6.2f}  {m['inroll'] / m['outroll']:6.2f}  {m['sfs']:6.2f}  {m['sfb']:5.2f}  {m['engram']:5.2f}")
    # Group means of log multipliers, modern layouts.
    mod = [x for x in r["rows"] if x["kind"] == "modern"]
    print("\nmodern layouts: geometric-mean multiplier per weight (n informative / n), sorted by |log|")
    stats = []
    for k in ids:
        lm = np.array([np.log(x["mult"][k]) for x in mod])
        inf = sum(x["informative"][k] for x in mod)
        up = sum(v > 0 for v in lm)
        stats.append((k, float(np.exp(lm.mean())), int(inf), int(up), len(mod)))
    stats.sort(key=lambda t: -abs(np.log(t[1])))
    for k, gm, inf, up, n in stats:
        print(f"  {k:14} ×{gm:6.2f}   informative {inf}/{n}   up in {up}/{n}")
    r["modern_summary"] = [dict(zip(["id", "geomean", "informative", "up", "n"], t)) for t in stats]
    # Correlation of fitted in/out ratio with the layout's in/out roll levels.
    ratio = np.array([np.log(x["mult"]["inroll"] / x["mult"]["outroll"]) for x in mod])
    lev = np.array([np.log(x["values"]["inroll"] / x["values"]["outroll"]) for x in mod])
    r["ratio_vs_level_r"] = float(np.corrcoef(ratio, lev)[0, 1])
    print(f"\nmodern: corr(log fitted in/out multiplier ratio, log in/out roll level ratio) = {r['ratio_vs_level_r']:.2f}")
    # Cross matrix: mean gap of modern layouts under each other's fits.
    names = r["cross"]["layouts"]
    gap = np.array(r["cross"]["gap_pct"])
    idx = [names.index(x["name"]) for x in mod]
    sub = gap[np.ix_(idx, idx)]
    off = sub[~np.eye(len(idx), dtype=bool)]
    print(f"modern under other modern fits: median gap {np.median(off):.1f} %, own fits median {np.median(np.diag(sub)):.2f} %")

# Price of inward rolls.
price = []
for p in sorted((HERE / "price").glob("x*-s*.json")):
    d = json.load(open(p))
    best = d["run"]["best"][0]
    price.append({"k": int(p.stem.split("-")[0][1:]), "seed": int(p.stem.split("-s")[1]), "chars": best["chars"], "score": best["score"]})
if price:
    F2 = feats([x["chars"] for x in price] + ["bldcvjfou,nrtsgyhaeixqmwzkp';."])
    J = {}
    out = subprocess.run([G, "--json", "judge", "-g", "ansi-30", "-c", "prose"] + ["raw:" + x["chars"] for x in price],
                         capture_output=True, text=True, cwd=ROOT, check=True).stdout
    for l in json.loads(out)["layouts"]:
        J[l["layout"][4:]] = {k: v["value"] for k, v in l["judges"].items()}
    best_by_k = {}
    for x in price:
        if x["k"] not in best_by_k or x["score"] < best_by_k[x["k"]]["score"]:
            best_by_k[x["k"]] = x
    base_costs = F2[best_by_k[1]["chars"]]["costs"]
    print("\nprice of inward rolls (best of 4 seeds per weight; default-profile score and judges)")
    print(f"{'inroll×':>7} {'default score':>13} {'Δ%':>6} {'inroll%':>7} {'outroll%':>8} {'sfb%':>5} {'sfs%':>5} {'time':>6} {'engram':>6}  layout")
    rows = []
    for k in sorted(best_by_k):
        x = best_by_k[k]
        c = F2[x["chars"]]["costs"]
        ds = F2[x["chars"]]["score"]
        row = {"k": k, "chars": x["chars"], "default_score": ds, "inroll": -c["inroll"], "outroll": -c["outroll"],
               "sfb": c["sfb"], "sfs": c["sfs"], "typing_time": J[x["chars"]]["typing_time"], "engram": J[x["chars"]]["engram"]}
        rows.append(row)
        print(f"{k:7} {ds:13.3f} {100 * (ds / rows[0]['default_score'] - 1):6.1f} {row['inroll']:7.1f} {row['outroll']:8.1f} "
              f"{row['sfb']:5.2f} {row['sfs']:5.2f} {row['typing_time']:6.2f} {row['engram']:6.2f}  {x['chars']}")
    res["price"] = rows

json.dump(res, open(HERE / "results.json", "w"), indent=1)
