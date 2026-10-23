"""#140 step 1, second analyzer: GECKO vs Oxeylyzer on shai.

Usage: python3 -I oxey_compare.py <oxeylyzer checkout> <numbers.json> > oxeylyzer.json

Runs Oxeylyzer's REPL `analyze` on the 10 reference layouts with the same key rows and
standard fingering as numbers.json (ortho board, `traditional` fingering), on Oxeylyzer's own
shai data (`static/language_data/shai.json`). Columns per stat:
  oxey       Oxeylyzer as shipped (trigram_precision 1000: only the top 1000 trigrams count)
  oxey_full  trigram_precision 1e6 (every trigram)
  aligned    oxey_full divided by the share of n-grams fully on the layout (GECKO's denominator;
             Oxeylyzer's includes digits, `-`, `=` and its shift symbol `⇑`)
  gecko_full GECKO, trigram_coverage 1.0 (from numbers.json)
Stats that the two tools split differently are compared as sums (see STATS).
"""
import json
import os
import re
import subprocess
import sys
import tempfile

OX, NUMBERS = sys.argv[1], sys.argv[2]
DATA = f"{OX}/static/language_data/shai.json"
# GECKO metric ids -> Oxeylyzer output lines summed; n-gram kind for the denominator
STATS = {
    "sfb": (["B01"], ["Sfb"], "bigrams"),
    "sfs": (["S01"], ["Dsfb"], "skipgrams"),
    # Oxeylyzer puts same-key ends (`ere`) in Alternates (sfs), GECKO in T01: compare the sum
    "alternate+alt-sfs": (["T01", "T02"], ["Alternates", "Alternates (sfs)"], "trigrams"),
    "alt-sfs": (["T02"], ["Alternates (sfs)"], "trigrams"),
    "roll-in": (["T03"], ["Inrolls"], "trigrams"),
    "roll-out": (["T04"], ["Outrolls"], "trigrams"),
    "onehand": (["T05"], ["Onehands"], "trigrams"),
    "redirect": (["T06"], ["Redirects", "Redirects Sfs"], "trigrams"),
    "bad-redirect": (["T07"], ["Bad Redirects", "Bad Redirects Sfs"], "trigrams"),
    "sft": (["T08"], ["Sft"], "trigrams"),
}

with open(NUMBERS) as f:
    numbers = json.load(f)
with open(DATA) as f:
    corpus = json.load(f)


def run(layouts, precision):
    """Analyze every layout in a throwaway Oxeylyzer data dir; returns {layout: {line: %}}."""
    root = tempfile.mkdtemp()
    os.makedirs(f"{root}/static/language_data")
    os.makedirs(f"{root}/static/layouts/val")
    open(f"{root}/.data-version", "w").close()  # skip Oxeylyzer's resource download
    os.symlink(DATA, f"{root}/static/language_data/shai.json")
    with open(f"{OX}/config.toml") as src:
        config = src.read()
    config = re.sub(r"layouts = \[[^\]]*\]", 'layouts = ["./static/layouts/val/*.dof"]', config)
    config = re.sub(r"trigram_precision = \d+", f"trigram_precision = {precision}", config)
    with open(f"{root}/config.toml", "w") as f:
        f.write(config)
    for name, rows in layouts.items():
        main = [" ".join(r[:5]) + "  " + " ".join(r[5:]) for r in rows]
        with open(f"{root}/static/layouts/val/{name}.dof", "w") as f:
            json.dump({"name": name, "board": "ortho", "fingering": "traditional",
                       "layers": {"main": main}}, f)
    script = "".join(f"a {name}\n" for name in layouts) + "q\n"
    out = subprocess.run([f"{OX}/target/release/main"], input=script, capture_output=True,
                         text=True, check=True, cwd=root, env={**os.environ, "OXEYLYZER_DATA_DIR": root},
                         timeout=600).stdout
    out = re.sub(r"\x1b\[[0-9;]*m", "", out)
    result, current = {}, None
    for line in out.splitlines():
        if line.strip() in layouts:
            current = result.setdefault(line.strip(), {})
        elif current is not None and (m := re.match(r"^([A-Za-z ()]+):\s+(-?[\d.]+)%$", line)):
            current[m.group(1).strip()] = float(m.group(2))
    subprocess.run(["rm", "-rf", root], check=True)
    return result


def on_layout(rows, kind):
    keys = set("".join(rows))
    return sum(p for g, p in corpus[kind].items() if all(c in keys for c in g)) / 100


layouts = {l["layout"]: l["rows"] for l in numbers["layouts"]}
gecko = {l["layout"]: l for l in numbers["layouts"]}
shipped, full = run(layouts, 1000), run(layouts, 1_000_000)
STAT_OF = {"B01": "sfb", "S01": "sfs", "T01": "alternate", "T02": "alt-sfs", "T03": "roll-in",
           "T04": "roll-out", "T05": "onehand", "T06": "redirect", "T07": "bad-redirect", "T08": "sft"}
result = []
for name, rows in layouts.items():
    row = {"layout": name, "rows": rows}
    for stat, (gids, lines, kind) in STATS.items():
        share = on_layout(rows, kind)
        row[stat] = {
            "oxey": sum(shipped[name][l] for l in lines),
            "oxey_full": sum(full[name][l] for l in lines),
            "aligned": sum(full[name][l] for l in lines) / share,
            "gecko_full": sum(gecko[name][STAT_OF[g]]["gecko_full"] for g in gids),
            "on_layout_share": share,
        }
    result.append(row)

json.dump({"corpus": "shai", "oxeylyzer_commit": subprocess.run(
    ["git", "-C", OX, "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip(),
    "layouts": result}, sys.stdout, indent=1)
