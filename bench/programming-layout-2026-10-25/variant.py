"""Write a layered corne-36 layout TOML from a run's best layout, optionally changed.
usage: python3 -I variant.py <run id> <out.toml> [digits-row] [oneshot]
digits-row: digits 1..0 on the Sym layer's home row (whatever sat there takes the freed slots).
oneshot: both layers one-shot instead of hold.
"""
import json, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
run, out, *opts = sys.argv[1:]
d = ROOT / "runs" / run
R = [list(r) for r in json.load(open(d / "best.json"))["best"][0]["rows"]]  # base, sym, num: 4 rows each
shifted = json.load(open(d / "config.json")).get("shifted", [])
E = "·"
if "digits-row" in opts:
    for col, dgt in enumerate("1234567890"):  # Sym layer home row = R[5]
        r = next(k for k, row in enumerate(R) if dgt in row)
        c = R[r].index(dgt)
        R[r][c], R[5][col] = R[5][col], R[r][c]
TOK = {"⏎": "{enter}", "␣": "{space}", "⌫": "{bspc}", "⇧": "{shift}", "①": "{layer1}", "②": "{layer2}", "⎋": "{esc}"}
def row(cs):
    return json.dumps("".join(TOK.get(c, c) for c in cs), ensure_ascii=False)
def rows(layer):
    return "[" + ", ".join(row(r) for r in R[4 * layer:4 * layer + 4]) + "]"
mode = '\nmode = "oneshot"' if "oneshot" in opts else ""
text = f'''name = "{(run + ' ' + ' '.join(opts)).strip()}"
geometry = "corne-36"
source = "bench/programming-layout-2026-10-25/variant.py {run} {' '.join(opts)}"
rows = {rows(0)}
shifted = {json.dumps(shifted, ensure_ascii=False)}

[[layer]]
name = "sym"{mode}
rows = {rows(1)}

[[layer]]
name = "num"{mode}
rows = {rows(2)}
'''
Path(out).write_text(text)
print(out)
