"""Letter-bigram press-to-press IKI over the full Aalto 136M mirror.
usage: iki.py FILES_DIR OUT_DIR
Variants: all (IKI < 2 s) and clean (also sentence typed exactly as shown).
"""
import sys, os, csv, collections
from multiprocessing import Pool

LEFT = set("QWERTASDFGZXCVB")
RIGHT = set("YUIOPHJKLNM")

def cat(a, b):
    if a == b: return "repeat"
    ha, hb = a in LEFT, b in LEFT
    if ha != hb: return "alt"
    return "left" if ha else "right"

def one(path):
    stats = collections.defaultdict(lambda: [0, 0.0])      # (variant, cat) -> n, sum
    big = collections.defaultdict(lambda: [0, 0.0])        # (variant, bigram) -> n, sum
    sec = collections.defaultdict(list)
    with open(path, encoding="utf-8", errors="replace", newline="") as f:
        next(f, None)
        for line in f:
            p = line.rstrip("\r\n").split("\t")
            if len(p) < 9: continue
            try: sec[p[1]].append((int(p[5]), p[2], p[3], p[7]))
            except ValueError: continue
    for rows in sec.values():
        if not rows: continue
        rows.sort(key=lambda r: r[0])
        clean = rows[0][1] == rows[0][2]
        for (t0, _, _, a), (t1, _, _, b) in zip(rows, rows[1:]):
            if len(a) != 1 or len(b) != 1 or not a.isalpha() or not b.isalpha(): continue
            d = t1 - t0
            if not 0 < d < 2000: continue
            a, b = a.upper(), b.upper()
            for v in (("all",) + (("clean",) if clean else ())):
                s = stats[(v, cat(a, b))]; s[0] += 1; s[1] += d
                g = big[(v, a + b)]; g[0] += 1; g[1] += d
    return os.path.basename(path), dict(stats), dict(big)

if __name__ == "__main__":
    src, out = sys.argv[1:3]
    files = sorted(os.path.join(src, f) for f in os.listdir(src) if f.endswith(".txt"))
    S = collections.defaultdict(lambda: [0, 0.0]); B = collections.defaultdict(lambda: [0, 0.0])
    with Pool(18) as pool:
        for i, (_, st, bg) in enumerate(pool.imap_unordered(one, files, chunksize=50)):
            for k, (n, s) in st.items(): S[k][0] += n; S[k][1] += s
            for k, (n, s) in bg.items(): B[k][0] += n; B[k][1] += s
            if i % 20000 == 0: print(i, flush=True)
    os.makedirs(out, exist_ok=True)
    print("files", len(files))
    for k in sorted(S): print(k, S[k][0], round(S[k][1] / S[k][0], 1))
    with open(os.path.join(out, "bigrams.csv"), "w", newline="") as f:
        w = csv.writer(f); w.writerow(["variant", "bigram", "n", "mean_ms"])
        for (v, g), (n, s) in sorted(B.items()): w.writerow([v, g, n, round(s / n, 2)])
