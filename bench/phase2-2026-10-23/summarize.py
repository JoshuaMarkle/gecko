"""#141: summarize seeds-*.json against refs.json.

Usage: python3 -I summarize.py > summary.json   (also prints Markdown tables to stderr)

Per board / corpus / symbol set / budget: best, median, worst, stdev over 30 seeds, how many
seeds reach the best score, distinct best layouts, and the margin to the best known layout of
the same symbol set (named references and best cmini). Holdout: the search winner's score on
the partner corpus vs the best known layout's score there.
"""
import glob
import json
import statistics
import sys

refs = json.load(open("refs.json"))
summary = []
for path in sorted(glob.glob("seeds-*.json")):
    d = json.load(open(path))
    group = "qwerty" if d.get("start", "qwerty") == "qwerty" else {"/": "apos", ";": "semi"}[d["start"][-1]]
    board = refs["boards"][d["board"]]
    known = [(n, v[d["corpus"]], v[d["holdout"]]) for n, v in board["named"].items() if v["set"] == group]
    cm = board["best_cmini"][group]
    known.append(("cmini:" + cm[d["corpus"]][0]["layout"], cm[d["corpus"]][0]["score"], None))
    best_known = min(known, key=lambda k: k[1])
    best_holdout_known = min([v for _, _, v in known if v is not None] + [cm[d["holdout"]][0]["score"]])
    for evals, rows in d["budgets"].items():
        scores = [r["score"] for r in rows]
        top = min(scores)
        win = min(rows, key=lambda r: r["score"])
        summary.append({
            "board": d["board"], "corpus": d["corpus"], "set": group, "evals": int(evals), "seeds": len(rows),
            "best": top, "median": statistics.median(scores), "worst": max(scores),
            "stdev": statistics.pstdev(scores), "hits": sum(1 for s in scores if s - top < 1e-9 * abs(top)),
            "distinct": len({r["chars"] for r in rows}), "winner": win["chars"], "winner_run": win["run"],
            "winner_holdout": win["holdout"], "best_holdout_known": best_holdout_known,
            "best_known": {"layout": best_known[0], "score": best_known[1]},
            "margin_best_pct": 100 * (best_known[1] - top) / best_known[1],
            "margin_worst_pct": 100 * (best_known[1] - max(scores)) / best_known[1],
            "bound_lower": win["bound"], "mean_elapsed_s": statistics.mean(r["elapsed_ms"] for r in rows) / 1000,
        })
json.dump(summary, sys.stdout, indent=1)

print("| board | corpus | set | evals | best | median | worst | hits/30 | distinct | best known | margin (best / worst seed) | holdout: winner vs best known |", file=sys.stderr)
print("|---|---|---|---:|---:|---:|---:|---:|---:|---|---|---|", file=sys.stderr)
for s in summary:
    print(f"| {s['board']} | {s['corpus']} | {s['set']} | {s['evals'] // 1_000_000}M | {s['best']:.3f} | {s['median']:.3f} | "
          f"{s['worst']:.3f} | {s['hits']} | {s['distinct']} | {s['best_known']['layout']} {s['best_known']['score']:.3f} | "
          f"{s['margin_best_pct']:.1f} % / {s['margin_worst_pct']:.1f} % | {s['winner_holdout']:.3f} vs {s['best_holdout_known']:.3f} |",
          file=sys.stderr)
