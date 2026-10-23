"""Collect every number the cost-validity paper uses into results.json (aggregates only).
Inputs: data/*.jsonl from costlab (GECKO-derived), Aalto aggregate CSVs (workspace only)."""
import sys, csv, collections; sys.path.insert(0,'.'); from lib import *
from scipy.stats import spearmanr, kendalltau
OUT={}
p=profile(); w0=p['weights']; sc=p['scale']
REF=load('data/refs.jsonl'); CM=load('data/cmini.jsonl'); RA=load('data/random.jsonl')
ids=ids_of(REF); e0=eff(p,ids); used=[i for i,x in zip(ids,e0) if x!=0]; ui=[ids.index(i) for i in used]
# 1 variance shares
def shares(R):
    X=matrix(R,ids)[:,ui]*e0[ui]; s=X.sum(1); return {u:float(np.cov(X[:,j],s)[0,1]/s.var(ddof=1)) for j,u in enumerate(used)}
good=[c for c in CM if c['score']<22]
OUT['varshare']={'refs':shares(REF),'cmini':shares(CM),'good':shares(good),'random':shares(RA),'n_good':len(good)}
def pr(R):
    X=matrix(R,ids)[:,ui]; Z=(X-X.mean(0))/X.std(0); ev=np.linalg.eigvalsh(np.cov(Z.T))[::-1]; ev/=ev.sum(); return float(1/np.sum(ev**2))
OUT['participation']={'cmini':pr(CM),'good':pr(good),'random':pr(RA),'n_metrics':len(used)}
Xg=matrix(good,ids); OUT['corr_good']={'B01_B14':float(np.corrcoef(Xg[:,ids.index('B01')],Xg[:,ids.index('B14')])[0,1])}
# 2 rank robustness
X=matrix(REF,ids); nm=[r['name'] for r in REF]; s0=X@e0
modern=[nm.index(x) for x in ('graphite','sturdy','semimak','canary')]; classic=[nm.index(x) for x in ('dvorak','colemak','colemak-dh','workman','halmak')]
rng=np.random.default_rng(1); rob=[]
for sig in (0.25,0.5,0.75,1.0,1.5,2.0):
    taus=[];a=b=0;top=collections.Counter();N=4000
    for _ in range(N):
        w={k:v*np.exp(rng.normal(0,sig)) for k,v in w0.items()}; s=X@eff(p,ids,w)
        taus.append(kendalltau(s0,s)[0]); a+=s[modern].max()<s[classic].min(); b+=s[classic].max()<s[nm.index('qwerty')]; top[nm[int(np.argmin(s))]]+=1
    rob.append({'sigma':sig,'tau_med':float(np.median(taus)),'tau_p5':float(np.percentile(taus,5)),'p_modern':a/N,'p_classic':b/N,'p_top':{k:v/N for k,v in top.items()}})
a=b=0
for _ in range(4000):
    w={k:rng.exponential()*sc[k] for k in w0}; s=X@eff(p,ids,w); a+=s[modern].max()<s[classic].min(); b+=s[classic].max()<s[nm.index('qwerty')]
OUT['robust']={'mc':rob,'prior_free':{'p_modern':a/4000,'p_classic':b/4000}}
C=X[:,ui]; OUT['dominance']=[[nm[i],nm[j]] for i in range(len(nm)) for j in range(len(nm)) if i!=j and np.all(C[i]<=C[j]+1e-12)]
# 3 scales
def sd(R,m): return float(np.std([r['u'][m] for r in R],ddof=0))
noq=[r for r in REF if r['name']!='qwerty']; g26=[c for c in CM if c['score']<26]
OUT['scales']={k:{'profile':sc[k],'no_qwerty':sd(noq,k),'good':sd(g26,k)} for k in w0}
s_def=np.array([c['score'] for c in CM])
for nm_,rows in (('no_qwerty',noq),('good',g26)):
    e=np.array([w0.get(i,0)/sd(rows,i) if w0.get(i,0) else 0 for i in ids]); s=matrix(CM,ids)@e
    OUT['scales'][f'rerank_{nm_}']={'rho':float(spearmanr(s,s_def)[0]),'top20':len(set(np.argsort(s)[:20])&set(np.argsort(s_def)[:20])),'top1':CM[int(np.argmin(s))]['name']}
# 4 tradeoff
ALL=CM+REF; XA=matrix(ALL,ids); nA=[c['name'] for c in ALL]; tr=[]
for f in (0,0.25,0.5,1,2,4,8):
    s=XA@eff(p,ids,{**w0,'B14':w0['B14']*f}); o=np.argsort(s)
    tr.append({'B14':w0['B14']*f,'top1':nA[o[0]],'ranks':{x:int(np.where(o==nA.index(x))[0][0])+1 for x in ('graphite','sturdy','semimak')}})
OUT['tradeoff_b14']=tr
OUT['sfb_stretch']=[{'name':c['name'],'B01':c['raw']['B01'],'B14':c['raw']['B14'],'score':c['score']} for c in ALL if c['score']<30]
# 5 engram pair-level
import io, contextlib
with contextlib.redirect_stdout(io.StringIO()): exec(open('a5_engram.py').read())
gc=np.array([g(i,j,bi) for i,j,_,_ in P]); s=np.array([x[2] for x in P])
OUT['engram_pairs']={'n':len(P),'rho_gecko':float(spearmanr(s,-gc)[0]),'per_metric':{m:float(spearmanr(s,np.array([tb['M01']['v'][i]+tb['M01']['v'][j] if m=='M01' else tb[m]['v'][i*N+j] for i,j,_,_ in P]))[0]) for m in ['B01','B05','B09','B10','B11','B14','M01']}}
json.dump(OUT,open('data/results_part1.json','w'),indent=1)
print('part1 ok')
