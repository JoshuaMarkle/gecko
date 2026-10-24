"""Seeds x BLS under a profile, as Phase 2's run.py (bench/phase2-2026-10-23) but parallel and with -p.
usage: python3 -I seeds.py <gecko> <profile> <board> <corpus> [seeds=10] [evals=25000000] > out.json"""
import json, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
GECKO, PROFILE, BOARD, CORPUS = sys.argv[1:5]
SEEDS = int(sys.argv[5]) if len(sys.argv) > 5 else 10
EVALS = int(sys.argv[6]) if len(sys.argv) > 6 else 25_000_000
HOLDOUT = {"english": "shai", "shai": "english"}[CORPUS]
PINS = ["--pin", "{space}{enter}{bspc}{tab}{esc}{none}"] if BOARD == "corne-36" else []

def one(seed):
    cmd = [GECKO, "--json", "optimize", "--algo", "bls", "--evals", str(EVALS), "--seed", str(seed), "--threads", "4",
           "-p", PROFILE, "-g", BOARD, "-c", CORPUS, "--holdout", HOLDOUT, "--top", "3", *PINS]
    run = json.loads(subprocess.run(cmd, check=True, capture_output=True, text=True).stdout)["run"]
    b = run["best"][0]
    return {"seed": seed, "run": run["id"], "score": b["score"], "chars": b["chars"], "holdout": run["holdout"]["best"]}

with ThreadPoolExecutor(5) as ex:
    rows = list(ex.map(one, range(1, SEEDS + 1)))
best = min(r["score"] for r in rows)
json.dump({"profile": PROFILE, "board": BOARD, "corpus": CORPUS, "evals": EVALS, "best": best,
           "at_best": sum(abs(r["score"] - best) < 1e-6 for r in rows), "rows": rows}, sys.stdout, indent=1)
print(f"{PROFILE} {BOARD} {CORPUS}: best {best:.4f}, {sum(abs(r['score'] - best) < 1e-6 for r in rows)}/{len(rows)} seeds at best, worst {max(r['score'] for r in rows):.4f}", file=sys.stderr)
