import sys, io, contextlib, json, collections; sys.path.insert(0,'.'); from lib import *
from scipy.stats import spearmanr
OUT={}
def run(script, argv, upto=None):
    ns={'__name__':'x'}; sys.argv=['x']+argv
    src=open(script).read()
    if upto: src=src.split(upto)[0]
    with contextlib.redirect_stdout(io.StringIO()) as f: exec(src, ns)
    return ns, f.getvalue()
# timing, bigram level
T={}
for gname in ('touch','touch_fast','few'):
    ns,txt=run('a3b_iki_groups.py',[gname,'median_rel'])
    D,iki,g,ln,resid,cv,X,FK=ns['D'],ns['iki'],ns['g'],ns['ln'],ns['resid'],ns['cv'],ns['X'],ns['FK']
    ri,rg=resid(iki,ln),resid(g,ln)
    o=np.argsort(rg); bins=np.array_split(o,10)
    T[gname]={'n_bigrams':len(D),'n_pairs':int(sum(d[3] for d in D)),
      'rho':float(spearmanr(iki,g)[0]),'partial':float(spearmanr(ri,rg)[0]),
      'cv':{'logfreq':float(cv([ln])),'gecko':float(cv([ln,g])),'geom':float(cv([ln]+list(X.T)))},
      'bins':[[float(rg[b].mean()),float(ri[b].mean()),float(ri[b].std()/np.sqrt(len(b)))] for b in bins],
      'per_metric':{m:float(spearmanr(ri,resid(np.array([ns['tv'](m,d[1],d[2]) for d in D]),ln))[0]) for m in ('B01','B07','B08','B10','B11','B14','E05')},
      'coef':dict(zip(['const','logfreq']+FK,[float(x) for x in json.load(open(f'data/timemodel_{gname}.json'))['b']]))}
OUT['timing']=T
import csv
OUT['participants']=eval(open('/home/josh/projects/gecko-workspace/aalto-analysis/out2/groups.txt').read())
# trigram effects with CIs
TR={}
for gname in ('touch','touch_fast'):
    ns,_=run('a12_trigrams.py',[gname])
    B=ns['B']; b=ns['b']*100
    TR[gname]={nm:[float(b[j]),float(np.percentile(B[:,j],2.5)),float(np.percentile(B[:,j],97.5))] for j,nm in enumerate(ns['names']) if j>=3}
    TR[gname]['n']=len(ns['D'])
OUT['trigrams']=TR
# time judge
ns,_=run('a10_timejudge.py',['touch_fast_rel'],upto="sets={")
tj=ns['tjudge']
REF=load('data/refs.jsonl'); CM=load('data/cmini.jsonl'); RA=load('data/random.jsonl'); SW=load('data/sweep.jsonl')
good=[c for c in CM if c['score']<22]
OUT['timejudge']={k:float(spearmanr([r['score'] for r in R],[tj(r['chars']) for r in R])[0]) for k,R in (('refs',REF),('cmini',CM),('random',RA),('good',good))}
ct=np.array([tj(c['chars']) for c in CM])
OUT['timejudge_refs']={r['name']:{'time':tj(r['chars']),'pct':float(np.mean(ct<tj(r['chars']))),'score':r['score']} for r in REF}
d0=[r for r in SW if r['id']=='default'][0]
OUT['timejudge_refs']['gecko-opt']={'time':tj(d0['chars']),'pct':float(np.mean(ct<tj(d0['chars']))),'score':d0['score']}
ns,txt=run('a11_fit_time.py',[])
OUT['timefit']={'rho':float(spearmanr(ns['pred'],ns['y'])[0]),'share':{k:float(v/ns['tot']) for k,v in zip(ns['cand'],ns['wfit'])},'default_share':{k:float(ns['w0n'][k]) for k in ns['cand']}}
# corpus regret
names=json.load(open('data/corpus_opt_names.json')); C=['english','shai','reddit','mt-450k']; M={}
for c in C:
    for l in open(f'data/xcorpus_{c}.jsonl'):
        d=json.loads(l); M[(names.get(d['name'],d['name']),c)]=d['score']
for c in C: M[('corpus:english',c)]=M[('corpus:english-cased',c)]
OUT['corpus_regret']={r:{c:M[(('corpus:'+r) if r in C else r,c)]-M[('corpus:'+c,c)] for c in C} for r in C+['graphite','sturdy']}
json.dump(OUT,open('data/results_part2.json','w'),indent=1); print('part2 ok')
