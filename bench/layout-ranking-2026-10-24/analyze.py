"""Tables for the modern-layout ranking. Input: data.json (gather.py) and opt-*.json (set optima)."""
import json, statistics as st
from pathlib import Path
H = Path(__file__).resolve().parent
D = json.loads((H / 'data.json').read_text())
L = D['layouts']
FLAG = {'Vylet': 'has a `*` repeat key', 'Curtsy': 'no comma on the layout'}
SET = {',./;': 'qwerty', "',./": 'apos', "',.;": 'semi'}
def sset(c): return SET.get(''.join(sorted(x for x in c if not x.isalpha())), 'other')
OPT = {}
for f, g in [('canary', 'ansi-30'), ('graphite', 'ansi-30'), ('sturdy', 'ansi-30'),
             ('angle', 'ansi-30-angle'), ('angle-apos', 'ansi-30-angle'), ('angle-semi', 'ansi-30-angle')]:
    b = json.loads((H / f'opt-{f}.json').read_text())['run']['best'][0]
    OPT[(g, sset(''.join(b['rows'])))] = (b['score'], ''.join(b['rows']))
NAMES = {'B01': 'SFB', 'S01': 'SFS', 'S02': 'far SFS', 'B14': 'stretch', 'B11': 'finger-pair move', 'S04': 'skip stretch',
         'B05': 'scissors', 'B06': 'half scissors', 'B08': 'alternation', 'B09': 'pinky–ring', 'B10': 'row jumps',
         'M01': 'key effort', 'M05': 'home row', 'M02': 'finger load', 'M04': 'hand balance', 'T01': 'trigram alt.',
         'T03': 'rolls in', 'T04': 'rolls out', 'T06': 'redirects', 'T07': 'bad redirects', 'E05': 'comfort (Engram)'}
pool = [r for r in L if r['src'] in ('cmini', 'ref (ortho)') or r['name'] in ('Colemak-DH',)]
med = {k: st.median(r['contrib'][k] for r in pool) for k in D['weights']}
for r in L:
    r['set'] = sset(r['chars'])
    o = OPT.get((r['geom'], r['set']))
    r['opt'] = o[0] if o else None
    r['gap'] = 100 * (r['score'] / o[0] - 1) if o else None
    r['same_as_opt'] = sum(a == b for a, b in zip(r['polished'], o[1])) if o else None
    diff = {k: r['contrib'][k] - med[k] for k in med}
    r['good'] = sorted(diff, key=diff.get)[:3]; r['bad'] = sorted(diff, key=diff.get, reverse=True)[:3]
    r['diff'] = diff
    p = r['polish']
    r['s1'] = p[0]['score'] if p else r['score']; r['s3'] = p[min(2, len(p) - 1)]['score'] if p else r['score']
main = [r for r in L if r['name'] not in FLAG]
for i, r in enumerate(sorted(main, key=lambda r: r['score']), 1): r['rank'] = i
for i, r in enumerate(sorted(main, key=lambda r: r['balanced']), 1): r['rank_bal'] = i
corp = ['english', 'shai', 'reddit', 'mt-450k']
for c in corp:
    key = (lambda r: r['score']) if c == 'english' else (lambda r, c=c: r['corpora'][c])
    for i, r in enumerate(sorted(main, key=key), 1): r.setdefault('crank', {})[c] = i
if __name__ == '__main__':
    print('rank name set geom score bal rb gap E01 E05 E04 SFB SFS alt rollsin+out home | 1swap 3swap polish n moved same_as_opt | corpus ranks')
    for r in sorted(L, key=lambda r: r['score']):
        c = r['costs']
        print(f"{r.get('rank','-'):>2} {r['name']:18s} {r['set']:6s} {r['geom'][-5:]:5s} {r['score']:6.2f} {r['balanced']:6.2f} {r.get('rank_bal','-'):>2} "
              f"{(r['gap'] or 0):5.1f}% E01 {c['E01']:.2f} E05 {c['E05']:.2f} E04 {c['E04']:.2f} SFB {c['B01']:.2f} SFS {c['S01']:.1f} alt {-c['B08']:.1f} roll {-c['T03']-c['T04']:.1f} home {-c['M05']:.1f} | "
              f"{r['s1']:.2f} {r['s3']:.2f} {r['polished_score']:.2f} {len(r['polish'])} {r['moved']} {r['same_as_opt']} | {r.get('crank')}")
        print('     +', [(NAMES[k], round(r['diff'][k], 2)) for k in r['good']], ' -', [(NAMES[k], round(r['diff'][k], 2)) for k in r['bad']],
              ' swaps', [p['swap'] for p in r['polish'][:3]])
