import sys; sys.path.insert(0,'.'); from lib import *
p=profile(); w0=p['weights']; sc=p['scale']
R=load('data/refs.jsonl'); C=load('data/cmini.jsonl'); ids=ids_of(R)
from metrics_dir import DIR
def sd(rows,m): return np.std([r['u'][m] for r in rows],ddof=1)
noq=[r for r in R if r['name']!='qwerty']; good=[c for c in C if c['score']<26]
print(f"{'m':4} {'profile':>8} {'10 refs':>8} {'9 (no qwerty)':>13} {'good cmini':>10} {'eff w / B01 eff w, as is → no-qwerty → good':>40}")
base={k:w0[k]/sc[k] for k in w0}
for k in w0:
    s10,s9,sg=sd(R,k),sd(noq,k),sd(good,k)
    r=lambda s,s1: (w0[k]/s)/(w0['B01']/s1)
    print(f"{k:4} {sc[k]:8.3f} {s10:8.3f} {s9:13.3f} {sg:10.3f}   {r(sc[k],sc['B01']):8.3f} {r(s9,sd(noq,'B01')):8.3f} {r(sg,sd(good,'B01')):8.3f}")
# effect: re-rank refs and good cmini with spread units recomputed on 'good' set (same nominal weights)
for nm,rows in (('no-qwerty',noq),('good cmini',good)):
    e=np.array([w0.get(i,0)/sd(rows,i) if w0.get(i,0) else 0 for i in ids])
    s=matrix(C,ids)@e; s0=np.array([c['score'] for c in C])
    from scipy.stats import spearmanr
    top0=set(np.argsort(s0)[:20]); top1=set(np.argsort(s)[:20])
    print(f"scales from {nm}: Spearman with default over cmini {spearmanr(s,s0)[0]:.3f}; top-20 overlap {len(top0&top1)}/20; new #1 {C[int(np.argmin(s))]['name']} (default #1 {C[int(np.argmin(s0))]['name']})")
