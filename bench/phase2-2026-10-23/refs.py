"""#141: scores of the best known layouts, per board, corpus and symbol set (default profile).

Usage: python3 -I refs.py <gecko binary> > refs.json

A layout's score depends on which four punctuation symbols share the 30 keys with the letters
(an empty key or a rarer symbol drops n-grams from the cost), so references are grouped by
symbol set and each group is compared with a search over the same symbols:
  semi  ' , . ;   Sturdy
  apos  ' , . /   Graphite, Semimak
  qwerty , . / ;  Canary (and QWERTY, the optimizer's default start)
"Best cmini" is the best-scoring layout of data/community/cmini.json in each group: 30 distinct
symbols with all 26 letters and no empty key, angle-mod layouts excluded (their fingering is
not the board's).
"""
import json
import string
import subprocess
import sys

GECKO = sys.argv[1]
BOARDS = ["ansi-30", "corne-36"]
CORPORA = "english,shai"
SETS = {"',.;": "semi", "',./": "apos", ",./;": "qwerty"}
NAMED = {"graphite": "apos", "semimak": "apos", "sturdy": "semi", "canary": "qwerty"}


def symbol_set(chars):
    if len(chars) != 30 or len(set(chars)) != 30 or not set(string.ascii_lowercase) <= set(chars):
        return None
    return SETS.get("".join(sorted(set(chars) - set(string.ascii_lowercase))))


cmini = [(l["name"], symbol_set("".join(l["rows"]))) for l in
         json.load(open("../../data/community/cmini.json"))["layouts"] if l["board"] != "angle"]
cmini = [(n, s) for n, s in cmini if s]

result = {"profile": "default", "cmini_in_sets": {s: sum(1 for _, x in cmini if x == s) for s in SETS.values()},
          "boards": {}}
for board in BOARDS:
    def compare(spec):
        p = subprocess.run([GECKO, "--json", "compare", spec, "--corpora", CORPORA, "-g", board],
                           capture_output=True, text=True)
        return json.loads(p.stdout)["layouts"][0]["corpora"] if p.returncode == 0 else None
    named = {n: {"set": s, **{c: v["score"] for c, v in compare(n).items()}} for n, s in NAMED.items()}
    scored = [(n, s, compare(f"cmini:{n}")) for n, s in cmini]
    best = {}
    for group in SETS.values():
        best[group] = {}
        for corpus in CORPORA.split(","):
            top = sorted((r[corpus]["score"], n) for n, s, r in scored if r and s == group)[:5]
            best[group][corpus] = [{"layout": n, "score": v} for v, n in top]
    result["boards"][board] = {"named": named, "best_cmini": best}
json.dump(result, sys.stdout, indent=1)
