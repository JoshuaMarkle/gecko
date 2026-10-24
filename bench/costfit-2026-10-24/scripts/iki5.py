"""Key-pair and key-triple timing on the 30 ansi-30 keys, Aalto 136M mirror (2026-10-24).
usage: iki4.py KEYSTROKE_FILES_DIR METADATA_FILE OUT.npz
Touch typists (FINGERS 9-10) by layout (qwerty, dvorak, azerty, qwertz; characters mapped to
their physical key), split in two halves by participant id (even/odd). Sections typed exactly as shown and without any Backspace.
Per participant every press-to-press interval (0 < d < 1500 ms) is divided by that
participant's median interval between two of the 30 keys; we store log(rel) clipped to ±2.
Bigram stats: interval i->j. Trigram stats: the interval j->k when the key before j was i.
Output arrays [group, half, ...]: n, sum, sum of squares of log rel.
"""
import sys, os, csv, collections
from multiprocessing import Pool
import numpy as np

KEYS = "qwertyuiopasdfghjkl;zxcvbnm,./"
# Character typed at each ansi-30 position (QWERTY order) for each typist layout.
MAPS = {'qwerty': KEYS, 'dvorak': "',.pyfgcrlaoeuidhtns;qjkxbmwvz",
        'azerty': "azertyuiopqsdfghjklmwxcvbn,;:!", 'qwertz': "qwertzuiopasdfghjkl\u00f6yxcvbnm,.-"}
LAYS = list(MAPS)
IDXS = [{c: i for i, c in enumerate(MAPS[l])} for l in LAYS]
N = 30
META = {}

def groups_of(m):
    """Group index = typist layout (touch typists, 9-10 fingers only)."""
    lay = (m.get('LAYOUT') or '').lower()
    f = m.get('FINGERS') or ''
    if lay not in MAPS or not f.startswith('9'):
        return []
    return [LAYS.index(lay)]

def one(paths):
    B = np.zeros((4, 2, 3, N * N)); T = np.zeros((4, 2, 3, N * N * N)); NP = np.zeros((4, 2), np.int64)
    for path in paths:
        pid = os.path.basename(path).split('_')[0]
        m = META.get(pid)
        if m is None: continue
        gs = groups_of(m)
        if not gs: continue
        half = int(pid) % 2
        sec = collections.defaultdict(list)
        with open(path, encoding='utf-8', errors='replace', newline='') as f:
            next(f, None)
            for line in f:
                p = line.rstrip('\r\n').split('\t')
                if len(p) < 9: continue
                try: sec[p[1]].append((int(p[5]), p[2], p[3], p[7]))
                except ValueError: continue
        bi = []; bd = []; tri = []; td = []
        for rows in sec.values():
            if not rows or rows[0][1] != rows[0][2]: continue
            if any(r[3] == 'BKSP' for r in rows): continue
            rows.sort(key=lambda r: r[0])
            IDX = IDXS[gs[0]]
            ks = [IDX.get(r[3].lower(), -1) if len(r[3]) == 1 else -1 for r in rows]
            ts = [r[0] for r in rows]
            for x in range(1, len(rows)):
                a, b = ks[x - 1], ks[x]
                d = ts[x] - ts[x - 1]
                if a < 0 or b < 0 or not 0 < d < 1500: continue
                bi.append(a * N + b); bd.append(d)
                if x >= 2 and ks[x - 2] >= 0 and 0 < ts[x - 1] - ts[x - 2] < 1500:
                    tri.append((ks[x - 2] * N + a) * N + b); td.append(d)
        if len(bd) < 50: continue
        bd = np.array(bd, float); med = np.median(bd)
        lb = np.clip(np.log(bd / med), -2, 2); bi = np.array(bi)
        lt = np.clip(np.log(np.array(td, float) / med), -2, 2) if td else np.zeros(0); tri = np.array(tri, int)
        for g in gs:
            NP[g, half] += 1
            for k, v in ((0, 1.0), (1, lb), (2, lb * lb)):
                np.add.at(B[g, half, k], bi, v)
            if len(tri):
                for k, v in ((0, 1.0), (1, lt), (2, lt * lt)):
                    np.add.at(T[g, half, k], tri, v)
    return B, T, NP

def init(meta):
    global META; META = meta

if __name__ == '__main__':
    src, metaf, out = sys.argv[1:4]
    meta = {}
    with open(metaf, encoding='utf-8', errors='replace') as f:
        for row in csv.DictReader(f, delimiter='\t'): meta[row['PARTICIPANT_ID']] = row
    files = sorted(os.path.join(src, f) for f in os.listdir(src) if f.endswith('_keystrokes.txt'))
    chunks = [files[i::400] for i in range(400)]
    B = np.zeros((4, 2, 3, N * N)); T = np.zeros((4, 2, 3, N * N * N)); NP = np.zeros((4, 2), np.int64)
    with Pool(16, initializer=init, initargs=(meta,)) as pool:
        for i, (b, t, n) in enumerate(pool.imap_unordered(one, chunks)):
            B += b; T += t; NP += n
            if i % 20 == 0: print('chunk', i, NP.tolist(), flush=True)
    np.savez_compressed(out, B=B, T=T, NP=NP, keys=KEYS, lays=LAYS)
    print('participants [group][half]', NP.tolist())
