"""Study 1: pair-level calibration. Practice-adjusted key-pair time (log rel IKI, QWERTY touch
typists) regressed on GECKO's per-metric pair tables (ansi-30). Sign-constrained least squares
(costs >= 0, rewards <= 0), cross-validated by typist half, by pair group and by extrapolation
to rarely typed pairs (the regime optimized layouts live in)."""
import json, sys, itertools, numpy as np
from scipy.optimize import nnls
from scipy.stats import spearmanr
sys.path.insert(0, '/home/josh/projects/gecko-workspace/costfit')
from ept import pair_means, practice_slope, N, KEYS

TB = json.load(open('/home/josh/projects/gecko-workspace/costfit/data/tables_ansi30.json'))['tables']
GROUP = 0

def cols(ids):
    """Signed pair table per metric (Max metrics negated so every coefficient is >= 0)."""
    out = []
    for m in ids:
        t = TB[m]; v = np.array(t['v'], float)
        if t['order'] == 'mono':
            v = np.tile(v, N)  # cost of the second key j: index i*N+j -> v[j]
        sgn = -1.0 if t['dir'] == 'max' else 1.0
        out.append(sgn * v / 100.0 if t['order'] != 'mono' else sgn * v)
    return np.array(out).T

def target(half=None, slope=None):
    n, m, var = pair_means(GROUP, half)
    b = practice_slope(GROUP)[0] if slope is None else slope
    return n, m - b * np.log(np.maximum(n, 1)), var

LETTER = np.array([KEYS[k // N].isalpha() and KEYS[k % N].isalpha() for k in range(N * N)])
NUIS = 'quad'  # practice nuisance: 'none', 'lin' (log n), 'quad' (log n, log² n); set by main
def nuis(n):
    l = np.log(np.maximum(n, 1)); l = l - 7.0
    if NUIS == 'letters':
        nn = pair_means(GROUP)[0].reshape(N, N); lo = np.log(np.maximum(nn.sum(1), 1)); li = np.log(np.maximum(nn.sum(0), 1))
        I = np.repeat(np.arange(N), N); J = np.tile(np.arange(N), N)
        return np.c_[l, lo[I] - 10, li[J] - 10]
    return {'none': np.zeros((len(n), 0)), 'lin': l[:, None], 'quad': np.c_[l, l * l]}[NUIS]

def fit(X, y, w, Z=None):
    """Sign-constrained weighted LS (metric coefficients >= 0) with a free intercept and free
    nuisance columns Z (practice). Returns (beta, intercept, nuisance coefs)."""
    Z = np.zeros((len(y), 0)) if Z is None else Z
    A = np.c_[X, Z, -Z]
    sw = np.sqrt(w); mx = np.average(A, 0, w); my = np.average(y, weights=w)
    b, _ = nnls((A - mx) * sw[:, None], (y - my) * sw)
    k = X.shape[1]; z = b[k:k + Z.shape[1]] - b[k + Z.shape[1]:]
    return b[:k], my - mx @ b, z

def r2(y, p, w):
    my = np.average(y, weights=w)
    return 1 - np.sum(w * (y - p) ** 2) / np.sum(w * (y - my) ** 2)

CANDS = ['B01', 'B04', 'B05', 'B06', 'B07', 'B08', 'B09', 'B10', 'B11', 'B14', 'E04', 'E05', 'M01', 'M05', 'M06', 'G01', 'G05']
PROFILE_SET = ['B01', 'B05', 'B06', 'B09', 'B10', 'B11', 'B14', 'E05', 'M01', 'M05']

def run(ids, verbose=True, letters=True):
    X = cols(ids)
    n, y, var = target(None, 0.0)
    nA, yA, _ = target(0, 0.0); nB, yB, _ = target(1, 0.0)
    off = np.array([i != j for i in range(N) for j in range(N)])
    base = off & (LETTER if letters else True)
    ok = base & (n >= 30)
    w = 1 / (np.mean(var[ok]) / np.maximum(n, 1) + 0.003)
    Z = nuis(n)
    pr = lambda b, a, z, m: X[m] @ b + a + Z[m] @ z
    res = {}
    beta, a, z = fit(X[ok], y[ok], w[ok], Z[ok]); res['beta'] = dict(zip(ids, beta.round(5).tolist())); res['nuis'] = z.round(4).tolist()
    res['r2_in'] = r2(y[ok], pr(beta, a, z, ok), w[ok])
    # variance explained beyond practice alone
    _, a0, z0 = fit(X[ok][:, :0], y[ok], w[ok], Z[ok]); y0 = y - (a0 + Z @ z0)
    res['r2_metrics_partial'] = r2(y0[ok], pr(beta, a, z, ok) - (a0 + Z[ok] @ z0), w[ok])
    okAB = base & (nA >= 30) & (nB >= 30)
    bA, aA, zA = fit(X[okAB], yA[okAB], w[okAB], Z[okAB]); bB, aB, zB = fit(X[okAB], yB[okAB], w[okAB], Z[okAB])
    res['r2_typist'] = (r2(yB[okAB], pr(bA, aA, zA, okAB), w[okAB]) + r2(yA[okAB], pr(bB, aB, zB, okAB), w[okAB])) / 2
    res['r2_ceiling'] = (r2(yB[okAB], yA[okAB], w[okAB]) + r2(yA[okAB], yB[okAB], w[okAB])) / 2
    rng = np.random.default_rng(0)
    grp = np.array([min(i, j) * N + max(i, j) for i in range(N) for j in range(N)])
    ug = np.unique(grp[ok]); fold = dict(zip(ug, rng.integers(0, 10, len(ug))))
    f = np.array([fold.get(g, -1) for g in grp]); pred = np.zeros(N * N)
    for k in range(10):
        tr = ok & (f != k); te = ok & (f == k)
        b, a1, z1 = fit(X[tr], y[tr], w[tr], Z[tr]); pred[te] = pr(b, a1, z1, te)
    res['r2_pairCV'] = r2(y[ok], pred[ok], w[ok])
    res['r2_pairCV_partial'] = r2(y0[ok], pred[ok] - (a0 + Z[ok] @ z0), w[ok])
    tr = ok & (n >= 1000); te = ok & (n < 1000)
    b, a1, z1 = fit(X[tr], y[tr], w[tr], Z[tr])
    res['r2_extrap_partial'] = r2(y0[te], pr(b, a1, z1, te) - (a0 + Z[te] @ z0), w[te]); res['n_extrap_test'] = int(te.sum())
    res['rho_extrap'] = float(spearmanr(y0[te], pr(b, a1, z1, te))[0])
    B = []
    for _ in range(200):
        pick = rng.choice(ug, len(ug)); cnt = dict(zip(*np.unique(pick, return_counts=True)))
        ww = w * np.array([cnt.get(g, 0) for g in grp])
        b, _, _ = fit(X[ok], y[ok], ww[ok], Z[ok]); B.append(b)
    B = np.array(B)
    res['ci'] = {m: [round(float(np.percentile(B[:, k], 5)), 5), round(float(np.percentile(B[:, k], 95)), 5), round(float(np.mean(B[:, k] > 0)), 3)] for k, m in enumerate(ids)}
    res['n_pairs'] = int(ok.sum())
    if verbose:
        print(f"ids {ids}  [practice {NUIS}: {res['nuis']}]\n  pairs {res['n_pairs']}  R2 in {res['r2_in']:.3f} (beyond practice {res['r2_metrics_partial']:.3f})  typist-split {res['r2_typist']:.3f} (ceiling {res['r2_ceiling']:.3f})  pair-CV {res['r2_pairCV']:.3f} (beyond practice {res['r2_pairCV_partial']:.3f})  extrap beyond practice {res['r2_extrap_partial']:.3f} (rho {res['rho_extrap']:.3f}, n {res['n_extrap_test']})")
        for m in ids:
            print(f"  {m}: beta {res['beta'][m]:.5f}  90% CI {res['ci'][m][:2]}  P(>0) {res['ci'][m][2]}")
    return res

if __name__ == '__main__':
    NUIS = sys.argv[1] if len(sys.argv) > 1 else 'quad'
    out = {'nuis': NUIS}
    out['all'] = run(CANDS)
    out['profile'] = run(PROFILE_SET)
    out['B01_only'] = run(['B01'])
    out['E01_judge'] = run(['E01'])
    # forward selection on pair-CV R2
    chosen = []; best = -1; path = []
    while True:
        cand = [(run(chosen + [m], verbose=False)['r2_pairCV'], m) for m in CANDS if m not in chosen]
        if not cand: break
        sc, m = max(cand)
        if sc < best + 0.002: break
        chosen.append(m); best = sc; path.append((m, round(sc, 4)))
    print('forward selection (pair-CV R2):', path)
    out['forward'] = path
    out['selected'] = run(chosen)
    json.dump(out, open(f"/home/josh/projects/gecko-workspace/costfit/data/s1_pairs_{NUIS}.json", 'w'), indent=1)
