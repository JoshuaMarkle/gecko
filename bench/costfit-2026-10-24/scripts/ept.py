"""Empirical key-pair time tables (EPT) from timing30.npz, and layout judges built on them."""
import json, numpy as np
KEYS = "qwertyuiopasdfghjkl;zxcvbnm,./"
N = 30
D = np.load('/home/josh/projects/gecko-workspace/costfit/timing30.npz')
B, T = D['B'], D['T']  # [group, half, stat, ...]

# Slot structure of ansi-30 (QWERTY positions), for the frequency-slope control only.
FING = [0, 1, 2, 3, 3, 6, 6, 7, 8, 9] * 3
ROW = [i // 10 for i in range(N)]
HAND = [0 if FING[i] < 5 else 1 for i in range(N)]
EFF = [3.0, 2.4, 2.0, 2.2, 3.2, 3.2, 2.2, 2.0, 2.4, 3.0, 1.6, 1.3, 1.1, 1.0, 2.9, 2.9, 1.0, 1.1, 1.3, 1.6,
       3.2, 2.6, 2.3, 1.6, 3.0, 3.0, 1.6, 2.3, 2.6, 3.2]

def struct_feats(i, j):
    sf = FING[i] == FING[j] and i != j
    sh = HAND[i] == HAND[j] and not sf and i != j
    adj = sh and abs(FING[i] - FING[j]) == 1
    rows = abs(ROW[i] - ROW[j]) if sh else 0
    return [1.0, float(i == j), float(sf), float(sh), float(adj), rows, float(adj and rows >= 2),
            float(sh and {FING[i] % 9, FING[j] % 9} == {0, 1} or sh and {FING[i], FING[j]} == {8, 9}), EFF[i], EFF[j]]

def pair_means(group=0, half=None):
    s = B[group].sum(0) if half is None else B[group, half]
    n, m = s[0], np.where(s[0] > 0, s[1] / np.maximum(s[0], 1), 0.0)
    var = np.where(s[0] > 1, s[2] / np.maximum(s[0], 1) - m * m, 0.0)
    return n, m, var

def practice_slope(group=0, min_n=200):
    """Slope of mean log rel IKI on log(pair count), controlling key-pair structure."""
    n, m, _ = pair_means(group)
    rows, y, w = [], [], []
    for k in range(N * N):
        if n[k] >= min_n:
            i, j = divmod(k, N)
            rows.append(struct_feats(i, j) + [np.log(n[k])]); y.append(m[k]); w.append(np.sqrt(n[k]))
    X = np.array(rows); y = np.array(y); w = np.array(w) ** 0  # unweighted: one point per pair
    beta = np.linalg.lstsq(X * w[:, None], y * w, rcond=None)[0]
    return beta[-1], beta

def ept(group=0, half=None, k_shrink=None, slope=None, prior='struct'):
    """Motor cost per slot pair: mean log rel IKI minus the practice effect (slope × log count),
    shrunk toward a structural regression (prior='struct') or the mean (prior='mean')."""
    n, m, var = pair_means(group, half)
    b = practice_slope(group)[0] if slope is None else slope
    adj = m - b * np.log(np.maximum(n, 1))
    ok = n >= 30
    X = np.array([struct_feats(*divmod(k, N)) for k in range(N * N)])
    if prior == 'struct':
        beta = np.linalg.lstsq(X[ok], adj[ok], rcond=None)[0]; pr = X @ beta
    else:
        pr = np.full(N * N, np.average(adj[ok], weights=n[ok]))
    sig2_obs = np.average(var[ok], weights=n[ok])
    sig2_pair = max(np.var(adj[ok] - pr[ok]) - np.mean(sig2_obs / n[ok]), 1e-4)
    k = sig2_obs / sig2_pair if k_shrink is None else k_shrink
    out = (n * adj + k * pr) / (n + k)
    mu = np.average(out[ok], weights=n[ok])
    return (out - mu).reshape(N, N), n.reshape(N, N)

def tri_context(group=0, half=None, k_shrink=200.0):
    """Third-interval context effect: mean log rel of j->k after i minus the pair mean of j->k."""
    s = T[group].sum(0) if half is None else T[group, half]
    n = s[0]; m = np.where(n > 0, s[1] / np.maximum(n, 1), 0.0)
    pn, pm, _ = pair_means(group, half)
    jk = np.arange(N ** 3) % (N * N)
    c = (n * (m - pm[jk])) / (n + k_shrink)
    return c.reshape(N, N, N), n.reshape(N, N, N)

class Corpus:
    def __init__(self, path):
        d = json.load(open(path))
        self.cs = d['charset']; n = len(self.cs)
        self.F2 = np.zeros(n * n); [self.F2.__setitem__(i, v) for i, v in d['bi']]
        self.F2 = self.F2.reshape(n, n)
        self.t_idx = np.array([i for i, _ in d['tri']]); self.t_cnt = np.array([v for _, v in d['tri']], float)
        self.t = np.stack([self.t_idx // (n * n), (self.t_idx // n) % n, self.t_idx % n], 1)
        self.ix = {c: i for i, c in enumerate(self.cs)}

    def slots(self, chars):
        """Slot of every corpus symbol (-1 off the layout)."""
        s = -np.ones(len(self.cs), int)
        for k, c in enumerate(chars):
            if c in self.ix: s[self.ix[c]] = k
        return s

    def judge2(self, chars, table):
        s = self.slots(chars); on = np.where(s >= 0)[0]
        F = self.F2[np.ix_(on, on)]; Tt = table[np.ix_(s[on], s[on])]
        return float((F * Tt).sum() / F.sum())

    def judge3(self, chars, ctx):
        s = self.slots(chars); a, b, c = s[self.t[:, 0]], s[self.t[:, 1]], s[self.t[:, 2]]
        ok = (a >= 0) & (b >= 0) & (c >= 0)
        return float((self.t_cnt[ok] * ctx[a[ok], b[ok], c[ok]]).sum() / self.t_cnt[ok].sum())
