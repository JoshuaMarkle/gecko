"""What does a held modifier cost? Shift timing in the Aalto 136M mirror (2026-10-24).
usage: shift.py KEYSTROKE_FILES_DIR METADATA_FILE OUT.json
QWERTY touch typists, sections typed exactly with no Backspace. Intervals press-to-press between
non-Shift keys, divided by the participant's median interval (log rel, clipped ±2).
Per letter c: space→C vs space→c (the Shift chord into the letter) and C→next vs c→next for the
same next letter (the release). Shifted punctuation: letter→'?' vs letter→'.' and letter→':' vs ';'.
Also Shift lead (Shift press → letter press) and whether Shift is released before the next key.
"""
import sys, os, csv, json, collections
from multiprocessing import Pool
import numpy as np

META = {}
LET = "abcdefghijklmnopqrstuvwxyz"

def ok(m):
    return (m.get('LAYOUT') or '').lower() == 'qwerty' and (m.get('FINGERS') or '').startswith('9')

def one(paths):
    S = collections.defaultdict(lambda: np.zeros(3))  # key -> n, sum, sumsq of log rel
    lead = []; npart = 0
    for path in paths:
        pid = os.path.basename(path).split('_')[0]
        m = META.get(pid)
        if m is None or not ok(m): continue
        sec = collections.defaultdict(list)
        with open(path, encoding='utf-8', errors='replace', newline='') as f:
            next(f, None)
            for line in f:
                p = line.rstrip('\r\n').split('\t')
                if len(p) < 9: continue
                try: sec[p[1]].append((int(p[5]), int(p[6]), p[2], p[3], p[7]))
                except ValueError: continue
        local = []  # (key, d)
        for rows in sec.values():
            if not rows or rows[0][2] != rows[0][3]: continue
            if any(r[4] == 'BKSP' for r in rows): continue
            rows.sort(key=lambda r: r[0])
            keys = [r for r in rows if r[4] != 'SHIFT' and len(r[4]) == 1]
            shifts = [r for r in rows if r[4] == 'SHIFT']
            for x in range(1, len(keys)):
                a, b = keys[x - 1][4], keys[x][4]
                d = keys[x][0] - keys[x - 1][0]
                if not 0 < d < 1500: continue
                local.append((('all',), d))
                if a == ' ' and b.lower() in LET:
                    local.append((('into', b.lower(), b.isupper()), d))
                if a.lower() in LET and b in LET and x >= 2 and keys[x - 2][4] == ' ':
                    local.append((('out', a.lower() + b, a.isupper()), d))  # word-initial letter → next letter
                if a in LET and b in '?.:;!1"\'':
                    local.append((('punct', b), d))
                if b.isupper() and b.lower() in LET:
                    sh = [s for s in shifts if s[0] < keys[x][0] and s[1] > keys[x][0] and s[0] > keys[x - 1][0] - 300]
                    if sh:
                        s = sh[-1]
                        nxt = keys[x + 1][0] if x + 1 < len(keys) else None
                        lead.append((keys[x][0] - s[0], s[0] - keys[x - 1][0], (nxt is not None and s[1] > nxt)))
        ds = [d for k, d in local if k == ('all',)]
        if len(ds) < 50: continue
        med = np.median(ds); npart += 1
        for k, d in local:
            if k == ('all',): continue
            v = float(np.clip(np.log(d / med), -2, 2))
            S[k] += (1, v, v * v)
    return dict(S), lead, npart

def init(meta):
    global META; META = meta

if __name__ == '__main__':
    src, metaf, out = sys.argv[1:4]
    meta = {}
    with open(metaf, encoding='utf-8', errors='replace') as f:
        for row in csv.DictReader(f, delimiter='\t'): meta[row['PARTICIPANT_ID']] = row
    files = sorted(os.path.join(src, f) for f in os.listdir(src) if f.endswith('_keystrokes.txt'))
    chunks = [files[i::400] for i in range(400)]
    S = collections.defaultdict(lambda: np.zeros(3)); L = []; NP = 0
    with Pool(16, initializer=init, initargs=(meta,)) as pool:
        for i, (s, lead, n) in enumerate(pool.imap_unordered(one, chunks)):
            for k, v in s.items(): S[k] += v
            L += lead[:2000]; NP += n
            if i % 50 == 0: print('chunk', i, NP, flush=True)
    rows = [{'key': list(k), 'n': v[0], 'mean': v[1] / v[0], 'var': v[2] / v[0] - (v[1] / v[0]) ** 2} for k, v in S.items()]
    L = np.array(L, float)
    json.dump({'participants': NP, 'cells': rows,
               'lead_ms': {'shift_to_letter_median': float(np.median(L[:, 0])), 'prev_to_shift_median': float(np.median(L[:, 1])),
                           'held_past_next_key': float(L[:, 2].mean()), 'n': len(L)}}, open(out, 'w'))
    print('participants', NP)
