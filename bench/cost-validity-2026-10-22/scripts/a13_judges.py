import sys; sys.path.insert(0,'.'); from lib import *
exec(open('a10_timejudge.py').read().split("sets={")[0].replace("group=sys.argv[1]","group='touch_fast_rel'"))
p=profile(); ids=None
PO=load('data/profile_opt.jsonl'); R=load('data/refs.jsonl'); C=load('data/cmini.jsonl')
best={}
for r in PO:
    if r['id'] not in best or r['opt_score']<best[r['id']]['opt_score']: best[r['id']]=r
rows=[(k.replace('profile:','opt:'),v) for k,v in best.items()]+[(n,[r for r in R if r['name']==n][0]) for n in ('graphite','sturdy','canary','colemak-dh','qwerty')]
ids=ids_of(R); e=eff(p,ids)
allc_t=np.array([tjudge(c['chars']) for c in C])
print(f"{'layout':16} {'default':>8} {'time':>7} {'t pct':>6} {'SFB%':>6} {'SFS%':>6} {'E05':>6} {'E04':>6} {'B14':>6} {'home%':>6} {'effort':>6} {'G01':>6}  chars")
for nm,r in rows:
    d=float(np.dot([r['u'][i] for i in ids],e)); t=tjudge(r['chars']); x=r['raw']
    print(f"{nm:16} {d:8.2f} {t:7.4f} {np.mean(allc_t<t)*100:5.0f}% {x['B01']:6.2f} {x['S01']:6.2f} {x['E05']:6.2f} {x['E04']:6.2f} {x['B14']:6.2f} {x['M05']:6.1f} {x['M01']:6.3f} {x['G01']:6.3f}  {r['chars']}")
