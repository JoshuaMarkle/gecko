import sys; sys.path.insert(0,'.')
from s2_trigrams import *
from ept import FING
n, y, var, pn = stats(); ok = LETTER3 & (n >= 30) & (I3 != J3) & (J3 != K3)
w = 1/(np.mean(var[ok])/np.maximum(n,1)+0.002)
F = np.array(FING); H = (F >= 5).astype(int)
hi, hj, hk = H[I3], H[J3], H[K3]; fi, fj, fk = F[I3], F[J3], F[K3]
Z = np.log(np.maximum(n,1)) - np.log(np.maximum(pn,1))
def ols(cols, names):
    A = np.c_[np.ones(len(Z)), Z, np.array(cols).T][ok]; sw=np.sqrt(w[ok])
    b = np.linalg.lstsq(A*sw[:,None], y[ok]*sw, rcond=None)[0]; p=A@b
    print(f"  R2 {r2(y[ok],p,w[ok]):.3f}"); [print(f"   {nm:28} {v:+.4f}") for nm,v in zip(['icpt','practice']+names,b)]
print('GECKO classes, unconstrained:')
ids=['T01','T03','T04','T06','T07','S01','S04']; ols([col(m) for m in ids], ids)
print('hand pattern of (i, j, k) (vs i same hand as j and k = one-hand):')
pat = [((hi==hj)&(hj!=hk)), ((hi!=hj)&(hj==hk)), ((hi!=hj)&(hj!=hk))]
ols(pat + [(I3==K3).astype(float), ((fi==fk)&(I3!=K3)).astype(float)], ['i,j same hand, k other','i other, j,k same (into roll)','alternate LRL','i==k same key','i,k same finger (SFS)'])
