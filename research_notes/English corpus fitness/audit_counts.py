"""Read bundled English GKC tables only; stdlib, single thread, no optimization."""
from pathlib import Path
import struct, json, hashlib, itertools
root=Path('/home/josh/projects/gecko')
names=['english','english-cased','english-full','english-full-bspc','shai','reddit','mt-10k','mt-450k']
parsed={}; result={}
for name in names:
 p=root/'data/corpora'/f'{name}.gkc'; b=p.read_bytes(); off=8; strings=[]
 for _ in range(2):
  n=struct.unpack_from('<I',b,off)[0];off+=4;strings.append(b[off:off+n].decode());off+=n
 c=len(strings[1]); tables={}
 specs=[('mono',c),('bi',c*c),('skip',c*c),('skip2',c*c),('skip3',c*c),('tri',c**3)]
 if b.startswith(b'GKCORP3'):specs += [('shift',c),('shift_bi',c*c)]
 for key,n in specs:tables[key]=struct.unpack_from(f'<{n}Q',b,off);off+=8*n
 assert off==len(b)
 cs=strings[1]; m=tables['mono']; tot=sum(m)
 result[name]={'sha256':hashlib.sha256(b).hexdigest(),'magic':b[:7].decode(),'charset':cs,'file_bytes':len(b),'table_totals':{k:sum(v) for k,v in tables.items()},'nonzero':{k:sum(x>0 for x in v) for k,v in tables.items()},'mono_pct':{x:round(v/tot*100,6) for x,v in zip(cs,m)},'letters_pct':sum(v for x,v in zip(cs,m) if x.isascii() and x.isalpha())/tot*100,'shift_per_100_tokens':sum(tables.get('shift',()))/tot*100}
 parsed[name]=(cs,tables)
# On common 26 letters, separately normalize each order; TV is distribution difference, not score difference.
tvs={}
for a,b in itertools.combinations(names,2):
 if 'full' in a or 'full' in b:continue
 row={}
 for key,order in [('mono',1),('bi',2),('tri',3),('skip',2)]:
  vectors=[]
  for name in [a,b]:
   cs,t=parsed[name]; inds=[cs.index(x) for x in 'abcdefghijklmnopqrstuvwxyz']; c=len(cs)
   v=[t[key][sum(i*c**(order-j-1) for j,i in enumerate(ix))] for ix in itertools.product(inds,repeat=order)]
   s=sum(v); vectors.append([x/s for x in v])
  row[key]=sum(abs(x-y) for x,y in zip(*vectors))/2
 tvs[a+' vs '+b]=row
result['letter_only_total_variation']=tvs
(root/'research_notes/English corpus fitness/counts.json').write_text(json.dumps(result,indent=2)+'\n')
for name in names:
 d=result[name];print(name,d['magic'],d['table_totals']['mono'],'letters%',round(d['letters_pct'],3),'space%',d['mono_pct'].get('␣',0),'shift/100',round(d['shift_per_100_tokens'],3),'nonzero tri',d['nonzero']['tri'])
print(json.dumps(tvs,indent=2))
