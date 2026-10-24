"""What do GECKO's pair metrics miss? Residual structure of key-pair time."""
import sys, numpy as np; sys.path.insert(0,'.')
from s1_pairs import *
from ept import KEYS, FING, ROW
ids = ['B01','B05','B06','B08','B09','B10','B14','E05','M01','G05']
X = cols(ids); n, y, var = target()
off = np.array([i != j for i in range(N) for j in range(N)]); ok = off & (n >= 30)
w = 1/(np.mean(var[ok])/np.maximum(n,1)+0.003)
I = np.array([i for i in range(N) for j in range(N)]); J = np.array([j for i in range(N) for j in range(N)])
def onehot(v, k=N): M = np.zeros((len(v), k)); M[np.arange(len(v)), v] = 1; return M
def cv(Xm, signed=None):
    rng = np.random.default_rng(0); grp = np.minimum(I,J)*N+np.maximum(I,J); ug=np.unique(grp[ok]); fold=dict(zip(ug,rng.integers(0,10,len(ug))))
    f=np.array([fold.get(g,-1) for g in grp]); pred=np.zeros(N*N)
    for k in range(10):
        tr=ok&(f!=k); te=ok&(f==k); sw=np.sqrt(w[tr])
        A=np.c_[np.ones(tr.sum()), Xm[tr]]; b=np.linalg.lstsq(A*sw[:,None], y[tr]*sw, rcond=1e-8)[0]
        pred[te]=np.c_[np.ones(te.sum()), Xm[te]]@b
    return r2(y[ok],pred[ok],w[ok])
print('metrics (OLS)                ', round(cv(X),3))
print('metrics + key j              ', round(cv(np.c_[X, onehot(J)[:,1:]]),3))
print('metrics + key i + key j      ', round(cv(np.c_[X, onehot(I)[:,1:], onehot(J)[:,1:]]),3))
print('key i + key j only           ', round(cv(np.c_[onehot(I)[:,1:], onehot(J)[:,1:]]),3))
F=np.array(FING); fi=F[I]; fj=F[J]
sf=(fi==fj)&(I!=J); sh=((fi<5)==(fj<5))&~sf
ft=np.zeros((N*N,100)); ft[np.arange(N*N), fi*10+fj]=1   # finger-pair type
print('finger pair (100 cells)      ', round(cv(ft[:, ft[ok].sum(0)>0][:,1:]),3))
rowt=np.zeros((N*N,9)); rowt[np.arange(N*N), np.array(ROW)[I]*3+np.array(ROW)[J]]=1
print('finger pair x row pair       ', round(cv(np.c_[ft[:, ft[ok].sum(0)>0][:,1:], rowt[:,1:]*sh[:,None], rowt[:,1:]*sf[:,None], rowt[:,1:]]),3))
print('metrics + finger pair + rows ', round(cv(np.c_[X, ft[:, ft[ok].sum(0)>0][:,1:], rowt[:,1:]*sh[:,None], rowt[:,1:]]),3))
# residuals of the metric fit
sw=np.sqrt(w[ok]); A=np.c_[np.ones(ok.sum()),X[ok]]; b=np.linalg.lstsq(A*sw[:,None],y[ok]*sw,rcond=None)[0]
res=np.full(N*N,np.nan); res[ok]=y[ok]-A@b
print('largest residuals (pair: resid, n)')
o=np.argsort(-np.abs(np.nan_to_num(res)))[:24]
print('  '+'  '.join(f"{KEYS[k//N]}{KEYS[k%N]}:{res[k]:+.2f}({int(n[k])})" for k in o))
# residual by pair class
for nm,mask in [('same finger',sf),('same hand',sh),('alternate',~sf&~sh&(I!=J))]:
    m=ok&mask; print(f"  {nm:12} mean resid {np.average(res[m],weights=w[m]):+.3f} sd {np.sqrt(np.average(res[m]**2,weights=w[m])):.3f} n {m.sum()}")
# residual by key j (first/second) means
rj=[np.nanmean(np.where(ok&(J==j),res,np.nan)) for j in range(N)]; ri=[np.nanmean(np.where(ok&(I==j),res,np.nan)) for j in range(N)]
print('  mean resid as 2nd key:', ' '.join(f"{KEYS[j]}{rj[j]:+.2f}" for j in range(N)))
print('  mean resid as 1st key:', ' '.join(f"{KEYS[j]}{ri[j]:+.2f}" for j in range(N)))
