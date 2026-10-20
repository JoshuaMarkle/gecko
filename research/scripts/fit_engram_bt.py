"""Fit a Bradley-Terry comfort table from Engram's raw bigram choices and map it onto ansi-30.

usage: python3 -I research/scripts/fit_engram_bt.py processed_bigram_data.csv data/comfort/engram-2key.csv data/comfort/engram-bt.csv
Input: typing_preferences_to_comfort_scores@d8978d4 data/processed_bigram_data.csv (MIT). Needs numpy + scipy.
Prints the fit statistics quoted in research/sources/engram-preference-study.md. Deterministic (fold seed 1).
"""
import csv
import sys
from collections import Counter, defaultdict

import numpy as np
from scipy.optimize import minimize
from scipy.stats import spearmanr

choices_path, avg4_path, out_path = sys.argv[1:4]

# QWERTY left-hand letters -> (row, col) on the 3x10 block.
POS = {}
for r, row in enumerate(["qwert", "asdfg", "zxcvb"]):
    for c, ch in enumerate(row):
        POS[ch] = (r, c)

rows = list(csv.DictReader(open(choices_path)))
bigrams = sorted({b for r in rows for b in (r["chosen_bigram"], r["unchosen_bigram"])})
assert all(len(b) == 2 and b[0] in POS and b[1] in POS for b in bigrams), "unexpected bigram"
idx = {b: i for i, b in enumerate(bigrams)}
W = np.array([idx[r["chosen_bigram"]] for r in rows])
L = np.array([idx[r["unchosen_bigram"]] for r in rows])
users = np.array([r["user_id"] for r in rows])
K = len(bigrams)

# avg4 per ordered (row,col) pair, from the formula table.
avg4 = {}
for line in open(avg4_path):
    if line.startswith("#") or line.startswith("a_row"):
        continue
    f = line.strip().split(",")
    avg4[(int(f[0]), int(f[1]), int(f[2]), int(f[3]))] = float(f[4])
A4 = np.array([avg4[POS[b[0]] + POS[b[1]]] for b in bigrams])


def fit(w, l, lam):
    def f(s):
        d = s[w] - s[l]
        loss = np.logaddexp(0, -d).sum() + 0.5 * lam * (s @ s)
        p = 1 / (1 + np.exp(d))  # d loss / d d = -p
        g = lam * s
        np.add.at(g, w, -p)
        np.add.at(g, l, p)
        return loss, g

    return minimize(f, np.zeros(K), jac=True, method="L-BFGS-B").x


def evaluate(s, w, l):
    d = s[w] - s[l]
    acc = np.mean(np.where(d > 0, 1.0, np.where(d == 0, 0.5, 0.0)))
    ll = np.mean(np.logaddexp(0, -d))
    return acc, ll


def formula_acc(w, l):
    d = A4[w] - A4[l]
    return np.mean(np.where(d > 0, 1.0, np.where(d == 0, 0.5, 0.0))), np.mean(d == 0)


# Participant-aware 5-fold CV.
uniq = np.array(sorted(set(users)))
rng = np.random.default_rng(1)
rng.shuffle(uniq)
folds = np.array_split(uniq, 5)
print(f"choices {len(rows)}, participants {len(uniq)}, distinct bigrams {K}, distinct pairs {len({tuple(sorted((a, b))) for a, b in zip(W, L)})}")
results = {}
for lam in [0.03, 0.1, 0.3, 1.0, 3.0, 10.0]:
    accs, lls, faccs, ties = [], [], [], []
    for fu in folds:
        te = np.isin(users, fu)
        s = fit(W[~te], L[~te], lam)
        a, ll = evaluate(s, W[te], L[te])
        fa, ft = formula_acc(W[te], L[te])
        accs.append(a), lls.append(ll), faccs.append(fa), ties.append(ft)
    results[lam] = (np.mean(accs), np.mean(lls))
    print(f"lambda {lam:5}: CV acc {np.mean(accs):.3f}  CV logloss {np.mean(lls):.4f}  | avg4 formula acc {np.mean(faccs):.3f} (ties {np.mean(ties):.2f})")
lam = min(results, key=lambda k: results[k][1])
print("chosen lambda", lam)

s = fit(W, L, lam)
acc_in, ll_in = evaluate(s, W, L)
print(f"in-sample acc {acc_in:.3f}, logloss {ll_in:.4f}; majority-of-pair ceiling below")
# Agreement ceiling: per unordered pair, share of choices that agree with the majority.
pc = defaultdict(Counter)
for w, l in zip(W, L):
    pc[tuple(sorted((w, l)))][w] += 1
ceil = sum(max(c.values()) for c in pc.values()) / len(W)
print(f"majority-per-pair ceiling {ceil:.3f}")

# Rescale: match mean and std of avg4 over the measured bigrams, clip to [0, 1].
score = np.clip(A4.mean() + (s - s.mean()) / s.std() * A4.std(), 0.0, 1.0)
n_ch = Counter(list(W) + list(L))
rho = spearmanr(score, A4).statistic
offd = np.array([b[0] != b[1] for b in bigrams])
rho_off = spearmanr(score[offd], A4[offd]).statistic
print(f"spearman(BT score, avg4) over {K} measured bigrams: {rho:.3f} (off-diagonal {offd.sum()}: {rho_off:.3f})")

by_rc = {POS[b[0]] + POS[b[1]]: (score[i], s[i], n_ch[i]) for i, b in enumerate(bigrams)}

for b in ["df", "fd", "sd", "ds", "sf", "as", "er", "re", "fr", "rf", "ex", "xe", "wz", "zw", "qz", "zq", "aq", "xq", "cv", "dd", "ff", "qq", "zz"]:
    k = POS[b[0]] + POS[b[1]]
    if k in by_rc:
        print(f"  {b}: bt {by_rc[k][0]:.3f}  avg4 {avg4[k]:.3f}  n {by_rc[k][2]}")
    else:
        print(f"  {b}: unmeasured (avg4 {avg4[k]:.3f})")

order = np.argsort(-score)
print("top 8:", [bigrams[i] for i in order[:8]], " bottom 8:", [bigrams[i] for i in order[-8:]])
rk_bt = np.argsort(np.argsort(-score))
rk_a4 = np.argsort(np.argsort(-A4))
dis = np.argsort(-(np.abs(rk_bt - rk_a4)))[:8]
print("largest rank disagreements (bigram, bt, avg4):", [(bigrams[i], round(score[i], 2), round(A4[i], 2)) for i in dis])

# Write the ansi-30 table. Right hand mirrors left: col c -> 9 - c.
def hand(c):
    return 0 if c < 5 else 1


def to_left(r, c):
    return (r, c) if c < 5 else (r, 9 - c)


out = []
measured_cells = 0
for ar in range(3):
    for ac in range(10):
        for br in range(3):
            for bc in range(10):
                key = (ar, ac, br, bc)
                sc, st, n, meas = avg4[key], "", 0, 0
                if hand(ac) == hand(bc):
                    lk = to_left(ar, ac) + to_left(br, bc)
                    if lk in by_rc:
                        sc, st, n = by_rc[lk][0], f"{by_rc[lk][1]:.4f}", by_rc[lk][2]
                        meas = 1
                        measured_cells += (ar, ac) != (br, bc)
                out.append(f"{ar},{ac},{br},{bc},{sc:.4f},{meas},{st},{n},{avg4[key]:.4f}")

same_hand_offdiag = 2 * (15 * 15 - 15)
print(f"same-hand off-diagonal ansi-30 pairs measured: {measured_cells}/{same_hand_offdiag}")
all_scores = [float(l.split(",")[4]) for l in out]
all_a4 = [float(l.split(",")[8]) for l in out]
print(f"spearman over all 900 rows vs engram-2key: {spearmanr(all_scores, all_a4).statistic:.3f}")

header = f"""# Engram key-pair comfort FITTED from the raw pairwise choices (Bradley-Terry), mapped onto ansi-30.
# source: https://github.com/binarybottle/typing_preferences_to_comfort_scores/blob/d8978d4ba8f81cd39c96ac2f8f2b730ed608a148/data/processed_bigram_data.csv
# licence: MIT (c) Arno Klein
# data: {len(rows)} choices, {len(uniq)} participants, {K} distinct bigrams over 12 left-hand keys (QWERTY qwer/asdf/zxcv).
# model: P(a preferred over b) = logistic(s_a - s_b), L2 lambda = {lam} (chosen by participant-grouped 5-fold CV log-loss).
# scale: score = clip(mean(avg4) + z(s) * std(avg4), 0, 1) over measured bigrams, so it shares engram-2key's 0..1 scale;
#        higher = more comfortable. Unmeasured pairs (center columns, unseen bigrams, cross-hand) fall back to avg4 (measured = 0).
# mapping: left hand measured; right hand mirrored (col c -> 9 - c), order kept (a typed first). Cross-hand = avg4 (1.0 unless a center column).
# columns: score (use this), measured (1 = from choices), bt_strength (raw s, blank if unmeasured), n (choices involving the bigram), avg4 (formula value).
a_row,a_col,b_row,b_col,score,measured,bt_strength,n,avg4
"""
open(out_path, "w").write(header + "\n".join(out) + "\n")
print("wrote", out_path)
