import sys; sys.path.insert(0,'.'); from lib import *
p=profile(); w0=p['weights']
R=load('data/refs.jsonl'); ids=ids_of(R); X=matrix(R,ids); nm=[r['name'] for r in R]
modern=[nm.index(x) for x in ('graphite','sturdy','semimak','canary')]
classic=[nm.index(x) for x in ('dvorak','colemak','colemak-dh','workman','halmak')]
q=nm.index('qwerty'); rng=np.random.default_rng(3)
for sig in (0.5,1.0,2.0):
    a=b=0;N=4000
    for _ in range(N):
        w={k:v*np.exp(rng.normal(0,sig)) for k,v in w0.items()}; s=X@eff(p,ids,w)
        a+= s[modern].max()<s[classic].min(); b+= s[classic].max()<s[q]
    print(f"sigma {sig}: P(every modern beats every classic) {a/N:.2f}; P(every classic beats QWERTY) {b/N:.2f}")
# uniform random weights on the simplex (no default prior at all)
a=b=0;N=4000
for _ in range(N):
    w={k:rng.exponential()*p['scale'].get(k,1) for k in w0}; s=X@eff(p,ids,w)
    a+= s[modern].max()<s[classic].min(); b+= s[classic].max()<s[q]
print(f"prior-free (each metric equal expected spread weight): modern>classic {a/N:.2f}; classic>QWERTY {b/N:.2f}")
