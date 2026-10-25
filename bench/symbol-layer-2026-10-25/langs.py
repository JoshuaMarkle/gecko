"""Score layouts on each typed-code corpus (free profile) → langs.json; prints a gap table per corpus.
usage: python3 -I langs.py <frame> <layout.toml>...
"""
import json, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
CORPORA = ["code-python-typed", "code-rust-typed", "code-js-typed", "code-c-typed", "code-shell-typed", "code-lua-typed", "markdown"]
frame, *layouts = sys.argv[1:]
prof = HERE / "profiles" / f"free-{'vim' if frame.endswith('vim') else 'prog'}.toml"


def one(job):
    lay, corpus = job
    out = subprocess.run([str(ROOT / "target/release/gecko"), "--json", "score", lay, "-g", "corne-36", "-c", corpus, "-p", str(prof)],
                         check=True, capture_output=True, text=True, cwd=ROOT).stdout
    return lay, corpus, json.loads(out)["breakdown"]["score"]


jobs = [(str(Path(l).resolve()), c) for l in layouts for c in CORPORA]
with ThreadPoolExecutor(8) as ex:
    res = list(ex.map(one, jobs))
tab = {}
for lay, c, s in res:
    tab.setdefault(Path(lay).name, {})[c] = s
path = HERE / "langs.json"
allres = json.load(open(path)) if path.exists() else {}
allres[frame] = tab
json.dump(allres, open(path, "w"), indent=1)
print("| layout | " + " | ".join(c.replace("code-", "").replace("-typed", "") for c in CORPORA) + " |")
print("|---|" + "---|" * len(CORPORA))
best = {c: min(t[c] for t in tab.values()) for c in CORPORA}
for name, t in sorted(tab.items(), key=lambda kv: sum(kv[1][c] - best[c] for c in CORPORA)):
    print(f"| {name} | " + " | ".join(f"{t[c] - best[c]:+.2f}" for c in CORPORA) + " |")
