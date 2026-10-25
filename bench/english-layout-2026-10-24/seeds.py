"""Seeds x BLS on one board / corpus / symbol set (as costfit seeds.py, with --from for the set).
usage: python3 -I seeds.py <gecko> <corpus> <set> [seeds=10] [evals=25000000] [holdout=shai] [profile=default] > out.json
set: qwerty (, . / ;), apos (' , . /), semi (' , . ;)"""
import json, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
SETS = {"qwerty": ",./;", "apos": "',./", "semi": "',.;"}
GECKO, CORPUS, SET = sys.argv[1:4]
SEEDS = int(sys.argv[4]) if len(sys.argv) > 4 else 10
EVALS = int(sys.argv[5]) if len(sys.argv) > 5 else 25_000_000
HOLDOUT = sys.argv[6] if len(sys.argv) > 6 else "shai"
PROFILE = sys.argv[7] if len(sys.argv) > 7 else "default"
FROM = "raw:abcdefghijklmnopqrstuvwxyz" + SETS[SET]

def one(seed):
    cmd = [GECKO, "--json", "optimize", "--algo", "bls", "--evals", str(EVALS), "--seed", str(seed), "--threads", "4",
           "-p", PROFILE, "-g", "ansi-30", "-c", CORPUS, "--from", FROM, "--top", "5"]
    if HOLDOUT != CORPUS:
        cmd += ["--holdout", HOLDOUT]
    run = json.loads(subprocess.run(cmd, check=True, capture_output=True, text=True).stdout)["run"]
    b = run["best"][0]
    return {"seed": seed, "run": run["id"], "score": b["score"], "chars": b["chars"],
            "holdout": run.get("holdout", {}).get("best"), "top": [(x["score"], x["chars"]) for x in run["best"]]}

with ThreadPoolExecutor(5) as ex:
    rows = list(ex.map(one, range(1, SEEDS + 1)))
best = min(r["score"] for r in rows)
at = sum(abs(r["score"] - best) < 1e-6 for r in rows)
json.dump({"profile": PROFILE, "board": "ansi-30", "corpus": CORPUS, "set": SET, "evals": EVALS, "best": best,
           "at_best": at, "rows": rows}, sys.stdout, indent=1)
print(f"{CORPUS} {SET}: best {best:.4f}, {at}/{len(rows)} seeds at best, worst {max(r['score'] for r in rows):.4f}", file=sys.stderr)
