"""Trigram timing, Aalto mirror (2026-10-22). usage: iki3.py FILES_DIR METADATA OUT_DIR
Groups touch (FINGERS 9-10, qwerty) and touch_fast (also AVG_WPM_15 >= 70). Sections typed exactly;
three consecutive letter keystrokes, both intervals 0 < d < 1500 ms. Per participant, intervals are
divided by that participant's median letter-letter IKI. Per trigram: n, mean log(rel) of the second
interval (b->c) and of the whole span (a->c)."""
import sys, os, csv, collections
from multiprocessing import Pool
import numpy as np
GROUPS = ['touch', 'touch_fast']
META = {}
def gof(m):
    g = []
    if (m.get('LAYOUT') or '').lower() == 'qwerty' and (m.get('FINGERS') or '').startswith('9'):
        g.append(0)
        try:
            if float(m.get('AVG_WPM_15') or 0) >= 70: g.append(1)
        except ValueError: pass
    return g
def one(paths):
    n = np.zeros((2, 17576), np.int64); s2 = np.zeros((2, 17576)); st = np.zeros((2, 17576))
    for path in paths:
        m = META.get(os.path.basename(path).split('_')[0])
        if m is None: continue
        gi = gof(m)
        if not gi: continue
        sec = collections.defaultdict(list)
        with open(path, encoding='utf-8', errors='replace', newline='') as f:
            next(f, None)
            for line in f:
                p = line.rstrip('\r\n').split('\t')
                if len(p) < 9: continue
                try: sec[p[1]].append((int(p[5]), p[2], p[3], p[7]))
                except ValueError: continue
        ds = []; tri = []; d1s = []
        allbi = []
        for rows in sec.values():
            if not rows or rows[0][1] != rows[0][2]: continue
            rows.sort(key=lambda r: r[0])
            for x, y, z in zip(rows, rows[1:], rows[2:]):
                a, b, c = x[3].lower(), y[3].lower(), z[3].lower()
                if len(a) != 1 or len(b) != 1 or len(c) != 1: continue
                if not ('a' <= a <= 'z' and 'a' <= b <= 'z' and 'a' <= c <= 'z'): continue
                d1, d2 = y[0] - x[0], z[0] - y[0]
                if 0 < d1 < 1500 and 0 < d2 < 1500:
                    tri.append(((ord(a) - 97) * 26 + ord(b) - 97) * 26 + ord(c) - 97); d1s.append(d1); ds.append(d2)
        if len(ds) < 50: continue
        med = np.median(d1s + ds); tri = np.array(tri)
        l2 = np.log(np.array(ds) / med); lt = np.log((np.array(ds) + np.array(d1s)) / med)
        for g in gi:
            np.add.at(n[g], tri, 1); np.add.at(s2[g], tri, l2); np.add.at(st[g], tri, lt)
    return n, s2, st
def init(meta):
    global META; META = meta
if __name__ == '__main__':
    src, metaf, out = sys.argv[1:4]
    meta = {}
    with open(metaf, encoding='utf-8', errors='replace') as f:
        for row in csv.DictReader(f, delimiter='\t'): meta[row['PARTICIPANT_ID']] = row
    files = sorted(os.path.join(src, f) for f in os.listdir(src) if f.endswith('.txt'))
    chunks = [files[i::160] for i in range(160)]
    N = np.zeros((2, 17576), np.int64); S2 = np.zeros((2, 17576)); ST = np.zeros((2, 17576))
    with Pool(12, initializer=init, initargs=(meta,)) as pool:
        for i, (n, s2, st) in enumerate(pool.imap_unordered(one, chunks)):
            N += n; S2 += s2; ST += st; print('chunk', i, flush=True)
    os.makedirs(out, exist_ok=True)
    with open(os.path.join(out, 'trigrams_groups.csv'), 'w', newline='') as f:
        w = csv.writer(f); w.writerow(['group', 'trigram', 'n', 'mean_logrel2', 'mean_logrel_span'])
        for g, gn in enumerate(GROUPS):
            for k in np.nonzero(N[g])[0]:
                t = chr(97 + k // 676) + chr(97 + k // 26 % 26) + chr(97 + k % 26)
                w.writerow([gn, t, int(N[g, k]), round(S2[g, k] / N[g, k], 5), round(ST[g, k] / N[g, k], 5)])
    print('done')
