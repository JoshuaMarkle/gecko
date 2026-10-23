import sys; sys.path.insert(0,'.'); from lib import *
p=profile(); C=load('data/cmini.jsonl'); ids=ids_of(C); e=eff(p,ids)
used=[i for i,x in zip(ids,e) if x!=0]; ui=[ids.index(i) for i in used]
for cut in (26,22,20):
    G=[c for c in C if c['score']<cut]
    X=matrix(G,ids)[:,ui]*e[ui]; s=X.sum(1)
    sh={u:np.cov(X[:,j],s)[0,1]/s.var(ddof=1) for j,u in enumerate(used)}
    top=sorted(sh.items(),key=lambda t:-t[1])
    print(f"cmini score<{cut} (n={len(G)}, sd {s.std():.2f}): "+', '.join(f"{k} {v*100:.0f}%" for k,v in top[:7]))
G=[c for c in C if c['score']<22]
X=matrix(G,ids)[:,ui]; r=np.corrcoef(X.T)
print("strong correlations among good layouts (score<22):")
for a in range(len(used)):
    for b in range(a+1,len(used)):
        if abs(r[a,b])>=0.6: print(f"  {used[a]}-{used[b]} {r[a,b]:+.2f}")
Z=(X-X.mean(0))/X.std(0); ev=np.linalg.eigvalsh(np.cov(Z.T))[::-1]; ev/=ev.sum()
print('participation ratio among good:',round(1/np.sum(ev**2),1))
