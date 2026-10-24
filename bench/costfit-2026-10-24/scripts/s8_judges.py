"""Study 8: judge the evidence optimum vs the default optimum and the best known layouts."""
import json, subprocess, tomllib
from lib4 import *
from judge import Judge, KEYS
EV = tomllib.load(open(f'{GECKO}/data/profiles/evidence.toml', 'rb'))
L = {'evidence opt (english)': 'raw:wgncvzuo,qdtrsfyheiapblmjxk/;.', 'evidence opt (shai)': 'raw:bgdlxjyou.cstnwpheiafvmrqzk/;,',
     'default opt': 'raw:lwdgbzquo.rntsfpheaijxmcvky/;,', 'galite-reference': 'cmini:galite-reference',
     'gallium-stdpct': 'cmini:gallium-stdpct', 'graphite': 'graphite', 'sturdy': 'sturdy', 'canary': 'canary', 'semimak': 'semimak',
     'dvorak': 'dvorak', 'colemak-dh': 'colemak-dh', 'qwerty': 'qwerty'}
def feats(corpus):
    out = subprocess.run([f'{W}/target/release/costfit', 'features', '-c', corpus], input='\n'.join(L.values()) + '\n', capture_output=True, text=True, check=True).stdout
    rows = [json.loads(l) for l in out.splitlines()]; return {k: r for k, r in zip(L, rows)}
F = feats('english')
JA, JB, JBl = Judge(0), Judge(1), Judge(1, letters=True)
bA, bB = fit_time(0), fit_time(1)
T = lambda r, b: 100 * sum((x if m == 'M01' else x / 100) * r['u'][m] for m, x in b.items())
# E05 coverage: share of bigram mass on measured Engram pairs
import csv
meas = {}
for row in csv.reader(l for l in open(f'{GECKO}/data/comfort/engram-bt.csv') if not l.startswith('#')):
    if row[0] == 'a_row': continue
    ar, ac, br, bc = map(int, row[:4]); meas[(ar * 10 + ac, br * 10 + bc)] = row[5] == '1'
def measured_share(chars):
    s = JB.C.slots(chars); on = np.where(s >= 0)[0]; tot = m = 0.0
    for a in on:
        for b in on:
            f = JB.C.F2[a, b]
            if f and s[a] != s[b] and (s[a] // 10 == s[b] // 10 or True):
                same_hand = (s[a] % 10 < 5) == (s[b] % 10 < 5)
                if same_hand: tot += f; m += f * meas.get((s[a], s[b]), False)
    return m / tot
rows = {}
hdr = f"{'layout':24} {'evid':>6} {'dflt':>6} {'T_A%':>6} {'T_B%':>6} {'J_B':>7} {'J_B+l':>7} {'E01':>5} {'E05':>5} {'E04':>5} {'SFB':>5} {'SFS':>5} {'alt':>5} {'roll':>5} {'redir':>5} {'home':>5} {'M02':>5} {'E05meas':>7}"
print(hdr)
for k, r in F.items():
    c = r['chars']; raw = r['raw']
    rec = {'chars': c, 'evidence': score(r, EV['weights']) if False else None}
    ev = sum(EV['weights'][m] / EV['scale'][m] * r['u'][m] for m in EV['weights'])
    rec = {'chars': c, 'evidence': ev, 'default': score(r, DEFAULT['weights']), 'T_A': T(r, bA), 'T_B': T(r, bB), 'J_B': JB(c), 'J_B_letters': JBl(c),
           'E01': raw['E01'], 'E05': raw['E05'], 'E04': raw['E04'], 'SFB': raw['B01'], 'SFS': raw['S01'], 'alt': raw['B08'], 'rolls': raw['T03'] + raw['T04'],
           'redirects': raw['T06'], 'home': raw['M05'], 'M02': raw['M02'], 'E05_measured_share': measured_share(c)}
    rows[k] = rec
    print(f"{k:24} {ev:6.2f} {rec['default']:6.2f} {rec['T_A']:+6.2f} {rec['T_B']:+6.2f} {rec['J_B']:+.4f} {rec['J_B_letters']:+.4f} {rec['E01']:5.2f} {rec['E05']:5.2f} {rec['E04']:5.2f} {rec['SFB']:5.2f} {rec['SFS']:5.2f} {rec['alt']:5.1f} {rec['rolls']:5.1f} {rec['redirects']:5.2f} {rec['home']:5.1f} {rec['M02']:5.2f} {rec['E05_measured_share']*100:6.1f}%")
# best cmini (qwerty set: , . / ;) under each profile, all 30 distinct symbols
cm = load(f'{W}/data/cmini.jsonl')
qset = set("abcdefghijklmnopqrstuvwxyz,./;")
ok = [r for r in cm if set(r['chars']) == qset]
for nm, P in (('evidence', EV), ('default', DEFAULT)):
    sc = sorted(ok, key=lambda r: sum(P['weights'][m] / P['scale'][m] * r['u'][m] for m in P['weights']))
    b = sc[0]; s = sum(P['weights'][m] / P['scale'][m] * b['u'][m] for m in P['weights'])
    print(f"best cmini (qwerty set, {len(ok)} layouts) under {nm}: {b['name']} {s:.3f}")
json.dump(rows, open(f'{W}/data/s8_judges.json', 'w'), indent=1)
