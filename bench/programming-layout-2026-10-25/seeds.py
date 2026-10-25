"""Seeds × BLS on corne-36 from a layered start (as bench/english-layout-2026-10-24/seeds.py).
usage: python3 -I seeds.py <gecko> <tag> <start> <corpus> <profile> [seeds=6] [evals=40000000] [pin=""] > out.json
"""
import json, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
GECKO, TAG, START, CORPUS, PROFILE = sys.argv[1:6]
SEEDS = int(sys.argv[6]) if len(sys.argv) > 6 else 6
EVALS = int(sys.argv[7]) if len(sys.argv) > 7 else 40_000_000
PIN = sys.argv[8] if len(sys.argv) > 8 else ""

def one(seed):
    cmd = [GECKO, "--json", "optimize", "--algo", "bls", "--evals", str(EVALS), "--seed", str(seed), "--threads", "4",
           "-p", PROFILE, "-g", "corne-36", "-c", CORPUS, "--from", START, "--top", "3"]
    if PIN:
        cmd += ["--pin", PIN]
    run = json.loads(subprocess.run(cmd, check=True, capture_output=True, text=True).stdout)["run"]
    b = run["best"][0]
    return {"seed": seed, "run": run["id"], "score": b["score"], "chars": b["chars"], "rows": b["rows"]}

with ThreadPoolExecutor(5) as ex:
    rows = list(ex.map(one, range(1, SEEDS + 1)))
best = min(r["score"] for r in rows)
at = sum(abs(r["score"] - best) < 1e-6 for r in rows)
json.dump({"tag": TAG, "start": START, "corpus": CORPUS, "profile": PROFILE, "pin": PIN, "evals": EVALS,
           "best": best, "at_best": at, "rows": rows}, sys.stdout, indent=1)
print(f"{TAG}: best {best:.4f}, {at}/{len(rows)} seeds at best, worst {max(r['score'] for r in rows):.4f}", file=sys.stderr)
