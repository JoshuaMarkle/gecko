"""#140 step 2/3: GECKO's ranking of well-known layouts vs community consensus, per corpus.

Usage: python3 -I ranking.py <gecko binary> [profile] > ranking-<profile>.json

Consensus signals (no single published ranking exists; see context/objective.md §Validation):
  tier   community tiers, 1 = best. Tier 1: modern analyzer-era layouts the AKL community
         recommends (Graphite tops AKL votes; Canary, Sturdy, Graphite, APTv3 named as "highly
         regarded"; Gallium, Semimak, Nerps, Maya, Rain, Recurva among cmini's most liked).
         Then Colemak-DH > Colemak (DH fixes Colemak's centre-column LSBs), then the older
         alternatives Dvorak / Workman / Halmak (SFB 2.6–3.1 %, "above 2 % is high", Layouts
         Doc), then QWERTY. Ties within a tier are ties.
  likes  cmini likes (AKL Discord), references matched by name.
"""
import json
import subprocess
import sys

GECKO = sys.argv[1]
PROFILE = sys.argv[2] if len(sys.argv) > 2 else "default"
CORPORA = "english,english-cased,shai,mt-450k,reddit"
# spec, consensus tier, cmini likes (data/community/cmini.json)
LAYOUTS = [
    ("graphite", 1, 77), ("cmini:gallium", 1, 47), ("sturdy", 1, 36), ("canary", 1, 36),
    ("cmini:rain", 1, 29), ("cmini:maya", 1, 21), ("semimak", 1, 20), ("cmini:recurva", 1, 20),
    ("cmini:nerps", 1, 17), ("cmini:aptv3", 1, 6),
    ("colemak-dh", 2, 19), ("colemak", 3, 15),
    ("dvorak", 4, 20), ("workman", 4, 4), ("halmak", 4, 0), ("qwerty", 5, 0),
]


def ranks(xs):
    """Average ranks (1 = smallest), ties share their mean rank."""
    order = sorted(range(len(xs)), key=lambda i: xs[i])
    r = [0.0] * len(xs)
    i = 0
    while i < len(order):
        j = i
        while j + 1 < len(order) and xs[order[j + 1]] == xs[order[i]]:
            j += 1
        for k in range(i, j + 1):
            r[order[k]] = (i + j) / 2 + 1
        i = j + 1
    return r


def spearman(a, b):
    ra, rb = ranks(a), ranks(b)
    ma, mb = sum(ra) / len(ra), sum(rb) / len(rb)
    cov = sum((x - ma) * (y - mb) for x, y in zip(ra, rb))
    va = sum((x - ma) ** 2 for x in ra) ** 0.5
    vb = sum((y - mb) ** 2 for y in rb) ** 0.5
    return cov / (va * vb)


out = subprocess.run([GECKO, "compare", *[s for s, _, _ in LAYOUTS], "--corpora", CORPORA,
                      "-p", PROFILE, "--json"], check=True, capture_output=True, text=True).stdout
report = json.loads(out)
# compare reports display names; map back in input order via a single-corpus run per layout name
names = {}
for spec, _, _ in LAYOUTS:
    one = json.loads(subprocess.run([GECKO, "compare", spec, "-c", "shai", "-p", PROFILE, "--json"],
                                    check=True, capture_output=True, text=True).stdout)
    names[one["layouts"][0]["layout"]] = spec
by_spec = {names[r["layout"]]: r for r in report["layouts"]}

result = {"profile": PROFILE, "geometry": report["geometry"], "corpora": {}}
tiers = [t for _, t, _ in LAYOUTS]
likes = [-l for _, _, l in LAYOUTS]  # more likes = better = smaller
for corpus in report["corpora"]:
    scores = [by_spec[s]["corpora"][corpus]["score"] for s, _, _ in LAYOUTS]
    # cross-tier pairs GECKO orders like the tiers
    agree = total = 0
    wrong = []
    for i, (si, ti, _) in enumerate(LAYOUTS):
        for j, (sj, tj, _) in enumerate(LAYOUTS):
            if ti < tj:
                total += 1
                if scores[i] < scores[j]:
                    agree += 1
                else:
                    wrong.append(f"{sj} ({scores[j]:.2f}) beats {si} ({scores[i]:.2f})")
    result["corpora"][corpus] = {
        "spearman_tier": spearman(scores, tiers),
        "spearman_likes": spearman(scores, likes),
        "spearman_likes_tier1": spearman([scores[i] for i in range(10)], likes[:10]),
        "cross_tier_pairs": total,
        "cross_tier_agree": agree,
        "disagreements": wrong,
        "ranking": [s for _, s in sorted(zip(scores, [s for s, _, _ in LAYOUTS]))],
        "scores": dict(zip([s for s, _, _ in LAYOUTS], scores)),
    }
json.dump(result, sys.stdout, indent=1)
