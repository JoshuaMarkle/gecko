"""Study 6: bilevel weight search. CMA-ES over log-weights of the profile's metrics; each candidate
is scored by optimizing a layout under it (SA) and judging that optimum:
  f = (1-a)·T_B(opt)/sd_T + a·E05(opt)/sd_E + penalties
T_B = time model fitted on typist half B (the weights never see it directly), E05 = Engram measured
comfort. Penalties keep community sanity: tier rho of the weights >= RHO_MIN, and the optimum's
finger-load error M02 <= M02_MAX (the default optimum's). Every evaluation is logged.
usage: s6_cma.py ALPHA RHO_MIN GENERATIONS TAG"""
import json, sys, time, numpy as np
from lib4 import *

if __name__ == '__main__':
    ALPHA = float(sys.argv[1]); RHO_MIN = float(sys.argv[2]); GENS = int(sys.argv[3]); TAG = sys.argv[4]
else:
    ALPHA, RHO_MIN, GENS, TAG = 0.5, 0.8, 0, ''
EVALS = 3_000_000
beta_B = fit_time(1)
T = lambda r: sum((b if m == 'M01' else b / 100) * r['u'][m] for m, b in beta_B.items())
nonq = [r for r in REFS if r['name'] != 'qwerty']
sdT = float(np.std([T(r) for r in nonq])); sdE = SCALE['E05']
M02_MAX = 2.66  # default optimum (study 4)
w0 = dict(DEFAULT['weights']); w0['B08'] = 0.5
IDS = [k for k in w0 if k != 'B01']

def weights(x):
    w = {k: float(w0[k] * np.exp(v)) for k, v in zip(IDS, x)}; w['B01'] = w0['B01']; return w

def objective(rows, ws):
    out = []
    for r, w in zip(rows, ws):
        t, e, rho, m02 = T(r) / sdT, r['raw']['E05'] / sdE, tier_rho(w), r['raw']['M02']
        f = (1 - ALPHA) * t + ALPHA * e + 10 * max(0, RHO_MIN - rho) + 2 * max(0, m02 - M02_MAX)
        out.append((f, t, e, rho, m02))
    return out

def run():
    d = len(IDS); lam = 16; mu = lam // 2
    wts = np.log(mu + 0.5) - np.log(np.arange(1, mu + 1)); wts /= wts.sum(); mueff = 1 / np.sum(wts ** 2)
    cc = (4 + mueff / d) / (d + 4 + 2 * mueff / d); cs = (mueff + 2) / (d + mueff + 5)
    c1 = 2 / ((d + 1.3) ** 2 + mueff); cmu = min(1 - c1, 2 * (mueff - 2 + 1 / mueff) / ((d + 2) ** 2 + mueff))
    damps = 1 + 2 * max(0, np.sqrt((mueff - 1) / (d + 1)) - 1) + cs; chiN = np.sqrt(d) * (1 - 1 / (4 * d) + 1 / (21 * d * d))
    m = np.zeros(d); sigma = 0.5; C = np.eye(d); pc = np.zeros(d); ps = np.zeros(d)
    rng = np.random.default_rng(int(ALPHA * 100) + 7)
    log = open(f'{W}/data/s6_{TAG}.jsonl', 'w'); best = (1e9, None)
    for g in range(GENS):
        t0 = time.time()
        Bm, Dv = np.linalg.eigh(C)[1], np.sqrt(np.maximum(np.linalg.eigh(C)[0], 1e-20))
        Z = rng.standard_normal((lam, d)); Y = Z @ np.diag(Dv) @ Bm.T; X = m + sigma * Y
        if g == 0: X[0] = 0.0  # the default itself
        ws = [weights(x) for x in X]
        opt = optimize({f'{g}:{i}': w for i, w in enumerate(ws)}, evals=EVALS, seeds=1, threads=4, par=5)
        rows = [opt[f'{g}:{i}'] for i in range(lam)]
        fs = objective(rows, ws)
        for i in range(lam):
            log.write(json.dumps({'gen': g, 'i': i, 'f': fs[i][0], 'T': fs[i][1], 'E': fs[i][2], 'rho': fs[i][3], 'M02': fs[i][4],
                                  'chars': rows[i]['chars'], 'weights': ws[i], 'raw': rows[i]['raw']}) + '\n')
            if fs[i][0] < best[0]: best = (fs[i][0], ws[i], rows[i]['chars'], fs[i])
        log.flush()
        order = np.argsort([f[0] for f in fs]); Ysel = Y[order[:mu]]; yw = wts @ Ysel
        m = m + sigma * yw
        Cih = Bm @ np.diag(1 / Dv) @ Bm.T
        ps = (1 - cs) * ps + np.sqrt(cs * (2 - cs) * mueff) * Cih @ yw
        hs = np.linalg.norm(ps) / np.sqrt(1 - (1 - cs) ** (2 * (g + 1))) < (1.4 + 2 / (d + 1)) * chiN
        pc = (1 - cc) * pc + hs * np.sqrt(cc * (2 - cc) * mueff) * yw
        C = (1 - c1 - cmu) * C + c1 * (np.outer(pc, pc) + (1 - hs) * cc * (2 - cc) * C) + cmu * (Ysel.T * wts) @ Ysel
        sigma *= np.exp((cs / damps) * (np.linalg.norm(ps) / chiN - 1))
        fb = fs[order[0]]
        print(f"gen {g:2} best f {fb[0]:+.3f} (T {fb[1]:+.2f} E {fb[2]:.2f} rho {fb[3]:.2f} M02 {fb[4]:.2f})  overall {best[0]:+.3f}  sigma {sigma:.3f}  {time.time() - t0:.0f}s", flush=True)
    json.dump({'alpha': ALPHA, 'rho_min': RHO_MIN, 'best_f': best[0], 'weights': best[1], 'chars': best[2], 'parts': best[3], 'mean_weights': weights(m),
               'sdT': sdT, 'sdE': sdE}, open(f'{W}/data/s6_{TAG}_best.json', 'w'), indent=1)

if __name__ == '__main__':
    run()
