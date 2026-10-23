"""IKI by typist group vs GECKO pair costs; geometric time model; time judge for layouts."""
import sys, csv, re, glob, collections; sys.path.insert(0,'.'); from lib import *
from scipy.stats import spearmanr
T=json.load(open('data/tables_ansi30.json')); S=T['slots']; tb=T['tables']; N=30
q="qwertyuiopasdfghjkl;zxcvbnm,./"; slot={c:i for i,c in enumerate(q)}
p=profile(); w=p['weights']; sc=p['scale']
F=lambda i:S[i]['finger']; hand=lambda i:F(i)<5
def tv(m,i,j): return tb[m]['v'][i*N+j]
bi=[m for m in tb if tb[m]['order']=='bi' and tb[m]['dir']!='info']
def gcost(i,j):
    return sum((w.get(m,0)/sc.get(m,1))*(1 if tb[m]['dir']=='min' else -1)*tv(m,i,j) for m in bi)+w['M01']/sc['M01']*tb['M01']['v'][j]
def feats(i,j):
    fi,fj=F(i),F(j); sh=hand(i)==hand(j)
    dy=S[j]['row']-S[i]['row']; dx=abs(S[j]['x']-S[i]['x'])
    sf=fi==fj
    inward = sh and not sf and ((hand(i) and fj>fi) or (not hand(i) and fj<fi))
    adj = sh and not sf and abs(fi-fj)==1
    return {
      'sfb': float(sf),
      'sfb_dist': float(sf)*np.hypot(dx,dy),
      'samehand_diff': float(sh and not sf),
      'inward': float(inward),
      'adjacent': float(adj),
      'rows_samehand': float(sh and not sf)*abs(dy),
      'scissor2': float(adj and abs(dy)>=2),
      'pinky_ring': float(sh and {fi,fj} in ({0,1},{8,9})),
      'effort_b': tb['M01']['v'][j],
      'pinky_b': float(F(j) in (0,9)),
      'center_b': float(S[j]['col'] in (4,5)),
      'bottom_b': float(S[j]['row']==2), 'top_b': float(S[j]['row']==0),
    }
FK=list(feats(0,1))
if len(sys.argv)>3 and sys.argv[3]=='rel': FK=['sfb','sfb_dist','samehand_diff','inward','adjacent','rows_samehand','scissor2','pinky_ring','effort_b']
group=sys.argv[1]; col=sys.argv[2] if len(sys.argv)>2 else 'median_rel'
rows=[r for r in csv.DictReader(open('/home/josh/projects/gecko-workspace/aalto-analysis/out2/bigrams_groups.csv')) if r['group']==group]
D=[]
for r in rows:
    a,b=r['bigram']
    if a==b or a not in slot or b not in slot or int(r['n'])<300: continue
    D.append((r['bigram'],slot[a],slot[b],int(r['n']),float(r[col])))
iki=np.array([d[4] for d in D]); n=np.array([d[3] for d in D],float); ln=np.log(n)
g=np.array([gcost(d[1],d[2]) for d in D]); X=np.array([[feats(d[1],d[2])[k] for k in FK] for d in D])
print(f"group {group} ({col}): {len(D)} bigrams, {int(n.sum())} keystroke pairs")
print(f"  Spearman(time, GECKO pair cost) {spearmanr(iki,g)[0]:+.3f}; (time, log freq) {spearmanr(iki,ln)[0]:+.3f}")
def resid(y,x): A=np.column_stack([np.ones(len(x)),x]); return y-A@np.linalg.lstsq(A,y,rcond=None)[0]
print(f"  partial Spearman(time, GECKO | log freq) {spearmanr(resid(iki,ln),resid(g,ln))[0]:+.3f}")
print("  per-metric partial Spearman | log freq:", ', '.join(f"{m} {spearmanr(resid(iki,ln),resid(np.array([tv(m,d[1],d[2]) for d in D]),ln))[0]:+.2f}" for m in bi if np.std([tv(m,d[1],d[2]) for d in D])>0))
wt=np.minimum(n,np.percentile(n,90))
def fit(cols,y,wt):
    A=np.column_stack([np.ones(len(y))]+cols); W=np.sqrt(wt); b,*_=np.linalg.lstsq(A*W[:,None],y*W,rcond=None); return b
def r2(y,yh,wt): return 1-np.sum(wt*(y-yh)**2)/np.sum(wt*(y-np.average(y,weights=wt))**2)
# CV by held-out first letter
def cv(cols):
    yh=np.zeros(len(D)); A=np.column_stack([np.ones(len(D))]+cols)
    for L in set(d[0][0] for d in D):
        te=np.array([d[0][0]==L for d in D]); W=np.sqrt(wt[~te])
        b,*_=np.linalg.lstsq(A[~te]*W[:,None],iki[~te]*W,rcond=None); yh[te]=A[te]@b
    return r2(iki,yh,wt)
print(f"  CV R2: logfreq {cv([ln]):.3f} | +GECKO {cv([ln,g]):.3f} | +geometric model {cv([ln]+list(X.T)):.3f} | geometric only {cv(list(X.T)):.3f}")
b=fit([ln]+list(X.T),iki,wt)
print("  geometric model (controlling log freq), effect in units of", col)
for k,c in zip(['const','logfreq']+FK,b): print(f"     {k:14} {c:+.4f}")
json.dump({'FK':FK,'b':b.tolist(),'group':group,'col':col},open(f'data/timemodel_{group}'+('_rel' if len(sys.argv)>3 else '')+'.json','w'))
