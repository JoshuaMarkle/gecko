"""Bigram IKI by typist group, Aalto 136M mirror (second pass, 2026-10-22).
usage: iki2.py KEYSTROKE_FILES_DIR METADATA_FILE OUT_DIR
Groups from participant metadata: touch (FINGERS 9-10, qwerty), touch_fast (also AVG_WPM_15 >= 70),
few (FINGERS 1-2 or 3-4), all. Per participant, IKIs are also divided by that participant's
median letter-letter IKI (rel), removing speed differences between people.
Only sections typed exactly as shown; press-to-press, both keys letters, 0 < IKI < 1500 ms.
Output per group and ordered letter pair: n, median ms (5 ms bins), mean rel, median rel.
"""
import sys, os, csv, collections
from multiprocessing import Pool
import numpy as np

BINS = 300  # 5 ms bins up to 1500 ms
RBINS = 400  # rel bins of 0.01 up to 4.0

def groups_of(m):
    g = ['all']
    lay = (m.get('LAYOUT') or '').lower()
    f = m.get('FINGERS') or ''
    try: wpm = float(m.get('AVG_WPM_15') or 0)
    except ValueError: wpm = 0
    if lay == 'qwerty' and f.startswith('9'):
        g.append('touch')
        if wpm >= 70: g.append('touch_fast')
    if lay == 'qwerty' and (f.startswith('1') or f.startswith('3')):
        g.append('few')
    return g

META = {}
GROUPS = ['all', 'touch', 'touch_fast', 'few']

def one(paths):
    H = np.zeros((len(GROUPS), 676, BINS), np.int32); R = np.zeros((len(GROUPS), 676, RBINS), np.int32)
    NP = np.zeros(len(GROUPS), np.int64)
    for path in paths:
        pid = os.path.basename(path).split('_')[0]
        m = META.get(pid)
        if m is None: continue
        gi = [GROUPS.index(g) for g in groups_of(m)]
        sec = collections.defaultdict(list)
        with open(path, encoding='utf-8', errors='replace', newline='') as f:
            next(f, None)
            for line in f:
                p = line.rstrip('\r\n').split('\t')
                if len(p) < 9: continue
                try: sec[p[1]].append((int(p[5]), p[2], p[3], p[7]))
                except ValueError: continue
        bg = []; ds = []
        for rows in sec.values():
            if not rows or rows[0][1] != rows[0][2]: continue
            rows.sort(key=lambda r: r[0])
            for (t0, _, _, a), (t1, _, _, b) in zip(rows, rows[1:]):
                if len(a) != 1 or len(b) != 1: continue
                a = a.lower(); b = b.lower()
                if not ('a' <= a <= 'z' and 'a' <= b <= 'z'): continue
                d = t1 - t0
                if 0 < d < 1500: bg.append((ord(a) - 97) * 26 + ord(b) - 97); ds.append(d)
        if len(ds) < 50: continue
        bg = np.array(bg); ds = np.array(ds); med = np.median(ds)
        db = np.minimum(ds // 5, BINS - 1); rb = np.minimum((ds / med * 100).astype(int), RBINS - 1)
        for g in gi:
            NP[g] += 1
            np.add.at(H[g], (bg, db), 1); np.add.at(R[g], (bg, rb), 1)
    return H, R, NP

def init(meta):
    global META; META = meta

if __name__ == '__main__':
    src, metaf, out = sys.argv[1:4]
    meta = {}
    with open(metaf, encoding='utf-8', errors='replace') as f:
        r = csv.DictReader(f, delimiter='\t')
        for row in r: meta[row['PARTICIPANT_ID']] = row
    print('participants in metadata', len(meta), flush=True)
    files = sorted(os.path.join(src, f) for f in os.listdir(src) if f.endswith('.txt'))
    chunks = [files[i::160] for i in range(160)]
    H = np.zeros((len(GROUPS), 676, BINS), np.int64); R = np.zeros((len(GROUPS), 676, RBINS), np.int64); NP = np.zeros(len(GROUPS), np.int64)
    with Pool(16, initializer=init, initargs=(meta,)) as pool:
        for i, (h, r, n) in enumerate(pool.imap_unordered(one, chunks)):
            H += h; R += r; NP += n
            print('chunk', i, flush=True)
    os.makedirs(out, exist_ok=True)
    print('participants per group', dict(zip(GROUPS, NP.tolist())))
    def med(h, w):
        c = np.cumsum(h); return (np.searchsorted(c, c[-1] / 2) + 0.5) * w
    with open(os.path.join(out, 'bigrams_groups.csv'), 'w', newline='') as f:
        w = csv.writer(f); w.writerow(['group', 'bigram', 'n', 'median_ms', 'mean_rel', 'median_rel'])
        for g, gn in enumerate(GROUPS):
            for k in range(676):
                h = H[g, k]; n = int(h.sum())
                if n == 0: continue
                r = R[g, k]
                mean_rel = float((r * (np.arange(RBINS) + 0.5)).sum() / n / 100)
                w.writerow([gn, chr(97 + k // 26) + chr(97 + k % 26), n, round(med(h, 5), 1), round(mean_rel, 4), round(med(r, 0.01), 3)])
    with open(os.path.join(out, 'groups.txt'), 'w') as f: f.write(repr(dict(zip(GROUPS, NP.tolist()))) + '\n')
