"""Study 7: pool the top distinct candidates of every CMA-ES run (+ CMA means, default, s4/s5
profiles), re-optimize each with 10M evals x 3 seeds, score under every alpha."""
import json
from lib4 import *
import s6_cma as S6
cands = {'default': DEFAULT['weights']}
for tag in ('a50', 'a25', 'a75', 'a50free'):
    log = sorted(load(f'{W}/data/s6_{tag}.jsonl'), key=lambda r: r['f']); seen = set(); k = 0
    for r in log:
        if r['chars'] in seen: continue
        seen.add(r['chars']); cands[f'{tag}:top{k}'] = r['weights']; k += 1
        if k == 6: break
    cands[f'{tag}:mean'] = json.load(open(f'{W}/data/s6_{tag}_best.json'))['mean_weights']
opt = optimize(cands, evals=10_000_000, seeds=3, threads=4, par=5)
out = {}
for a in (0.25, 0.5, 0.75):
    S6.ALPHA = a
    for rmin in (0.8, 0.0):
        S6.RHO_MIN = rmin
        fs = S6.objective([opt[i] for i in cands], [cands[i] for i in cands])
        ranked = sorted(zip(cands, fs), key=lambda t: t[1][0])
        print(f"alpha {a} rho_min {rmin}: " + ' | '.join(f"{i} {f[0]:+.3f}" for i, f in ranked[:4]) + f" | default {dict(zip(cands, fs))['default'][0]:+.3f}")
for i in cands:
    o = opt[i]; S6.ALPHA = 0.5; S6.RHO_MIN = 0.8
    f = S6.objective([o], [cands[i]])[0]
    out[i] = {'chars': o['chars'], 'T': f[1], 'E': f[2], 'rho': f[3], 'M02': f[4], 'seeds': o['seeds_score'], 'weights': cands[i], 'raw': o['raw']}
json.dump(out, open(f'{W}/data/s7_pool.json', 'w'), indent=1)
print(f"{'id':14} {'T':>6} {'E':>6} {'rho':>5} {'M02':>5} {'SFB':>5} layout  seeds")
for i, r in sorted(out.items(), key=lambda t: t[1]['T'] + t[1]['E']):
    print(f"{i:14} {r['T']:+6.2f} {r['E']:6.3f} {r['rho']:5.2f} {r['M02']:5.2f} {r['raw']['B01']:5.2f} {r['chars']} {[round(x,3) for x in r['seeds']]}")
