"""Held-out time judge, independent of GECKO's metric definitions.
Pair part: practice-adjusted mean log rel IKI per key pair from one typist half, shrunk toward a
nonparametric structural model fitted on the same half (finger pair x row pair cells + key
effects + log-count practice). Pairs with punctuation keys use the structural model only.
Trigram part: third-keystroke context effect per key triple, shrunk toward a hand-pattern model.
J(layout) = mean pair cost over corpus bigrams + mean context cost over corpus trigrams
(log time per keystroke; lower = faster)."""
import json, numpy as np, sys
sys.path.insert(0, '/home/josh/projects/gecko-workspace/costfit')
from ept import N, KEYS, FING, ROW, B as PB, T as PT, Corpus

F = np.array(FING); R = np.array(ROW); H = (F >= 5).astype(int)
I = np.repeat(np.arange(N), N); J = np.tile(np.arange(N), N)
LET = np.array([c.isalpha() for c in KEYS])

def _design():
    cells = F[I] * 10 + F[J]
    rp = R[I] * 3 + R[J]
    sh = (H[I] == H[J]) & (I != J)
    X = [np.eye(100)[cells], np.eye(9)[rp] * sh[:, None], np.eye(N)[J], np.eye(N)[I], (I == J)[:, None].astype(float)]
    return np.concatenate(X, 1)
XD = _design()

MIRROR = {KEYS.index(';'): KEYS.index('a'), KEYS.index(','): KEYS.index('c'), KEYS.index('.'): KEYS.index('x'),
          KEYS.index('/'): KEYS.index('z')}

def letter_counts():
    n = PB[0].sum(0)[0].reshape(N, N)
    return np.log(np.maximum(n.sum(1), 1)), np.log(np.maximum(n.sum(0), 1))

def pair_table(half, ridge=1.0, letters=True):
    """letters: also control log letter frequency of both keys (key = letter on QWERTY)."""
    s = PB[0, half] if half is not None else PB[0].sum(0)
    n = s[0]; m = np.where(n > 0, s[1] / np.maximum(n, 1), 0); var = np.where(n > 1, s[2] / np.maximum(n, 1) - m * m, 0)
    ok = LET[I] & LET[J] & (n >= 30)
    lo, li = letter_counts()
    L = np.c_[np.log(np.maximum(n, 1)) - 7.0] if not letters else np.c_[np.log(np.maximum(n, 1)) - 7.0, lo[I] - 10, li[J] - 10]
    w = 1 / (np.mean(var[ok]) / np.maximum(n, 1) + 0.003)
    A = np.c_[XD, L]; sw = np.sqrt(w[ok]); q = L.shape[1]
    reg = ridge * np.eye(A.shape[1]); reg[-q:, -q:] = 0
    beta = np.linalg.solve((A[ok] * sw[:, None]).T @ (A[ok] * sw[:, None]) + reg, (A[ok] * sw[:, None]).T @ (m[ok] * sw))
    bk = beta[:-q].copy()
    for p, src in MIRROR.items():  # punctuation keys have no letter data: use the mirror key's effects
        mi = {k: v for k, v in MIRROR.items()}
        bk[109 + N + p] = bk[109 + N + src]; bk[109 + p] = bk[109 + src]
    struct = XD @ bk; motor = m - L @ beta[-q:]
    resid_var = max(np.average((motor[ok] - struct[ok]) ** 2, weights=w[ok]) - np.mean(np.mean(var[ok]) / n[ok]), 1e-4)
    k = np.mean(var[ok]) / resid_var
    use = LET[I] & LET[J] & (n > 0)
    out = np.where(use, (n * motor + k * struct) / (n + k), struct)
    return out.reshape(N, N), {'practice': beta[-q:].round(4).tolist(), 'k': float(k), 'n_obs': int(ok.sum())}

def tri_table(half, k=200.0):
    s = PT[0, half] if half is not None else PT[0].sum(0)
    p = PB[0, half] if half is not None else PB[0].sum(0)
    n = s[0]; m = np.where(n > 0, s[1] / np.maximum(n, 1), 0)
    I3 = np.arange(N ** 3) // (N * N); J3 = (np.arange(N ** 3) // N) % N; K3 = np.arange(N ** 3) % N
    pn = p[0]; pm = np.where(pn > 0, p[1] / np.maximum(pn, 1), 0); jk = J3 * N + K3
    c = m - pm[jk]
    hi, hj, hk = H[I3], H[J3], H[K3]
    pat = np.c_[np.ones(N ** 3), (hi == hj) & (hj != hk), (hi != hj) & (hj == hk), (hi != hj) & (hj != hk), I3 == K3,
                (F[I3] == F[K3]) & (I3 != K3), (F[I3] == F[J3]) & (I3 != J3), (F[J3] == F[K3]) & (J3 != K3)].astype(float)
    Lp = np.log(np.maximum(n, 1)) - np.log(np.maximum(pn[jk], 1))
    ok = LET[I3] & LET[J3] & LET[K3] & (n >= 30)
    A = np.c_[pat, Lp]; b = np.linalg.lstsq(A[ok] * np.sqrt(n[ok])[:, None], c[ok] * np.sqrt(n[ok]), rcond=None)[0]
    struct = pat @ b[:-1]; motor = c - b[-1] * Lp
    use = LET[I3] & LET[J3] & LET[K3] & (n > 0)
    out = np.where(use, (n * motor + k * struct) / (n + k), struct)
    return out.reshape(N, N, N)

class Judge:
    def __init__(self, half, corpus='english', letters=False):
        self.P, self.info = pair_table(half, letters=letters); self.T3 = tri_table(half)
        self.C = Corpus(f'/home/josh/projects/gecko-workspace/costfit/data/corpus_{corpus}.json')
    def __call__(self, chars):
        return self.C.judge2(chars, self.P) + self.C.judge3(chars, self.T3)
    def parts(self, chars):
        return self.C.judge2(chars, self.P), self.C.judge3(chars, self.T3)

if __name__ == '__main__':
    JA, JB = Judge(0, letters='-l' in sys.argv), Judge(1, letters='-l' in sys.argv)
    print('half A', JA.info, ' half B', JB.info)
    print('pair tables r(A,B) all pairs', np.corrcoef(JA.P.ravel(), JB.P.ravel())[0, 1])
    from scipy.stats import spearmanr
    L = lambda p: [json.loads(l) for l in open(p)]
    for nm in ('refs', 'cmini', 'random'):
        R_ = L(f'/home/josh/projects/gecko-workspace/costfit/data/{nm}.jsonl')
        a = np.array([JA(r['chars']) for r in R_]); b = np.array([JB(r['chars']) for r in R_]); s = np.array([r['score'] for r in R_])
        e01 = np.array([r['raw']['E01'] for r in R_])
        print(f"{nm:6} n={len(R_)}  rho(J_A,J_B) {spearmanr(a, b)[0]:.3f}  rho(score,J_B) {spearmanr(s, b)[0]:.3f}  rho(E01,J_B) {spearmanr(e01, b)[0]:.3f}")
        if nm == 'refs':
            for r, x in sorted(zip(R_, b), key=lambda t: t[1]):
                p2, p3 = JB.parts(r['chars']); print(f"   {r['name']:11} J_B {x:+.4f} (pair {p2:+.4f}, tri {p3:+.4f})  score {r['score']:.2f}")
