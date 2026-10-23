"""#141 step 1: 30 seeds x BLS at fixed --evals budgets, per board and corpus.

Usage: python3 -I run.py <gecko binary> <board> <corpus> [budgets] [start] > seeds-<board>-<corpus>[-<set>].json

Each seed is one `gecko optimize --algo bls --evals N --seed s --threads 4` run (deterministic
for a given seed and thread count, so `gecko verify` replays it). The search corpus's partner
(english <-> shai) is the holdout. On corne-36 the thumb keys are pinned to the board's fills,
so the search places the same 30 symbols the reference layouts place. `start` (default
qwerty) fixes the symbol set: `raw:abc…z',./` searches Graphite's and Semimak's symbols.
"""
import json
import subprocess
import sys

GECKO, BOARD, CORPUS = sys.argv[1:4]
BUDGETS = [int(b) for b in (sys.argv[4] if len(sys.argv) > 4 else "1000000,5000000,25000000").split(",")]
SEEDS = range(1, 31)
START = sys.argv[5] if len(sys.argv) > 5 else "qwerty"
HOLDOUT = {"english": "shai", "shai": "english"}[CORPUS]
PINS = ["--pin", "{space}{enter}{bspc}{tab}{esc}{none}"] if BOARD == "corne-36" else []

out = {"board": BOARD, "corpus": CORPUS, "holdout": HOLDOUT, "algo": "bls", "threads": 4,
       "pins": PINS[1] if PINS else "", "start": START, "budgets": {}}
for evals in BUDGETS:
    rows = []
    for seed in SEEDS:
        cmd = [GECKO, "--json", "optimize", "--algo", "bls", "--evals", str(evals), "--seed", str(seed),
               "--threads", "4", "-g", BOARD, "-c", CORPUS, "--holdout", HOLDOUT, "--top", "3", "--from", START, *PINS]
        run = json.loads(subprocess.run(cmd, check=True, capture_output=True, text=True).stdout)["run"]
        best = run["best"][0]
        rows.append({"seed": seed, "run": run["id"], "score": best["score"], "chars": best["chars"],
                     "holdout": run["holdout"]["best"], "evals": run["evals"],
                     "elapsed_ms": run["elapsed_ms"], "bound": run.get("bound", {}).get("total", {}).get("lower")})
        print(f"{BOARD} {CORPUS} {evals} seed {seed}: {best['score']:.4f}", file=sys.stderr)
    out["budgets"][str(evals)] = rows
json.dump(out, sys.stdout, indent=1)
