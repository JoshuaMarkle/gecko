import sys; sys.path.insert(0,'.'); from lib import *
p=profile(); w0=p['weights']
R=load('data/refs.jsonl'); C=load('data/cmini.jsonl'); SW=load('data/sweep.jsonl')
opt=[r for r in SW if r['id']=='default'][0]
print('default optimum', opt['chars'], round(opt['score'],3))
# rank of optimum on each raw metric among refs + top cmini (score<30)
good=[c for c in C if c['score']<26]
print('good cmini layouts (score<26):',len(good))
ids=sorted(opt['raw'])
from metrics_dir import DIR
print(f"{'id':4} {'w':>5} {'opt':>8} {'refs med':>8} {'good med':>8} {'pct among good (higher=worse)':>10}")
for i in ids:
    d=DIR.get(i)
    if d is None or d=='info': continue
    v=opt['raw'][i]; g=np.array([c['raw'][i] for c in good]); r=np.array([c['raw'][i] for c in R if c['name']!='qwerty'])
    if np.std(g)==0 and v==g.mean(): continue
    worse = np.mean(g<v) if d=='min' else np.mean(g>v)
    flag=' <-- worse than 80% of good layouts' if worse>0.8 else ''
    print(f"{i:4} {w0.get(i,0):5.2f} {v:8.3f} {np.median(r):8.3f} {np.median(g):8.3f} {worse*100:8.0f}%{flag}")
