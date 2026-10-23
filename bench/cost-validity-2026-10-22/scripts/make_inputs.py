"""Layout lists (refs, cmini, random) and the weight-sweep design, as used on 2026-10-22."""
import sys, json, random, numpy as np; sys.path.insert(0, '.'); from lib import *
import os
refs = sorted(f[:-5] for f in os.listdir(f'{GECKO}/data/layouts') if f.endswith('.toml'))
open('data/refs.txt', 'w').write('\n'.join(refs) + '\n')
b = json.load(open(f'{GECKO}/data/community/cmini.json'))
L = [l for l in b['layouts'] if not l.get('dup_of')]
open('data/cmini.txt', 'w').write('\n'.join('cmini:' + l['name'] for l in L) + '\n')
random.seed(7); q = list("qwertyuiopasdfghjkl;zxcvbnm,./")
open('data/random.txt', 'w').write('\n'.join('raw:' + ''.join(random.sample(q, 30)) for _ in range(3000)) + '\n')
p = profile(); w0 = p['weights']; out = open('data/w_sweep.jsonl', 'w')
def emit(i, w): out.write(json.dumps({'id': i, 'weights': w}) + '\n')
emit('default', w0)
for k in w0:
    emit(f'drop:{k}', {**w0, k: 0.0}); emit(f'half:{k}', {**w0, k: w0[k] * 0.5}); emit(f'double:{k}', {**w0, k: w0[k] * 2})
rng = np.random.default_rng(42)
for s, n in ((0.5, 150), (1.0, 100)):
    for j in range(n):
        emit(f'mc{s}:{j}', {k: float(v * np.exp(rng.normal(0, s))) for k, v in w0.items()})
json.dump({'id': 'default', 'weights': w0}, open('data/w_default.jsonl', 'w'))
