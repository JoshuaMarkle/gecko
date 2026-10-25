"""Score modern layouts under the evidence `default` profile; polish each by steepest descent.
Usage: python3 bench/layout-ranking-2026-10-24/gather.py [out.json]  (needs ./target/release/gecko)
"""
import itertools, json, subprocess, sys
from pathlib import Path

CWD = str(Path(__file__).resolve().parents[2])
G = CWD + "/target/release/gecko"
OUT = sys.argv[1] if len(sys.argv) > 1 else str(Path(__file__).resolve().parent / "data.json")

cm = {l["name"]: l for l in json.load(open(f"{CWD}/data/community/cmini.json"))["layouts"]}
REF = {
    "QWERTY": "qwertyuiopasdfghjkl;zxcvbnm,./",
    "Colemak": "qwfpgjluy;arstdhneiozxcvbkm,./",
    "Colemak-DH": "qwfpbjluy;arstgmneiozxcdvkh,./",
    "Dvorak": "',.pyfgcrlaoeuidhtns;qjkxbmwvz",
    "Workman": "qdrwbjfup;ashtgyneoizxmcvkl,./",
    "Halmak": "wlrbz;qudjshnt,.aeoifmvc/gpxky",
    "Canary": "wlypkzxou;crstbfneiajvdgqmh/,.",
}
# name -> cmini key (ortho unless the cmini board says angle)
CMINI = ["graphite", "gallium", "sturdy", "Semimak-JQ", "rain", "recurva", "Nerps", "vylet",
         "noctum", "stronk", "whorf", "aptv3", "pine-v4", "dhorf", "whirl", "hrt",
         "galite-reference", "ISRT", "CTGAP", "focal", "Hands-Down-Reference", "MTGAP30",
         "Canary", "maya", "panke", "stand", "curtsy"]
NICE = {"graphite": "Graphite", "gallium": "Gallium", "sturdy": "Sturdy", "Semimak-JQ": "Semimak-JQ",
        "rain": "Rain", "recurva": "Recurva", "Nerps": "Nerps", "vylet": "Vylet", "noctum": "Noctum",
        "stronk": "Stronk", "whorf": "Whorf", "aptv3": "APTv3", "pine-v4": "Pine v4", "dhorf": "Dhorf",
        "whirl": "Whirl", "hrt": "HRT", "galite-reference": "Galite", "ISRT": "ISRT", "CTGAP": "CTGAP",
        "focal": "Focal", "Hands-Down-Reference": "Hands Down (ref)", "MTGAP30": "MTGAP",
        "Canary": "Canary (angle)", "maya": "Maya", "panke": "Panke", "stand": "Stand", "curtsy": "Curtsy"}
GECKO = {"GECKO optimum": "wgncvzuo,qdtrsfyheiapblmjxk/;."}

layouts = []  # (name, chars, geometry, source, likes)
for n, c in REF.items():
    layouts.append((n, c, "ansi-30", "classic" if n != "Canary" else "ref (ortho)", None))
for k in CMINI:
    l = cm[k]
    if k == "graphite":  # cmini's 30-key Graphite drops the comma; use the reference rows
        l = dict(l, rows=["bldwz'fouj", "nrtsgyhaei", "qxmcvkp,./"])
    layouts.append((NICE[k], "".join(l["rows"]), "ansi-30-angle" if l["board"] == "angle" else "ansi-30",
                    "cmini", l["likes"]))
for n, c in GECKO.items():
    layouts.append((n, c, "ansi-30", "gecko", None))


def feats(specs, geom="ansi-30", profile="default", corpus="english"):
    out = subprocess.run([G, "features", "-g", geom, "-p", profile, "-c", corpus],
                         input="\n".join("raw:" + s for s in specs) + "\n", capture_output=True,
                         text=True, cwd=CWD, check=True).stdout
    return [json.loads(x) for x in out.splitlines()]


def profile_weights(name):
    import tomllib
    p = tomllib.load(open(f"{CWD}/data/profiles/{name}.toml", "rb"))
    return {k: w / p["scale"][k] for k, w in p["weights"].items()}


W = profile_weights("default")


def polish(chars, geom):
    """Steepest descent over swaps of non-identical keys; returns path of swaps and scores."""
    cur = list(chars)
    score = feats(["".join(cur)], geom)[0]["score"]
    path = []
    while True:
        cands, moves = [], []
        for i, j in itertools.combinations(range(30), 2):
            t = cur[:]; t[i], t[j] = t[j], t[i]
            cands.append("".join(t)); moves.append((cur[i], cur[j]))
        res = feats(cands, geom)
        b = min(range(len(res)), key=lambda k: res[k]["score"])
        if res[b]["score"] >= score - 1e-9:
            break
        path.append({"swap": "".join(moves[b]), "score": res[b]["score"]})
        cur = list(cands[b]); score = res[b]["score"]
    return path, "".join(cur)


rows = []
for name, chars, geom, src, likes in layouts:
    f = feats([chars], geom)[0]
    bal = feats([chars], geom, "balanced")[0]["score"]
    corp = {c: feats([chars], geom, "default", c)[0]["score"] for c in ["shai", "reddit", "mt-450k"]}
    contrib = {k: W[k] * f["costs"][k] for k in W}
    path, final = polish(chars, geom)
    moved = sum(a != b for a, b in zip(chars, final))
    fin = feats([final], geom)[0]
    rows.append(dict(name=name, chars=chars, geom=geom, src=src, likes=likes, score=f["score"],
                     balanced=bal, corpora=corp, costs=f["costs"], contrib=contrib, polish=path,
                     polished=final, polished_score=fin["score"], moved=moved,
                     polished_costs={k: fin["costs"][k] for k in ["E01", "E05", "E04", "B01", "S01"]}))
    print(f"{name:22s} {f['score']:7.3f} bal {bal:7.3f} polish {len(path)} swaps -> {fin['score']:.3f}", flush=True)

json.dump({"weights": W, "layouts": rows}, open(OUT, "w"), indent=1)
