"""#141 step 2: where the gain comes from. Per-metric weighted cost, search winner vs best known.

Usage: python3 -I breakdown.py <gecko binary> > breakdown.json   (Markdown tables on stderr)

For each 25M-eval row of summary.json: the winner (`run:<id>`) and the best known layout of the
same symbol set, scored on the search corpus. `delta` = winner − known (negative = winner better);
rewards (rolls, alternates) are negative costs, so a larger reward is a negative delta too.
"""
import json
import subprocess
import sys

GECKO = sys.argv[1]
summary = [s for s in json.load(open("summary.json")) if s["evals"] == 25_000_000]


def metrics(layout, board, corpus):
    out = subprocess.run([GECKO, "--json", "score", layout, "-g", board, "-c", corpus],
                         check=True, capture_output=True, text=True).stdout
    return {m["id"]: m for m in json.loads(out)["breakdown"]["metrics"] if m["weight"]}


report = []
for s in summary:
    win = metrics(f"run:{s['winner_run']}", s["board"], s["corpus"])
    known = metrics(s["best_known"]["layout"], s["board"], s["corpus"])
    rows = [{"id": k, "name": m["name"], "unit": m["unit"], "winner": m["value"], "known": known[k]["value"],
             "delta": m["weighted"] - known[k]["weighted"]} for k, m in win.items()]
    rows.sort(key=lambda r: r["delta"])
    report.append({k: s[k] for k in ("board", "corpus", "set")} | {"known": s["best_known"]["layout"], "metrics": rows})
    print(f"\n{s['board']} / {s['corpus']} / {s['set']}: winner vs {s['best_known']['layout']}", file=sys.stderr)
    print("| metric | winner | known | Δ weighted |\n|---|---:|---:|---:|", file=sys.stderr)
    for r in rows:
        if abs(r["delta"]) >= 0.05:
            print(f"| {r['id']} {r['name']} | {r['winner']:.2f} | {r['known']:.2f} | {r['delta']:+.2f} |", file=sys.stderr)
json.dump(report, sys.stdout, indent=1)
