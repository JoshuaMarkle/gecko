import sys, collections; sys.path.insert(0,'.'); from lib import *
from metrics_dir import DIR
p=profile(); w0=p['weights']; e0=None
SW=load(sys.argv[1] if len(sys.argv)>1 else 'data/sweep.jsonl')
W={json.loads(l)['id']:json.loads(l)['weights'] for l in open('data/w_sweep.jsonl')}
ids=ids_of(SW); e0=eff(p,ids)
best={}; agree=collections.Counter()
by=collections.defaultdict(list)
for r in SW: by[r['id']].append(r)
for k,rs in by.items():
    rs.sort(key=lambda r:r['opt_score']); best[k]=rs[0]
    if len(rs)>1: agree[rs[0]['chars']==rs[1]['chars']]+=1
print('vectors',len(best),'seed agreement',dict(agree))
d0=best['default']; c0=d0['chars']
q="qwertyuiopasdfghjkl;zxcvbnm,./"
FING=[0,1,2,3,3,6,6,7,8,9]*3
def fingmap(ch): return {c:FING[i] for i,c in enumerate(ch)}
def ham(a,b):  # slot differences, mirror-invariant (left/right flip)
    m=lambda s:''.join(s[r*10:(r+1)*10][::-1] for r in range(3))
    return min(sum(x!=y for x,y in zip(a,b)), sum(x!=y for x,y in zip(m(a),b)))
def fdist(a,b):  # letters on a different finger, mirror-invariant
    fa,fb=fingmap(a),fingmap(b); mir={0:9,1:8,2:7,3:6,6:3,7:2,8:1,9:0}
    return min(sum(fa[c]!=fb[c] for c in fa), sum(mir[fa[c]]!=fb[c] for c in fa))
def dscore(r): return float(np.dot([r['u'][i] for i in ids],e0))
print('default optimum',c0,round(dscore(d0),3))
print("\n== one-at-a-time ==")
print(f"{'metric':6} {'drop: Δslots Δfing regret':>28} {'half':>18} {'double':>18}   key metric changes when dropped")
for k in w0:
    row=f"{k:6}"
    for tag in ('drop','half','double'):
        r=best.get(f'{tag}:{k}')
        if r is None: row+=' '*18; continue
        row+=f"  {ham(r['chars'],c0):3d} {fdist(r['chars'],c0):3d} {dscore(r)-dscore(d0):6.2f}"
    r=best.get(f'drop:{k}')
    if r:
        ch=[]
        for m in ('B01','B14','S01','E05','M01','M05','T03','T04','B11','T06'):
            a,b=d0['raw'][m],r['raw'][m]
            if abs(b-a)>0.1*abs(a)+1e-9: ch.append(f"{m} {a:.2f}->{b:.2f}")
        row+='   '+'; '.join(ch[:5])
    print(row)
for s in ('mc0.5','mc1.0'):
    R=[best[k] for k in best if k.startswith(s+':')]
    if not R: continue
    hs=np.array([ham(r['chars'],c0) for r in R]); fd=np.array([fdist(r['chars'],c0) for r in R]); rg=np.array([dscore(r)-dscore(d0) for r in R])
    uniq=len(set(r['chars'] for r in R))
    print(f"\n== {s} (n={len(R)}) distinct optima {uniq}; slots differing from default: median {np.median(hs):.0f} (IQR {np.percentile(hs,25):.0f}-{np.percentile(hs,75):.0f}); letters on another finger: median {np.median(fd):.0f}; regret under default: median {np.median(rg):.2f}, p90 {np.percentile(rg,90):.2f}")
    # sensitivity: correlate log weight ratio with finger distance and regret
    L=np.array([[np.log(max(W[k][m],1e-12)/w0[m]) for m in w0] for k in best if k.startswith(s+':')])
    from scipy.stats import spearmanr
    print('   Spearman(log weight ratio, letters-on-other-finger):', ', '.join(f"{m} {spearmanr(L[:,j],fd)[0]:+.2f}" for j,m in enumerate(w0) if abs(spearmanr(L[:,j],fd)[0])>0.15))
    # per-letter finger stability
    stab={}
    for c in c0:
        f0=fingmap(c0)[c]; mir={0:9,1:8,2:7,3:6,6:3,7:2,8:1,9:0}
        same=0
        for r in R:
            fm=fingmap(r['chars']); 
            # orient each optimum to the default by choosing mirror with smaller fdist
            flip = fdist(r['chars'],c0)!=sum(fm[x]!=fingmap(c0)[x] for x in fm)
            g=mir[fm[c]] if flip else fm[c]
            same+= g==f0
        stab[c]=same/len(R)
    print('   P(letter keeps its default finger):', ' '.join(f"{c}{v:.2f}" for c,v in sorted(stab.items(),key=lambda t:-t[1]) if c.isalpha()))
