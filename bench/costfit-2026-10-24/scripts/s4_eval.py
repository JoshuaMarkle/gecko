"""Study 4: candidate profiles -> optimized layout -> judges."""
import json, sys, tomllib
from lib4 import *
from judge import Judge
JB = Judge(1); JBl = Judge(1, letters=True)
beta_all = fit_time(None); beta_A = fit_time(0); beta_l = fit_time(None, 'letters')
T_ALL, K_ALL = to_weights(beta_all)
def tmodel(row, beta=beta_all):  # parametric time model (log time per keystroke)
    return sum((b if m == 'M01' else b / 100) * row['u'][m] for m, b in beta.items())
tf = tomllib.load(open(f'{GECKO}/bench/cost-validity-2026-10-22/timefit.toml', 'rb'))
ERGO = {k: DEFAULT['weights'][k] for k in ('M02', 'M04')}
cands = {'default': DEFAULT['weights'], 'timefit-1022': tf['weights'],
         'time': T_ALL, 'timeA': to_weights(beta_A)[0], 'time-letters': to_weights(beta_l)[0]}
for lam in (0, 0.6, 1.2, 2.5, 5.0):
    cands[f'time+ergo+E05x{lam}'] = {**T_ALL, **ERGO, **({'E05': lam} if lam else {})}
best = optimize(cands, evals=int(sys.argv[1]) if len(sys.argv) > 1 else 10_000_000)
refs = {r['name']: r for r in REFS}
rows = [(cid, best[cid]) for cid in cands] + [(n, refs[n]) for n in ('graphite', 'sturdy', 'canary', 'semimak', 'dvorak', 'qwerty')]
print(f"{'candidate':22} {'layout':32} {'J_B':>7} {'J_B+l':>7} {'Tmod':>7} {'E01':>5} {'E05':>5} {'SFB':>5} {'alt%':>5} {'roll%':>5} {'home%':>5} {'M02':>5} {'dflt':>6} {'tierρ':>5} seeds")
out = {}
for cid, r in rows:
    c = r['chars']; raw = r['raw']
    rec = {'chars': c, 'J_B': JB(c), 'J_B_letters': JBl(c), 'Tmodel': tmodel(r), 'E01': raw['E01'], 'E05': raw['E05'], 'SFB': raw['B01'],
           'alt': raw['B08'], 'rolls': raw['T03'] + raw['T04'], 'home': raw['M05'], 'M02': raw['M02'],
           'default_score': score(r, DEFAULT['weights']), 'tier_rho': tier_rho(cands[cid]) if cid in cands else None,
           'seeds': r.get('seeds_score'), 'raw': raw, 'weights': cands.get(cid)}
    out[cid] = rec
    print(f"{cid:22} {c[:10]+' '+c[10:20]+' '+c[20:]:32} {rec['J_B']:+.4f} {rec['J_B_letters']:+.4f} {rec['Tmodel']:+.4f} {rec['E01']:5.2f} {rec['E05']:5.2f} {rec['SFB']:5.2f} {rec['alt']:5.1f} {rec['rolls']:5.1f} {rec['home']:5.1f} {rec['M02']:5.2f} {rec['default_score']:6.2f} {rec['tier_rho'] if rec['tier_rho'] is not None else float('nan'):5.2f} {[round(x,3) for x in rec['seeds']] if rec['seeds'] else ''}")
json.dump({'beta_all': beta_all, 'beta_A': beta_A, 'beta_letters': beta_l, 'K': K_ALL, 'results': out}, open(f'{W}/data/s4_eval.json', 'w'), indent=1)
