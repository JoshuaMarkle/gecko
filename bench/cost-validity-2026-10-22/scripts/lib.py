import json, numpy as np, tomllib
GECKO = '/home/josh/projects/gecko'
def load(path):
    return [json.loads(l) for l in open(path)]
def profile(name='default'):
    p = tomllib.load(open(f'{GECKO}/data/profiles/{name}.toml','rb'))
    return p
def ids_of(rows):
    return sorted(rows[0]['u'].keys())
def matrix(rows, ids):
    return np.array([[r['u'][i] for i in ids] for r in rows])
def eff(p, ids, w=None):
    w = p['weights'] if w is None else w
    s = p.get('scale', {})
    return np.array([w.get(i, 0.0)/s.get(i, 1.0) for i in ids])
