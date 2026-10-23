import sys; sys.path.insert(0,'.'); from lib import *
from scipy.stats import kendalltau, spearmanr
p=profile(); w0=p['weights']
R=load('data/refs.jsonl'); ids=ids_of(R); X=matrix(R,ids); names=[r['name'] for r in R]
keys=list(w0); rng=np.random.default_rng(1)
s0=X@eff(p,ids); order0=np.argsort(s0)
print('default order:',[names[i] for i in order0])
for sig in (0.25,0.5,1.0,2.0):
    taus=[];top=np.zeros(len(R));pair=np.zeros((len(R),len(R)))
    N=4000
    for _ in range(N):
        w={k:v*np.exp(rng.normal(0,sig)) for k,v in w0.items()}
        s=X@eff(p,ids,w); taus.append(kendalltau(s0,s)[0]); top[np.argmin(s)]+=1
        pair+= (s[:,None]<s[None,:])
    taus=np.array(taus)
    print(f"sigma {sig}: tau median {np.median(taus):.3f} p5 {np.percentile(taus,5):.3f}; P(top) "+', '.join(f"{names[i]} {top[i]/N:.2f}" for i in np.argsort(-top)[:4]))
    if sig==1.0:
        print("  pairs that flip in >10% of draws (A<B by default):")
        for a in order0:
            for b in order0:
                if s0[a]<s0[b] and pair[b,a]/N>0.10: print(f"    {names[a]} < {names[b]}: holds {pair[a,b]/N:.2f}")
# dominance: A beats B for every nonnegative weight vector over the weighted metrics?
e=eff(p,ids); used=[j for j,x in enumerate(e) if x!=0]
sign=np.array([1.0]*len(ids))
C=X[:,used]  # signed unit contributions (Max metrics already negative)
print("\nPareto dominance among references (lower on every weighted metric):")
for a in range(len(R)):
    dom=[names[b] for b in range(len(R)) if a!=b and np.all(C[a]<=C[b]+1e-12)]
    if dom: print(f"  {names[a]} dominates {dom}")
# qwerty excluded: which pairs are not dominated
