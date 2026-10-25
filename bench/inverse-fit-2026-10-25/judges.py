"""Held-out empirical timing judges (cost-function study: half B of the Aalto typists; J_B controls
QWERTY practice by pair frequency, J_B+l by pair and letter frequency) on the inward-roll price
layouts and the English-study layouts. Output: judges.json (% vs QWERTY, lower = faster)."""
import json, sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(Path.home() / "projects/gecko-workspace/costfit"))
from judge import Judge  # noqa: E402

JB, JBl = Judge(1, corpus="prose"), Judge(1, corpus="prose", letters=True)
q = "qwertyuiopasdfghjkl;zxcvbnm,./"
qb, qbl = JB(q), JBl(q)
rows = []
res = json.load(open(HERE / "results.json")) if (HERE / "results.json").exists() else {}
for x in res.get("price", []):
    rows.append(("inroll x%d" % x["k"], x["chars"]))
for x in json.load(open(HERE / "layouts.json")):
    rows.append((x["name"], x["chars"]))
out = []
for name, c in rows:
    a, b = 100 * (JB(c) / qb - 1), 100 * (JBl(c) / qbl - 1)
    out.append({"name": name, "chars": c, "J_B": a, "J_B_letters": b})
    print(f"{name:22} J_B {a:+6.2f} %   J_B+l {b:+6.2f} %")
json.dump(out, open(HERE / "judges.json", "w"), indent=1)
