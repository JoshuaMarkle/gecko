"""#140 step 1: GECKO's SFB / SFS / trigram numbers vs cmini's analyzer on shai.

Usage: python3 -I cmini_compare.py <cmini checkout> <gecko binary> > numbers.json

For every reference layout (30 keys, standard ortho fingering, the same key rows in both
tools) it reports three columns per stat:
  cmini    cmini's own analyzer (util/analyzer.py logic, copied below) on its shai JSON
  aligned  the same definition, but n-grams lowercased and kept only when every symbol is on
           the layout, so the denominator matches GECKO's ("% of n-grams on the layout")
  matched  aligned, plus GECKO's definition differences: x?x trigrams (first == third key)
           get their finger class instead of cmini's "sfR"; one-hand trigrams whose ends share
           a finger (cmini dsfb-red) are redirects (bad without an index finger); alternations
           ending on the same key are alternates; SFS counts every in-word same-finger skip-1
           pair (any middle symbol) instead of the dsfb trigram classes
  words    matched, on n-grams counted from cmini's shai words.json instead of its
           bigrams/trigrams.json (GECKO's shai is imported from words.json); SFS here is
           every skip-1 pair of words.json, in-word plus across one space (x␣z, estimated
           from word ends and starts like `gecko corpus import`)
  gecko    `gecko score` with the default profile (trigram_coverage 0.99)
  gecko_full  the same with trigram_coverage = 1.0 (no rare trigrams dropped)
"""
import json
import os
import subprocess
import sys
import tempfile
from collections import defaultdict

CMINI, GECKO = sys.argv[1], sys.argv[2]
LAYOUTS = ["qwerty", "dvorak", "colemak", "colemak-dh", "workman", "halmak", "canary",
           "semimak", "sturdy", "graphite"]
FINGERS = ["LP", "LR", "LM", "LI", "LI", "RI", "RI", "RM", "RR", "RP"]
# cmini class(es) -> GECKO metric id
STATS = {
    "sfb": (["sfb"], "B01"),
    "sfs": (["dsfb-red", "dsfb-alt"], "S01"),  # matched: "skip-sfs"
    "alternate": (["alternate"], "T01"),
    "alt-sfs": (["dsfb-alt"], "T02"),
    "roll-in": (["roll-in"], "T03"),
    "roll-out": (["roll-out"], "T04"),
    "onehand": (["oneh-in", "oneh-out"], "T05"),
    "redirect": (["redirect"], "T06"),
    "bad-redirect": (["bad-redirect"], "T07"),
    "sft": (["sfT", "sft"], "T08"),
}

with open(f"{CMINI}/table.json") as f:
    TABLE = json.load(f)
with open(f"{CMINI}/corpora/shai/bigrams.json") as f:
    BIGRAMS = json.load(f)
with open(f"{CMINI}/corpora/shai/trigrams.json") as f:
    TRIGRAMS = json.load(f)
with open(f"{CMINI}/corpora/shai/words.json") as f:
    WORDS = json.load(f)
# In-word n-grams of words.json (what `gecko corpus import` reads): isolates data from definitions
WORD_BI, WORD_TRI, WORD_SKIP = defaultdict(int), defaultdict(int), defaultdict(float)
ENDS, STARTS, N_WORDS = defaultdict(int), defaultdict(int), 0
for w, n in WORDS.items():
    w = w.lower()
    N_WORDS += n
    ENDS[w[-1]] += n
    STARTS[w[0]] += n
    for i in range(len(w) - 1):
        WORD_BI[w[i:i + 2]] += n
    for i in range(len(w) - 2):
        WORD_TRI[w[i:i + 3]] += n
        WORD_SKIP[w[i] + w[i + 2]] += n
# Skip-1 pairs across one space (x␣z), consecutive words independent, as GECKO estimates them
for x, a in ENDS.items():
    for z, b in STARTS.items():
        WORD_SKIP[x + z] += a * b / N_WORDS


def fingers_of(rows):
    return {c: FINGERS[col] for row in rows for col, c in enumerate(row)}


def cmini_sfb(fingers, grams):
    # analyzer.sfb_bigram: repeats and off-layout grams excluded, denominator = all grams
    sfb = sum(n for g, n in grams.items()
              if all(c in fingers for c in g) and g[0] != g[1] and fingers[g[0]] == fingers[g[1]])
    return sfb / sum(grams.values())


def cmini_trigrams(fingers, grams, xyx=False):
    # analyzer.trigrams: space grams skipped, repeats -> sfR, off-layout -> 'unknown'.
    # xyx=True classifies x?x trigrams by finger like GECKO and adds GECKO's skip-1 SFS
    # (same finger, different keys, any middle symbol).
    counts, skip = defaultdict(float), 0
    for g, n in grams.items():
        if " " in g:
            continue
        if xyx and g[0] != g[2] and fingers.get(g[0], 0) == fingers.get(g[2], 1):
            skip += n
        if g[0] == g[1] or g[1] == g[2] or (g[0] == g[2] and not xyx):
            counts["sfR"] += n
            continue
        combo = "-".join(fingers[c] for c in g if c in fingers)
        cls = TABLE.get(combo, "unknown")
        if xyx and cls == "dsfb-red":  # GECKO: a redirect whatever the end fingers
            cls = "redirect" if "I" in combo else "bad-redirect"
        if xyx and cls == "dsfb-alt" and g[0] == g[2]:  # GECKO: same-key ends alternate
            cls = "alternate"
        counts[cls] += n
    counts["skip-sfs"] = skip
    total = sum(counts.values()) - skip
    return {k: v / total for k, v in counts.items()}


def aligned(grams, fingers):
    """Lowercase and keep only n-grams fully on the layout (GECKO's normalisation)."""
    out = defaultdict(int)
    for g, n in grams.items():
        g = g.lower()
        if all(c in fingers for c in g):
            out[g] += n
    return out


def gecko(rows, profile="default"):
    spec = "raw:" + "".join(rows)
    out = subprocess.run([GECKO, "score", spec, "-c", "shai", "-p", profile, "--json"], check=True,
                         capture_output=True, text=True).stdout
    return {m["id"]: m["value"] for m in json.loads(out)["breakdown"]["metrics"]}


ROOT = os.path.dirname(os.path.abspath(__file__)) + "/../.."
FULL_PROFILE = tempfile.NamedTemporaryFile("w", suffix=".toml", delete=False).name
with open(f"{ROOT}/data/profiles/default.toml") as src, open(FULL_PROFILE, "w") as dst:
    dst.write("trigram_coverage = 1.0\n" + src.read())

result = []
for name in LAYOUTS:
    meta = subprocess.run([GECKO, "score", name, "-c", "shai", "--json"], check=True,
                          capture_output=True, text=True).stdout
    rows = json.loads(meta)["rows"]
    fingers = fingers_of(rows)
    g = gecko(rows)
    # cmini lowercases inside its loops; mirror that for the raw column
    low_bi, low_tri = defaultdict(int), defaultdict(int)
    for k, n in BIGRAMS.items():
        low_bi[k.lower()] += n
    for k, n in TRIGRAMS.items():
        low_tri[k.lower()] += n
    raw_tri = cmini_trigrams(fingers, low_tri)
    raw_tri["sfb"] = cmini_sfb(fingers, low_bi)
    al_tri = cmini_trigrams(fingers, aligned(TRIGRAMS, fingers))
    al_tri["sfb"] = cmini_sfb(fingers, aligned(BIGRAMS, fingers))
    m_tri = cmini_trigrams(fingers, aligned(TRIGRAMS, fingers), xyx=True)
    m_tri["sfb"] = al_tri["sfb"]
    w_tri = cmini_trigrams(fingers, aligned(WORD_TRI, fingers), xyx=True)
    w_tri["sfb"] = cmini_sfb(fingers, aligned(WORD_BI, fingers))
    skip = aligned(WORD_SKIP, fingers)
    w_tri["skip-sfs"] = sum(n for k, n in skip.items()
                            if k[0] != k[1] and fingers[k[0]] == fingers[k[1]]) / sum(skip.values())
    g_full = gecko(rows, FULL_PROFILE)
    row = {"layout": name, "rows": rows}
    for stat, (classes, gid) in STATS.items():
        row[stat] = {
            "cmini": 100 * sum(raw_tri.get(c, 0) for c in classes),
            "aligned": 100 * sum(al_tri.get(c, 0) for c in classes),
            "matched": 100 * sum(m_tri.get(c, 0) for c in (["skip-sfs"] if stat == "sfs" else classes)),
            "words": 100 * sum(w_tri.get(c, 0) for c in (["skip-sfs"] if stat == "sfs" else classes)),
            "gecko": g[gid],
            "gecko_full": g_full[gid],
        }
    row["sfR"] = {"cmini": 100 * raw_tri.get("sfR", 0), "aligned": 100 * al_tri.get("sfR", 0)}
    row["unknown"] = {"cmini": 100 * raw_tri.get("unknown", 0)}
    result.append(row)

json.dump({"corpus": "shai", "cmini_commit": subprocess.run(
    ["git", "-C", CMINI, "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip(),
    "layouts": result}, sys.stdout, indent=1)
os.unlink(FULL_PROFILE)
