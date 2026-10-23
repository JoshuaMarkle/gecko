import sys; sys.path.insert(0,'.'); from lib import *
p=profile(); w0=p['weights']
C=load('data/cmini.jsonl')+load('data/refs.jsonl'); ids=ids_of(C); X=matrix(C,ids)
print("B14 weight (default 2.0) → #1 layout, its SFB % and stretch; rank of graphite/sturdy/semimak")
nm=[c['name'] for c in C]
for f in (0,0.25,0.5,1,2,4,8):
    w={**w0,'B14':w0['B14']*f}; s=X@eff(p,ids,w); o=np.argsort(s); i=o[0]
    rk={x:int(np.where(o==nm.index(x))[0][0])+1 for x in ('graphite','sturdy','semimak')}
    print(f"  B14={w['B14']:5.2f}: #1 {nm[i]:24} SFB {C[i]['raw']['B01']:.2f} B14 {C[i]['raw']['B14']:.2f}  ranks {rk}")
print("B01 weight (default 8.28):")
for f in (0.25,0.5,1,2):
    w={**w0,'B01':w0['B01']*f}; s=X@eff(p,ids,w); o=np.argsort(s); i=o[0]
    rk={x:int(np.where(o==nm.index(x))[0][0])+1 for x in ('graphite','sturdy','semimak')}
    print(f"  B01={w['B01']:5.2f}: #1 {nm[i]:24} SFB {C[i]['raw']['B01']:.2f} B14 {C[i]['raw']['B14']:.2f}  ranks {rk}")
# Pareto front in (SFB, stretch) among all
pts=sorted(((c['raw']['B01'],c['raw']['B14'],c['name']) for c in C))
front=[];best=1e9
for a,b,n in pts:
    if b<best: front.append((a,b,n)); best=b
print("SFB–stretch Pareto front (lower both):", '; '.join(f"{n} ({a:.2f}%, {b:.2f})" for a,b,n in front[:14]))
