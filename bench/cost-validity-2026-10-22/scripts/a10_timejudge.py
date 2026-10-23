"""Predicted typing time per layout from the fitted geometric model (no log-freq term: equal practice)."""
import sys; sys.path.insert(0,'.'); from lib import *
from scipy.stats import spearmanr
exec(open('a3b_iki_groups.py').read().split("FK=list(feats(0,1))")[0])  # reuse feats()
group=sys.argv[1]
M=json.load(open(f'data/timemodel_{group}.json')); b=np.array(M['b']); FK=M['FK']
fr=json.load(open('data/en_bigrams.json'))
FE=np.zeros((N,N))
for i in range(N):
    for j in range(N):
        if i!=j: f=feats(i,j); FE[i,j]=b[0]+sum(b[2+k]*f[K] for k,K in enumerate(FK))
def tjudge(chars):
    pos={c:i for i,c in enumerate(chars)}; t=z=0
    for k,v in fr.items():
        a,c=k
        if a==c or a not in pos or c not in pos: continue
        t+=v*FE[pos[a],pos[c]]; z+=v
    return t/z
sets={'refs':load('data/refs.jsonl'),'cmini':load('data/cmini.jsonl'),'random':load('data/random.jsonl')}
for k,R in sets.items():
    tj=np.array([tjudge(r['chars']) for r in R]); s=np.array([r['score'] for r in R])
    print(f"{k:7} n={len(R):5}  Spearman(GECKO score, predicted time) {spearmanr(s,tj)[0]:+.3f}")
    if k=='refs':
        for r,t in sorted(zip(R,tj),key=lambda x:x[1]): print(f"     {r['name']:11} time {t:.4f}  score {r['score']:.2f}")
good=[c for c in sets['cmini'] if c['score']<22]
tj=np.array([tjudge(r['chars']) for r in good]); s=np.array([r['score'] for r in good])
print(f"good cmini (score<22, n={len(good)}): Spearman {spearmanr(s,tj)[0]:+.3f}")
import os
if os.path.exists('data/sweep.jsonl'):
    SW=load('data/sweep.jsonl'); d=[r for r in SW if r['id']=='default'][0]
    allc=[tjudge(c['chars']) for c in sets['cmini']]
    t=tjudge(d['chars']); print(f"default optimum: predicted time {t:.4f}; percentile among cmini {np.mean(np.array(allc)<t)*100:.0f}% (lower=faster)")
    for nm in ('graphite','sturdy','qwerty'):
        r=[x for x in sets['refs'] if x['name']==nm][0]; print(f"  {nm}: {tjudge(r['chars']):.4f} pct {np.mean(np.array(allc)<tjudge(r['chars']))*100:.0f}%")
