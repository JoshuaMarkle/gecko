"""Study 5: evidence profiles. Objective = (1-a)·T/sd_T + a·E05/sd_E + g·(M02, M04), T = fitted
time model. Sweep comfort share a and ergonomic weight g; optimize; judge each optimum."""
import json, sys
from lib4 import *
from judge import Judge
JB = Judge(1); JBl = Judge(1, letters=True)
beta = fit_time(None)
nonq = [r for r in REFS if r['name'] != 'qwerty']
T = lambda r: sum((b if m == 'M01' else b / 100) * r['u'][m] for m, b in beta.items())
sdT = float(np.std([T(r) for r in nonq])); sdE = SCALE['E05']
print('sd_T over 9 refs', sdT, '(log time/keystroke)')
def profile(a, g):
    w = {m: (1 - a) * (b if m == 'M01' else b / 100) * SCALE[m] / sdT for m, b in beta.items() if b > 0}
    if a > 0: w['E05'] = w.get('E05', 0) + a
    if g > 0: w['M02'] = g; w['M04'] = g
    return w
cands = {f'a{a}_g{g}': profile(a, g) for a in (0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.8, 1.0) for g in (0, 0.05, 0.15)}
best = optimize(cands, evals=10_000_000, seeds=2)
out = {'sdT': sdT, 'beta': beta, 'rows': {}}
print(f"{'cand':10} {'layout':32} {'T%':>6} {'E05':>5} {'J_B':>7} {'J_B+l':>7} {'E01':>5} {'SFB':>5} {'M02':>5} {'pinky%':>6} {'home':>5} {'dflt':>6} {'tierρ':>5}")
for cid, w in cands.items():
    r = best[cid]; c = r['chars']; raw = r['raw']
    rec = {'chars': c, 'T': T(r), 'E05': raw['E05'], 'J_B': JB(c), 'J_B_letters': JBl(c), 'E01': raw['E01'], 'SFB': raw['B01'],
           'M02': raw['M02'], 'home': raw['M05'], 'default_score': score(r, DEFAULT['weights']), 'tier_rho': tier_rho(w), 'weights': w,
           'seeds': r['seeds_score'], 'raw': raw}
    out['rows'][cid] = rec
    print(f"{cid:10} {c[:10]+' '+c[10:20]+' '+c[20:]:32} {rec['T']*100:+6.2f} {rec['E05']:5.2f} {rec['J_B']:+.4f} {rec['J_B_letters']:+.4f} {rec['E01']:5.2f} {rec['SFB']:5.2f} {rec['M02']:5.2f} {'':>6} {rec['home']:5.1f} {rec['default_score']:6.2f} {rec['tier_rho']:5.2f}")
json.dump(out, open(f'{W}/data/s5_sweep.json', 'w'), indent=1)
