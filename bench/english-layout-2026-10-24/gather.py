"""Score the English optima and 17 named layouts on every English corpus, with judges.
Usage: python3 -I bench/english-layout-2026-10-24/gather.py   (after grid.sh; needs ./target/release/gecko)
Writes data.json beside this file. Timing judge J_B: ~/projects/gecko-workspace/costfit (held-out typist half).
"""
import json, subprocess, sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
G = str(ROOT / "target/release/gecko")
W = Path.home() / "projects/gecko-workspace/costfit"
CORPORA = ["prose", "prose-web", "prose-chat", "prose-books", "shai", "reddit", "english"]
SETS = {"qwerty": set(",./;"), "apos": set("',./"), "semi": set("',.;")}

# Named layouts: chars as in the layout-ranking study (bench/layout-ranking-2026-10-24/data.json).
NAMED = ["QWERTY", "Dvorak", "Colemak", "Colemak-DH", "Workman", "Halmak", "Canary", "Graphite", "Gallium",
         "Galite", "Sturdy", "Semimak-JQ", "Dhorf", "Focal", "Hands Down (ref)", "MTGAP", "APTv3"]
CLASSIC = {"QWERTY", "Dvorak", "Colemak", "Colemak-DH", "Workman", "Halmak"}
rank_data = {r["name"]: r for r in json.load(open(ROOT / "bench/layout-ranking-2026-10-24/data.json"))["layouts"]}


def set_of(chars):
    p = set(c for c in chars if not c.isalpha())
    return next(k for k, v in SETS.items() if v == p)


def seeds(name):
    return json.load(open(HERE / name))


layouts = []
for n in NAMED:
    r = rank_data[n]
    assert r["geom"] == "ansi-30", n
    layouts.append({"name": n, "chars": r["chars"], "kind": "classic" if n in CLASSIC else "modern"})
for s in ("apos", "qwerty", "semi"):
    d = seeds(f"seeds-prose-{s}.json")
    best = min(d["rows"], key=lambda r: r["score"])
    layouts.append({"name": f"GECKO prose ({s})", "chars": best["chars"], "kind": "gecko", "run": best["run"]})
for c in CORPORA[1:]:
    d = seeds(f"seeds-{c}-apos.json")
    best = min(d["rows"], key=lambda r: r["score"])
    layouts.append({"name": f"GECKO {c}", "chars": best["chars"], "kind": "corpus-opt", "run": best["run"], "corpus": c})
for L in layouts:
    L["set"] = set_of(L["chars"])


def feats(specs, corpus, profile="default"):
    out = subprocess.run([G, "features", "-g", "ansi-30", "-p", profile, "-c", corpus],
                         input="\n".join("raw:" + s for s in specs) + "\n", capture_output=True, text=True,
                         cwd=ROOT, check=True).stdout
    return [json.loads(x) for x in out.splitlines()]


specs = [L["chars"] for L in layouts]
for c in CORPORA:
    for L, f in zip(layouts, feats(specs, c)):
        L.setdefault("scores", {})[c] = f["score"]
        if c == "prose":
            L["costs"] = f["costs"]
for p in ("balanced", "sfb", "engram"):
    for L, f in zip(layouts, feats(specs, "prose", p)):
        L.setdefault("profiles", {})[p] = f["score"]

# Robust rank on prose: named layouts + the three set optima.
robust_names = [L for L in layouts if L["kind"] != "corpus-opt"]
out = subprocess.run([G, "--json", "compare", *["raw:" + L["chars"] for L in robust_names], "-c", "prose",
                      "--robust", "--draws", "2000", "--seed", "1"], capture_output=True, text=True, cwd=ROOT, check=True)
rob = {x["layout"][4:]: x for x in json.loads(out.stdout)["layouts"]}
for L in robust_names:
    x = rob[L["chars"]]
    L["robust"] = {k: x[k] for k in ("rank", "rank_lo", "rank_hi", "mean_rank", "p_first")}

# Held-out timing judge (typist half B; the default profile's weights were fitted on half A).
sys.path.insert(0, str(W))
from judge import Judge  # noqa: E402
JB, JBl = Judge(1, corpus="prose"), Judge(1, corpus="prose", letters=True)
for L in layouts:
    L["J_B"], L["J_B_letters"] = JB(L["chars"]), JBl(L["chars"])

json.dump({"corpora": CORPORA, "layouts": layouts}, open(HERE / "data.json", "w"), indent=1)
for L in sorted(layouts, key=lambda L: L["scores"]["prose"]):
    print(f"{L['name']:24s} {L['set']:6s} {L['scores']['prose']:7.3f}  J_B {L['J_B']:+.4f}")
