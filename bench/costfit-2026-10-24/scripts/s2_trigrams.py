"""Study 2: trigram-level calibration. Context effect on the third keystroke: mean log rel IKI of
j->k when typed after i, minus the pair mean of j->k (same typists). Regressed on GECKO's
trigram class tables and skipgram tables at (i, k), with practice nuisance (log trigram count
minus log pair count). Letters only; triples with n >= 30."""
import json, sys, numpy as np
from scipy.optimize import nnls
sys.path.insert(0, '/home/josh/projects/gecko-workspace/costfit')
from ept import N, KEYS, B as PB, T as PT

TB = json.load(open('/home/josh/projects/gecko-workspace/costfit/data/tables_ansi30.json'))['tables']
I3 = np.arange(N ** 3) // (N * N); J3 = (np.arange(N ** 3) // N) % N; K3 = np.arange(N ** 3) % N
LET = np.array([c.isalpha() for c in KEYS])
LETTER3 = LET[I3] & LET[J3] & LET[K3]

def stats(half=None):
    s = PT[0].sum(0) if half is None else PT[0, half]
    p = PB[0].sum(0) if half is None else PB[0, half]
    n = s[0]; m = np.where(n > 0, s[1] / np.maximum(n, 1), 0); var = np.where(n > 1, s[2] / np.maximum(n, 1) - m * m, 0)
    pn = p[0]; pm = np.where(pn > 0, p[1] / np.maximum(pn, 1), 0)
    jk = J3 * N + K3
    return n, m - pm[jk], var, pn[jk]

def col(m):
    t = TB[m]; v = np.array(t['v'], float) / 100.0
    if t['order'] in ('skip', 'skipfar', 'bi'):
        v = v[I3 * N + K3] if t['order'] != 'bi' else v[I3 * N + J3]
    return (-1.0 if t['dir'] == 'max' else 1.0) * v

def fit(X, y, w, Z):
    A = np.c_[X, Z, -Z]; sw = np.sqrt(w); mx = np.average(A, 0, w); my = np.average(y, weights=w)
    b, _ = nnls((A - mx) * sw[:, None], (y - my) * sw); k = X.shape[1]
    return b[:k], my - mx @ b, b[k:k + Z.shape[1]] - b[k + Z.shape[1]:]

def r2(y, p, w):
    return 1 - np.sum(w * (y - p) ** 2) / np.sum(w * (y - np.average(y, weights=w)) ** 2)

def run(ids, verbose=True):
    X = np.array([col(m) for m in ids]).T
    n, y, var, pn = stats(); nA, yA, _, pnA = stats(0); nB, yB, _, pnB = stats(1)
    ok = LETTER3 & (n >= 30) & (I3 != J3) & (J3 != K3)
    Z = np.c_[np.log(np.maximum(n, 1)) - np.log(np.maximum(pn, 1))]
    w = 1 / (np.mean(var[ok]) / np.maximum(n, 1) + 0.002)
    b, a, z = fit(X[ok], y[ok], w[ok], Z[ok]); out = {'beta': dict(zip(ids, b.round(5).tolist())), 'nuis': z.tolist()}
    out['r2_in'] = r2(y[ok], X[ok] @ b + a + Z[ok] @ z, w[ok])
    okAB = ok & (nA >= 15) & (nB >= 15)
    bA, aA, zA = fit(X[okAB], yA[okAB], w[okAB], Z[okAB])
    out['r2_typist'] = r2(yB[okAB], X[okAB] @ bA + aA + Z[okAB] @ zA, w[okAB])
    out['r2_ceiling'] = r2(yB[okAB], yA[okAB], w[okAB])
    rng = np.random.default_rng(1); g = np.minimum(I3, K3) * N * N + J3 * N + np.maximum(I3, K3)
    ug = np.unique(g[ok]); Bs = []
    for _ in range(200):
        pick = rng.choice(ug, len(ug)); cnt = dict(zip(*np.unique(pick, return_counts=True)))
        ww = w * np.array([cnt.get(x, 0) for x in g]) if False else w[ok] * np.array([cnt.get(x, 0) for x in g[ok]])
        bb, _, _ = fit(X[ok], y[ok], ww, Z[ok]); Bs.append(bb)
    Bs = np.array(Bs)
    out['ci'] = {m: [round(float(np.percentile(Bs[:, k], 5)), 5), round(float(np.percentile(Bs[:, k], 95)), 5), round(float(np.mean(Bs[:, k] > 0)), 3)] for k, m in enumerate(ids)}
    out['n'] = int(ok.sum())
    if verbose:
        print(f"ids {ids}\n  triples {out['n']}  R2 in {out['r2_in']:.3f}  typist-split {out['r2_typist']:.3f} (ceiling {out['r2_ceiling']:.3f})  practice {np.round(z, 4)}")
        for m in ids:
            print(f"  {m}: beta {out['beta'][m]:+.5f}  90% CI {out['ci'][m][:2]}  P(>0) {out['ci'][m][2]}")
    return out

if __name__ == '__main__':
    res = {}
    res['classes'] = run(['T01', 'T03', 'T04', 'T06', 'T07', 'T08', 'S01', 'S04', 'T09', 'T10'])
    res['profile'] = run(['T01', 'T03', 'T04', 'T06', 'T07', 'S01', 'S04'])
    res['with_prev'] = run(['T01', 'T03', 'T04', 'T06', 'T07', 'S01', 'S04', 'B01', 'B14', 'B10'])
    json.dump(res, open('/home/josh/projects/gecko-workspace/costfit/data/s2_trigrams.json', 'w'), indent=1)
