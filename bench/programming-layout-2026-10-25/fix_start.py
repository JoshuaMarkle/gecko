"""Complete `start` in config.json of runs made before 2026-10-25 01:47, when a layered start was
recorded as its base layer only (fixed in gecko-core run.rs). The flattened start comes from a
1-evaluation run of the same start TOML (the fixed binary records it whole), then the run's
config.json is patched; `gecko verify` re-runs the search from it and checks best.json.
usage: python3 -I fix_start.py <seeds-*.json>...   (patches runs/<id> of every seed row)
"""
import json, shutil, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
G = str(ROOT / "target/release/gecko")
flat = {}
for f in sys.argv[1:]:
    d = json.load(open(f))
    start = str((Path(f).parent / d["start"]).resolve())
    if start not in flat:
        out = json.loads(subprocess.run([G, "--json", "optimize", "--algo", "hc", "--evals", "1", "--seed", "1", "--threads", "1",
                                         "-g", "corne-36", "-c", d["corpus"], "-p", str((Path(f).parent / d["profile"]).resolve()),
                                         "--from", start], capture_output=True, text=True, check=True, cwd=ROOT).stdout)
        tmp = ROOT / "runs" / out["run"]["id"]
        flat[start] = json.load(open(tmp / "config.json"))["start"]
        shutil.rmtree(tmp)
    for r in d["rows"]:
        cfg_path = ROOT / "runs" / r["run"] / "config.json"
        cfg = json.load(open(cfg_path))
        if cfg["start"] != flat[start]:
            assert flat[start].startswith(cfg["start"]), (r["run"], cfg["start"])
            cfg["start"] = flat[start]
            cfg_path.write_text(json.dumps(cfg, indent=2, ensure_ascii=False))
            print("patched", r["run"])
