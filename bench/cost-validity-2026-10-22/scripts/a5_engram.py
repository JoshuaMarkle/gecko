import sys,csv; sys.path.insert(0,'.'); from lib import *
from scipy.stats import spearmanr
T=json.load(open('data/tables_ansi30.json')); S=T['slots']; tb=T['tables']; N=30
p=profile(); w=p['weights']; sc=p['scale']
idx={(s['row'],s['col']):i for i,s in enumerate(S)}
rows=[r for r in csv.DictReader(l for l in open('/home/josh/projects/gecko/data/comfort/engram-bt.csv') if not l.startswith('#')) if r['measured']=='1']
bi=[m for m in tb if tb[m]['order']=='bi' and tb[m]['dir']!='info' and m not in ('E04','E05','Y01','Y04')]
P=[]
for r in rows:
    i=idx[(int(r['a_row']),int(r['a_col']))]; j=idx[(int(r['b_row']),int(r['b_col']))]
    if i==j: continue
    P.append((i,j,float(r['bt_strength']),int(r['n'])))
print('measured distinct-key bigrams',len(P))
s=np.array([x[2] for x in P])
def g(i,j,ms):
    return sum((w.get(m,0)/sc.get(m,1))*(1 if tb[m]['dir']=='min' else -1)*tb[m]['v'][i*N+j] for m in ms)+w['M01']/sc['M01']*(tb['M01']['v'][i]+tb['M01']['v'][j])
gc=np.array([g(i,j,bi) for i,j,_,_ in P])
print(f"Spearman(BT comfort, -GECKO non-Engram pair cost) = {spearmanr(s,-gc)[0]:+.3f}")
for m in bi+['M01']:
    x=np.array([tb['M01']['v'][i]+tb['M01']['v'][j] if m=='M01' else tb[m]['v'][i*N+j] for i,j,_,_ in P])
    if np.std(x)>0: print(f"  {m} ({'+' if tb[m]['dir']=='max' else '-'}) Spearman with comfort {spearmanr(s,x)[0]:+.3f}  nonzero {np.mean(x!=0)*100:.0f}%")
sf=np.array([tb['B01']['v'][i*N+j]>0 for i,j,_,_ in P])
print(f"  mean BT strength: SFB {s[sf].mean():+.2f} (n={sf.sum()}), non-SFB {s[~sf].mean():+.2f}")
# pairwise accuracy: how often does GECKO pair cost order two measured bigrams like the BT strengths?
a=t=0
for x in range(len(P)):
    for y in range(x+1,len(P)):
        if abs(s[x]-s[y])<0.3: continue
        t+=1; a+= (s[x]>s[y])==(gc[x]<gc[y])
print(f"pairwise agreement (|Δs|>0.3): {a/t:.3f} of {t}")
