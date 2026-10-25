"""Inward vs outward rolls in the Aalto timing (fast QWERTY touch typists, timing30.npz from the
cost-function study) and in Engram's measured comfort choices.

Timing: each same-hand, different-finger key pair a->b against its reverse b->a (the same two
keys), on mean log interval relative to the typist's median, after removing the practice effect
(slope on log pair count, as ept.py). Also the third keystroke after a two-key roll (j->k after
i->j, k on the other hand), relative to the j->k pair mean. Engram: Bradley-Terry strength of
a->b minus b->a over the measured left-hand pairs. Output: roll_timing.json."""
import csv, json, sys
from pathlib import Path
import numpy as np

W = Path.home() / "projects/gecko-workspace/costfit"
sys.path.insert(0, str(W))
from ept import B, T, N, FING, ROW, HAND, practice_slope  # noqa: E402

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
rng = np.random.default_rng(1)

def inward(a, b):
    """a -> b moves toward the index finger (left hand: finger 0..3 rising; right: 9..6 falling)."""
    return FING[b] > FING[a] if HAND[a] == 0 else FING[b] < FING[a]

def kind(a, b):
    adj = abs(FING[a] - FING[b]) == 1
    return {"adjacent": adj, "skip": not adj, "same_row": ROW[a] == ROW[b], "cross_row": ROW[a] != ROW[b],
            "pinky": FING[a] in (0, 9) or FING[b] in (0, 9), "no_pinky": FING[a] not in (0, 9) and FING[b] not in (0, 9)}

def boot(d, w):
    d, w = np.array(d), np.array(w)
    m = np.average(d, weights=w)
    bs = [np.average(d[i], weights=w[i]) for i in (rng.integers(0, len(d), len(d)) for _ in range(2000))]
    return {"mean": float(m), "lo": float(np.percentile(bs, 2.5)), "hi": float(np.percentile(bs, 97.5)),
            "inward_faster": int(np.sum(d < 0)), "pairs": len(d)}

out = {}
# --- bigram: a->b vs b->a ---
slope = practice_slope(0)[0]
s = B[0].sum(0)
n = s[0]; m = np.where(n > 0, s[1] / np.maximum(n, 1), 0)
adj = m - slope * np.log(np.maximum(n, 1))
rows = []
for a in range(N):
    for b in range(N):
        if a == b or HAND[a] != HAND[b] or FING[a] == FING[b] or not inward(a, b):
            continue
        ka, kb = a * N + b, b * N + a
        if n[ka] < 200 or n[kb] < 200:
            continue
        rows.append((a, b, adj[ka] - adj[kb], min(n[ka], n[kb]), m[ka] - m[kb]))
out["practice_slope"] = float(slope)
out["bigram"] = {}
for cat in ["all", "adjacent", "skip", "same_row", "cross_row", "pinky", "no_pinky"]:
    r = [x for x in rows if cat == "all" or kind(x[0], x[1])[cat]]
    if len(r) >= 3:
        out["bigram"][cat] = boot([x[2] for x in r], [np.sqrt(x[3]) for x in r])
        out["bigram"][cat]["raw_mean"] = float(np.mean([x[4] for x in r]))

# --- trigram: third keystroke after a roll i->j (same hand), k on the other hand ---
st = T[0].sum(0)
tn = st[0]; tm = np.where(tn > 0, st[1] / np.maximum(tn, 1), 0)
tri = []
for i in range(N):
    for j in range(N):
        if i == j or HAND[i] != HAND[j] or FING[i] == FING[j]:
            continue
        for k in range(N):
            if HAND[k] == HAND[j]:
                continue
            t = (i * N + j) * N + k
            if tn[t] < 30 or n[j * N + k] < 200:
                continue
            tri.append((i, j, inward(i, j), tm[t] - m[j * N + k], tn[t]))
out["third_key"] = {}
for name, f in [("inroll", lambda x: x[2]), ("outroll", lambda x: not x[2])]:
    r = [x for x in tri if f(x)]
    out["third_key"][name] = boot([x[3] for x in r], [np.sqrt(x[4]) for x in r])

# --- Engram measured comfort: BT strength a->b minus b->a (left hand; inward = column rising) ---
rows_c = [r for r in csv.reader(l for l in open(ROOT / "data/comfort/engram-bt.csv") if not l.startswith("#"))]
h = rows_c[0]
d = {(int(r[0]), int(r[1]), int(r[2]), int(r[3])): dict(zip(h, r)) for r in rows_c[1:]}
eng = []
for (ar, ac, br, bc), r in d.items():
    if ac > 3 or bc > 3 or ac >= bc or r["measured"] != "1":
        continue
    rv = d[(br, bc, ar, ac)]
    if rv["measured"] != "1":
        continue
    a, b = ar * 10 + ac, br * 10 + bc
    eng.append((a, b, float(r["bt_strength"]) - float(rv["bt_strength"]), int(r["n"]) + int(rv["n"])))
out["engram"] = {}
for cat in ["all", "adjacent", "skip", "same_row", "cross_row", "pinky", "no_pinky"]:
    r = [x for x in eng if cat == "all" or kind(x[0], x[1])[cat]]
    if len(r) >= 3:
        b_ = boot([-x[2] for x in r], [np.sqrt(x[3]) for x in r])  # sign: negative = inward preferred
        out["engram"][cat] = {**b_, "p_inward": float(1 / (1 + np.exp(b_["mean"])))}

json.dump(out, open(HERE / "roll_timing.json", "w"), indent=1)
print(f"practice slope {slope:.4f}")
print("bigram a->b inward minus b->a outward (log interval, practice-adjusted; < 0 = inward faster)")
for k, v in out["bigram"].items():
    print(f"  {k:10} {v['mean']:+.4f} [{v['lo']:+.4f}, {v['hi']:+.4f}]  inward faster {v['inward_faster']}/{v['pairs']}  (raw {v['raw_mean']:+.4f})")
print("third keystroke after a roll, vs its pair mean (< 0 = faster)")
for k, v in out["third_key"].items():
    print(f"  {k:10} {v['mean']:+.4f} [{v['lo']:+.4f}, {v['hi']:+.4f}]  triples {v['pairs']}")
print("Engram comfort: P(inward preferred over its reverse)")
for k, v in out["engram"].items():
    print(f"  {k:10} P {v['p_inward']:.3f}  strength diff {-v['mean']:+.3f} [{-v['hi']:+.3f}, {-v['lo']:+.3f}]  pairs {v['pairs']}")
