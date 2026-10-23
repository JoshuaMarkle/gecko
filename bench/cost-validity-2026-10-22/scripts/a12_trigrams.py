import sys, csv; sys.path.insert(0,'.'); from lib import *
from scipy.stats import spearmanr
T=json.load(open('data/tables_ansi30.json')); S=T['slots']
q="qwertyuiopasdfghjkl;zxcvbnm,./"; slot={c:i for i,c in enumerate(q)}
F=lambda c:S[slot[c]]['finger']; H=lambda c:F(c)<5
group=sys.argv[1]
big={r['bigram']:float(r['median_rel']) for r in csv.DictReader(open('/home/josh/projects/gecko-workspace/aalto-analysis/out2/bigrams_groups.csv')) if r['group']==group}
rows=[r for r in csv.DictReader(open('/home/josh/projects/gecko-workspace/aalto-analysis/out2/trigrams_groups.csv')) if r['group']==group and int(r['n'])>=200]
def cls(a,b,c):
    if len({a,b,c})<3 or F(a)==F(b) or F(b)==F(c): return None   # skip repeats and SFB-containing
    ha,hb,hc=H(a),H(b),H(c)
    sfs = F(a)==F(c)
    if ha==hb==hc:
        fa,fb,fc=F(a),F(b),F(c)
        mono=(fa<fb<fc) or (fa>fb>fc)
        if mono: return 'onehand'
        idx={3,6}
        return 'redirect' if (fa in idx or fb in idx or fc in idx) else 'bad_redirect'
    if ha!=hb and hb!=hc: return 'alternate'
    return 'roll'
D=[]
for r in rows:
    a,b,c=r['trigram']
    if a not in slot or b not in slot or c not in slot or b+c not in big: continue
    k=cls(a,b,c)
    if k is None: continue
    D.append((r['trigram'],k,F(a)==F(c) and a!=c,int(r['n']),float(r['mean_logrel2']),float(r['mean_logrel_span']),np.log(big[b+c])))
print(group,'trigrams',len(D))
y=np.array([d[4] for d in D]); ln=np.log([d[3] for d in D]); bc=np.array([d[6] for d in D]); sfs=np.array([d[2] for d in D],float)
K=['alternate','roll','onehand','redirect','bad_redirect']
Xk=np.array([[d[1]==k for k in K[1:]] for d in D],float)
A=np.column_stack([np.ones(len(D)),ln,bc,sfs,Xk]); wt=np.minimum([d[3] for d in D],np.percentile([d[3] for d in D],90))
W=np.sqrt(wt); b,*_=np.linalg.lstsq(A*W[:,None],y*W,rcond=None)
names=['const','log trigram freq','log bigram(b,c) time','SFS (a,c same finger)']+[f'class {k} (vs alternate)' for k in K[1:]]
print("effect on log time of 3rd keystroke (×100 ≈ % slower), controlling bigram b→c time and trigram freq:")
for nm,c in zip(names,b): print(f"   {nm:32} {c*100:+6.1f}")
print("counts:", {k:sum(d[1]==k for d in D) for k in K}, 'sfs', int(sfs.sum()))
rng=np.random.default_rng(0); B=[]
for _ in range(1000):
    i=rng.integers(0,len(D),len(D)); Wi=np.sqrt(wt[i])
    bb,*_=np.linalg.lstsq(A[i]*Wi[:,None],y[i]*Wi,rcond=None); B.append(bb)
B=np.array(B)*100
for j,nm in enumerate(names):
    if j>=3: print(f"   95% CI {nm:32} [{np.percentile(B[:,j],2.5):+.1f}, {np.percentile(B[:,j],97.5):+.1f}]")
