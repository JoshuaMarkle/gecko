"""Skipgram (SFS) timing in detail: by context, finger, distance, outcome measure and typist group.
Data: Aalto 136M trigram aggregates (out2/trigrams_groups.csv: mean log relative interval of the
3rd keystroke, and of the a→c span). Controls: log trigram frequency, log median time of bigram b→c."""
import sys, csv; sys.path.insert(0,'.'); from lib import *
T=json.load(open('data/tables_ansi30.json')); S=T['slots']
q="qwertyuiopasdfghjkl;zxcvbnm,./"; slot={c:i for i,c in enumerate(q)}
F=lambda c:S[slot[c]]['finger']; H=lambda c:F(c)<5; R=lambda c:S[slot[c]]['row']
def cls(a,b,c):
    if len({a,b,c})<3 or F(a)==F(b) or F(b)==F(c): return None
    ha,hb,hc=H(a),H(b),H(c)
    if ha==hb==hc:
        fa,fb,fc=F(a),F(b),F(c)
        if (fa<fb<fc) or (fa>fb>fc): return 'onehand'
        return 'redirect' if {fa,fb,fc}&{3,6} else 'bad_redirect'
    if ha!=hb and hb!=hc: return 'alternate'
    return 'roll'
K=['alternate','roll','onehand','redirect','bad_redirect']
out={}
for group in ['touch','touch_fast']:
    big={r['bigram']:float(r['median_rel']) for r in csv.DictReader(open('/home/josh/projects/gecko-workspace/aalto-analysis/out2/bigrams_groups.csv')) if r['group']==group}
    D=[]
    for r in csv.DictReader(open('/home/josh/projects/gecko-workspace/aalto-analysis/out2/trigrams_groups.csv')):
        if r['group']!=group or int(r['n'])<200: continue
        a,b,c=r['trigram']
        if a not in slot or b not in slot or c not in slot or b+c not in big: continue
        k=cls(a,b,c)
        if k is None: continue
        sfs=F(a)==F(c) and a!=c
        D.append(dict(k=k,sfs=sfs,n=int(r['n']),y3=float(r['mean_logrel2']),ys=float(r['mean_logrel_span']),bc=np.log(big[b+c]),
                      ab=np.log(big[a+b]) if a+b in big else np.nan,
                      mid_same=H(b)==H(a), fing=min(F(a),9-F(a)) if sfs else -1, rows=abs(R(a)-R(c)) if sfs else 0))
    D=[d for d in D if np.isfinite(d['ab'])]
    n=np.array([d['n'] for d in D],float); wt=np.minimum(n,np.percentile(n,90)); W=np.sqrt(wt); ln=np.log(n)
    def fit(cols, names, yk, boot=1000):
        y=np.array([d[yk] for d in D]); A=np.column_stack([np.ones(len(D)),ln]+cols)
        b,*_=np.linalg.lstsq(A*W[:,None],y*W,rcond=None)
        rng=np.random.default_rng(0); B=[]
        for _ in range(boot):
            i=rng.integers(0,len(D),len(D)); Wi=np.sqrt(wt[i]); bb,*_=np.linalg.lstsq(A[i]*Wi[:,None],y[i]*Wi,rcond=None); B.append(bb)
        B=np.array(B)*100
        return {nm:(round(b[j+2]*100,2),round(np.percentile(B[:,j+2],2.5),2),round(np.percentile(B[:,j+2],97.5),2)) for j,nm in enumerate(names)}
    cls_cols=[np.array([d['k']==k for d in D],float) for k in K[1:]]
    bc=np.array([d['bc'] for d in D]); ab=np.array([d['ab'] for d in D])
    sfs=np.array([d['sfs'] for d in D],float)
    res={'trigrams':len(D),'sfs':int(sfs.sum())}
    # 1. overall, 3rd keystroke (as in the validity study) and a→c span (controls both bigrams)
    res['third']=fit([bc,sfs]+cls_cols,['bc','SFS']+K[1:],'y3')['SFS']
    res['span']=fit([bc,ab,sfs]+cls_cols,['bc','ab','SFS']+K[1:],'ys')['SFS']
    # 2. by context: middle key on the other hand (alternation, T02) vs same hand
    alt=np.array([d['sfs'] and not d['mid_same'] for d in D],float); same=np.array([d['sfs'] and d['mid_same'] for d in D],float)
    r=fit([bc,alt,same]+cls_cols,['bc','alt','same']+K[1:],'y3'); res['ctx_alt']=r['alt']; res['ctx_same']=r['same']
    res['n_alt']=int(alt.sum()); res['n_same']=int(same.sum())
    # 3. by finger: pinky/ring vs middle/index (fing: 0 pinky 1 ring 2 middle 3 index)
    weak=np.array([d['sfs'] and d['fing']<=1 for d in D],float); strong=np.array([d['sfs'] and d['fing']>=2 for d in D],float)
    r=fit([bc,weak,strong]+cls_cols,['bc','weak','strong']+K[1:],'y3'); res['pinky_ring']=r['weak']; res['middle_index']=r['strong']
    res['n_pinky_ring']=int(weak.sum()); res['n_middle_index']=int(strong.sum())
    # 4. by distance: same row vs row change between a and c
    r0=np.array([d['sfs'] and d['rows']==0 for d in D],float); r1=np.array([d['sfs'] and d['rows']>=1 for d in D],float)
    r=fit([bc,r0,r1]+cls_cols,['bc','r0','r1']+K[1:],'y3'); res['same_row']=r['r0']; res['row_change']=r['r1']
    res['n_same_row']=int(r0.sum()); res['n_row_change']=int(r1.sum())
    out[group]=res
    print(group, json.dumps(res))
json.dump(out,open('data/skipgram_timing.json','w'),indent=1)
