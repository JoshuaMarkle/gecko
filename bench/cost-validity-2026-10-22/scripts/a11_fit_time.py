import sys; sys.path.insert(0,'.'); from lib import *
from scipy.optimize import nnls
from scipy.stats import spearmanr
exec(open('a10_timejudge.py').read().split("sets={")[0].replace("group=sys.argv[1]","group='touch_fast_rel'"))
p=profile(); w0=p['weights']; sc=p['scale']
R=load('data/random.jsonl')+load('data/cmini.jsonl'); ids=ids_of(R)
cand=[k for k in w0]+['B10','E04','B09','M05','B05','B06','T02','T09','T10']
cand=list(dict.fromkeys(cand))
X=np.array([[r['u'][k]/sc.get(k,np.std([x['u'][k] for x in R])) for k in cand] for r in R])  # spread-ish units
y=np.array([tjudge(r['chars']) for r in R])
# fit on random+cmini, sign convention: u already signed (Max negative); nonneg weights
Xc=X-X.mean(0); yc=y-y.mean()
wfit,_=nnls(Xc,yc)
pred=Xc@wfit
print(f"NNLS fit of time judge from GECKO metrics: Spearman {spearmanr(pred,y)[0]:.3f}")
tot=sum(wfit)
w0n={k:w0.get(k,0)/sum(w0.values()) for k in cand}
print(f"{'metric':6} {'fit share':>9} {'default share':>13}")
for k,v in sorted(zip(cand,wfit),key=lambda t:-t[1]):
    if v>0 or w0.get(k,0)>0: print(f"{k:6} {v/tot*100:8.1f}% {w0n[k]*100:12.1f}%")
json.dump({k:float(v/tot*sum(w0.values())) for k,v in zip(cand,wfit) if v>0},open('data/w_timefit.json','w'))
