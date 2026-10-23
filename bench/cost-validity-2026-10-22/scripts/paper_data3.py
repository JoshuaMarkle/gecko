import sys, io, contextlib, json, collections; sys.path.insert(0,'.'); from lib import *
OUT={}
ns={'__name__':'x'}; sys.argv=['x','data/sweep.jsonl']
with contextlib.redirect_stdout(io.StringIO()): exec(open('a7_sweep.py').read(), ns)
best,ham,fdist,dscore,c0,d0,W=ns['best'],ns['ham'],ns['fdist'],ns['dscore'],ns['c0'],ns['d0'],ns['W']
ns2={'__name__':'x'}; sys.argv=['x','touch_fast_rel']
with contextlib.redirect_stdout(io.StringIO()): exec(open('a10_timejudge.py').read().split("sets={")[0], ns2)
tj=ns2['tjudge']; t0=tj(c0)
p=profile(); w0=p['weights']
OUT['seed_agree']=dict(ns['agree'])
OUT['default_opt']={'chars':c0,'score':dscore(d0),'raw':{k:d0['raw'][k] for k in ('B01','S01','B14','E05','M01','M05','T03','T04','T06')}}
oat={}
for k in w0:
    oat[k]={}
    for tag in ('drop','half','double'):
        r=best.get(f'{tag}:{k}')
        if r: oat[k][tag]={'slots':ham(r['chars'],c0),'fingers':fdist(r['chars'],c0),'regret':dscore(r)-dscore(d0),'dtime':(tj(r['chars'])-t0)/t0*100,'B01':r['raw']['B01'],'B14':r['raw']['B14']}
OUT['oat']=oat
mc={}
for s in ('mc0.5','mc1.0'):
    R=[(k,best[k]) for k in best if k.startswith(s+':')]
    if not R: continue
    mc[s]={'n':len(R),'distinct':len(set(r['chars'] for _,r in R)),
      'points':[{'slots':ham(r['chars'],c0),'fingers':fdist(r['chars'],c0),'regret':dscore(r)-dscore(d0),'dtime':(tj(r['chars'])-t0)/t0*100,'B01':r['raw']['B01'],'B14':r['raw']['B14']} for _,r in R]}
OUT['mc']=mc
# profile optima vs judges
PO=load('data/profile_opt.jsonl') if __import__('os').path.exists('data/profile_opt.jsonl') else []
pb={}
for r in PO:
    if r['id'] not in pb or r['opt_score']<pb[r['id']]['opt_score']: pb[r['id']]=r
REF=load('data/refs.jsonl'); CM=load('data/cmini.jsonl'); ct=np.array([tj(c['chars']) for c in CM])
rows=[(k.replace('profile:',''),v) for k,v in pb.items()]+[(n,[r for r in REF if r['name']==n][0]) for n in ('graphite','sturdy','qwerty')]
OUT['judges']=[{'name':nm,'chars':r['chars'],'default':dscore(r),'time_pct':float(np.mean(ct<tj(r['chars']))),'dtime':(tj(r['chars'])-t0)/t0*100,
   **{k:r['raw'][k] for k in ('B01','S01','B14','E05','E04','M05','M01','G01')}} for nm,r in rows]
json.dump(OUT,open('data/results_part3.json','w'),indent=1); print('part3 ok', {k:v['n'] for k,v in mc.items()}, len(pb),'profiles')
