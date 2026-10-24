"""Shared pieces for studies 4-6: scales, fitted time weights, optimizer runs, judges."""
import json, sys, subprocess, tomllib, numpy as np
from concurrent.futures import ThreadPoolExecutor
from scipy.stats import spearmanr
sys.path.insert(0, '/home/josh/projects/gecko-workspace/costfit')
import s1_pairs as S1
import s2_trigrams as S2

W = '/home/josh/projects/gecko-workspace/costfit'
GECKO = '/home/josh/projects/gecko'
load = lambda p: [json.loads(l) for l in open(p)]
DEFAULT = tomllib.load(open(f'{GECKO}/data/profiles/default.toml', 'rb'))
REFS = load(f'{W}/data/refs.jsonl')
TIERS = {r['name']: t for r, t in zip(load(f'{W}/data/tiers.jsonl'), [1] * 10 + [2, 3, 4, 4, 4, 5])}
TIER_ROWS = load(f'{W}/data/tiers.jsonl')

def scales():
    """Spread of each unit contribution over the 9 references without QWERTY (bundled basis)."""
    rows = [r for r in REFS if r['name'] != 'qwerty']
    return {k: float(np.std([r['u'][k] for r in rows]) or 1.0) for k in rows[0]['u']}
SCALE = scales()
for k, v in DEFAULT['scale'].items():
    SCALE[k] = v  # keep the bundled scales where they exist (same basis)

BI = ['B01', 'B05', 'B06', 'B08', 'B09', 'B10', 'B14', 'E05', 'M01']
TRI = ['T03', 'T04', 'T07', 'S01', 'S04', 'T01', 'T06']

def fit_time(half=None, nuis='lin', bi=BI, tri=TRI):
    """Time coefficients (log time per keystroke per unit of each metric's table)."""
    S1.NUIS = nuis
    X = S1.cols(bi); n, y, var = S1.target(half, 0.0)
    off = np.array([i != j for i in range(S1.N) for j in range(S1.N)]); ok = off & S1.LETTER & (n >= 30)
    w = 1 / (np.mean(var[ok]) / np.maximum(n, 1) + 0.003)
    b, _, _ = S1.fit(X[ok], y[ok], w[ok], S1.nuis(n)[ok])
    X3 = np.array([S2.col(m) for m in tri]).T
    n3, y3, v3, pn3 = S2.stats(half)
    ok3 = S2.LETTER3 & (n3 >= 30) & (S2.I3 != S2.J3) & (S2.J3 != S2.K3)
    Z3 = np.c_[np.log(np.maximum(n3, 1)) - np.log(np.maximum(pn3, 1))]
    w3 = 1 / (np.mean(v3[ok3]) / np.maximum(n3, 1) + 0.002)
    b3, _, _ = S2.fit(X3[ok3], y3[ok3], w3[ok3], Z3[ok3])
    return {**dict(zip(bi, b)), **dict(zip(tri, b3))}

def to_weights(beta, k_b01=None):
    """Coefficients -> spread-unit profile weights: w/scale = K·beta/100 (% metrics), K·beta (M01).
    K puts B01 at the default's weight unless k_b01 says otherwise."""
    per = {m: (b if m == 'M01' else b / 100.0) for m, b in beta.items()}
    K = (DEFAULT['weights']['B01'] if k_b01 is None else k_b01) / (per['B01'] * SCALE['B01'])
    return {m: float(K * v * SCALE[m]) for m, v in per.items() if v > 0}, K

def score(row, weights):
    return sum(weights[k] / SCALE.get(k, 1.0) * row['u'][k] for k in weights)

def tier_rho(weights):
    s = [score(r, weights) for r in TIER_ROWS]
    return float(spearmanr(s, [TIERS[r['name']] for r in TIER_ROWS])[0])

def optimize(cands, evals=10_000_000, seeds=2, threads=4, par=5, corpus='english', geom='ansi-30', algo='sa'):
    """cands: {id: weights}. Runs costfit opt per candidate and seed; returns best row per id."""
    jobs = [(cid, w, s) for cid, w in cands.items() for s in range(1, seeds + 1)]
    def one(job):
        cid, w, s = job
        inp = json.dumps({'id': cid, 'weights': w, 'scale': {k: SCALE[k] for k in w}}) + '\n'
        out = subprocess.run([f'{W}/target/release/costfit', 'opt', '--algo', algo, '--evals', str(evals), '--threads', str(threads),
                              '--seed', str(s), '-c', corpus, '-g', geom], input=inp, capture_output=True, text=True, check=True).stdout
        return json.loads(out.strip().splitlines()[-1])
    with ThreadPoolExecutor(par) as ex:
        rows = list(ex.map(one, jobs))
    best = {}
    for r in rows:
        r['seeds_score'] = [x['opt_score'] for x in rows if x['id'] == r['id']]
        if r['id'] not in best or r['opt_score'] < best[r['id']]['opt_score']:
            best[r['id']] = r
    return best

def finger_loads(row):
    return {k: row['raw'][k] for k in ('M02', 'M04', 'M07') if k in row['raw']}
