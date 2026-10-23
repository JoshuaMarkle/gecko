import sys, csv; sys.path.insert(0,'.'); from lib import *
from scipy.stats import spearmanr, pearsonr
T=json.load(open('data/tables_ansi30.json')); S=T['slots']; tb=T['tables']; N=30
q="qwertyuiopasdfghjkl;zxcvbnm,./"; slot={c:i for i,c in enumerate(q)}
p=profile(); w=p['weights']; sc=p['scale']
def tv(m,i,j): return tb[m]['v'][i*N+j]
var=sys.argv[1] if len(sys.argv)>1 else 'clean'
rows=[r for r in csv.DictReader(open('/home/josh/projects/gecko-workspace/aalto-analysis/out/bigrams.csv')) if r['variant']==var]
D=[]
for r in rows:
    a,b=r['bigram'][0].lower(),r['bigram'][1].lower()
    if a==b or a not in slot or b not in slot: continue
    i,j=slot[a],slot[b]; n=int(r['n'])
    if n<200: continue
    D.append((a+b,i,j,n,float(r['mean_ms'])))
print(var,'bigrams',len(D),'keystroke pairs',sum(d[3] for d in D))
bi=[m for m in tb if tb[m]['order']=='bi' and tb[m]['dir']!='info']
def gcost(i,j,ms=bi):
    c=sum((w.get(m,0)/sc.get(m,1))*(1 if tb[m]['dir']=='min' else -1)*tv(m,i,j) for m in ms)
    return c+ w['M01']/sc['M01']*tb['M01']['v'][j]
iki=np.array([d[4] for d in D]); ln=np.log([d[3] for d in D]); n=np.array([d[3] for d in D],float)
g=np.array([gcost(d[1],d[2]) for d in D])
hand=lambda i:S[i]['finger']<5
alt=np.array([hand(d[1])!=hand(d[2]) for d in D])
print(f"Spearman IKI vs GECKO bigram cost: {spearmanr(iki,g)[0]:+.3f}  (same-hand only: {spearmanr(iki[~alt],g[~alt])[0]:+.3f})")
print(f"Spearman IKI vs log frequency: {spearmanr(iki,ln)[0]:+.3f}")
def ols(Xc,y,wt):
    X=np.column_stack([np.ones(len(y))]+Xc); W=np.sqrt(wt)
    b,*_=np.linalg.lstsq(X*W[:,None],y*W,rcond=None); res=y-X@b
    r2=1-np.sum(wt*res**2)/np.sum(wt*(y-np.average(y,weights=wt))**2); return b,r2
wt=np.minimum(n,np.percentile(n,90))
for name,Xc in [('logfreq',[ln]),('logfreq+gecko',[ln,g]),('gecko',[g])]:
    b,r2=ols(Xc,iki,wt); print(f"  WLS {name:14} R2 {r2:.3f} coefs {np.round(b,2)}")
# partial correlation controlling for log freq
from numpy.linalg import lstsq
def resid(y,x): X=np.column_stack([np.ones(len(x)),x]); return y-X@lstsq(X,y,rcond=None)[0]
print(f"partial Spearman(IKI, gecko | logfreq): {spearmanr(resid(iki,ln),resid(g,ln))[0]:+.3f}")
# per-metric partial correlations
print("per-metric partial Spearman with IKI | logfreq (table value at the pair):")
for m in bi+['M01']:
    x=np.array([tb[m]['v'][d[2]] if m=='M01' else tv(m,d[1],d[2]) for d in D])
    if np.std(x)==0: continue
    print(f"   {m}: {spearmanr(resid(iki,ln),resid(x,ln))[0]:+.3f}  (nonzero on {np.mean(x!=0)*100:.0f}% of pairs)")
# geometric model: ms per class
F={}
F['sfb']=np.array([tv('B01',d[1],d[2])>0 for d in D],float)
F['alt']=alt.astype(float)
F['stretch']=np.array([tv('B14',d[1],d[2]) for d in D])
F['move']=np.array([tv('B11',d[1],d[2]) for d in D])
F['rowjump']=np.array([tv('B10',d[1],d[2]) for d in D])
F['roll_in']=np.array([tv('B07',d[1],d[2])>0 for d in D],float)
F['effort2']=np.array([tb['M01']['v'][d[2]] for d in D])
F['sfb_dist']=np.array([tv('B03',d[1],d[2]) for d in D])
F['e05']=np.array([tv('E05',d[1],d[2]) for d in D])
keys=list(F)
b,r2=ols([ln]+[F[k] for k in keys],iki,wt)
print(f"geometric WLS R2 {r2:.3f}; ms per unit (controlling log freq):")
for k,c in zip(['const','logfreq']+keys,b): print(f"   {k:9} {c:+8.2f}   (feature sd {np.std(F[k]) if k in F else 0:.2f})")
# leave-one-first-letter-out CV
letters=sorted(set(d[0][0] for d in D)); pred={}
for nm,cols in [('logfreq',[ln]),('logfreq+gecko',[ln,g]),('logfreq+geom',[ln]+[F[k] for k in keys])]:
    yh=np.zeros(len(D))
    for L in letters:
        te=np.array([d[0][0]==L for d in D]); tr=~te
        X=np.column_stack([np.ones(len(D))]+cols); W=np.sqrt(wt[tr])
        bb,*_=np.linalg.lstsq(X[tr]*W[:,None],iki[tr]*W,rcond=None); yh[te]=X[te]@bb
    r2=1-np.sum(wt*(iki-yh)**2)/np.sum(wt*(iki-np.average(iki,weights=wt))**2)
    print(f"  CV (leave first letter out) {nm:14} R2 {r2:.3f}")
json.dump({'pairs':[d[0] for d in D],'iki':iki.tolist(),'g':g.tolist(),'ln':ln.tolist()},open('data/iki_pairs.json','w'))
