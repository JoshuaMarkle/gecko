import sys; sys.path.insert(0,'.'); from lib import *
p=profile(); w=p['weights']
sets={k:load(f'data/{k}.jsonl') for k in ('refs','cmini','random')}
ids=ids_of(sets['refs']); e=eff(p,ids); used=[i for i,x in zip(ids,e) if x!=0]
ui=[ids.index(i) for i in used]
print("== variance share of score by metric: Cov(c_m, score)/Var(score) ==")
print(f"{'metric':6}"+''.join(f"{k:>9}" for k in sets))
shares={}
for k,R in sets.items():
    C=matrix(R,ids)[:,ui]*e[ui]; s=C.sum(1)
    shares[k]=[np.cov(C[:,j],s)[0,1]/s.var(ddof=1) for j in range(len(used))]
for j,i in enumerate(used):
    print(f"{i:6}"+''.join(f"{shares[k][j]*100:9.1f}" for k in sets))
for k,R in sets.items():
    C=matrix(R,ids)[:,ui]*e[ui]; s=C.sum(1)
    print(k,'n',len(R),'score mean %.2f sd %.2f min %.2f'%(s.mean(),s.std(),s.min()))
print("\n== correlation of weighted metrics, cmini set (|r|>=0.5) ==")
C=matrix(sets['cmini'],ids)[:,ui]
r=np.corrcoef(C.T)
for a in range(len(used)):
    for b in range(a+1,len(used)):
        if abs(r[a,b])>=0.5: print(f"  {used[a]}-{used[b]} {r[a,b]:+.2f}")
for k in ('cmini','random'):
    C=matrix(sets[k],ids)[:,ui]; Z=(C-C.mean(0))/C.std(0)
    ev=np.linalg.eigvalsh(np.cov(Z.T))[::-1]; ev/=ev.sum()
    pr=1/np.sum(ev**2)
    print(f"{k}: PCA top5 var {np.round(ev[:5]*100,1)} cum {np.round(np.cumsum(ev)[[0,2,4,7]]*100,1)}  participation ratio {pr:.1f} of {len(used)}")
