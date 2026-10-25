"""Figures for the modern-layout ranking. Data: bench/layout-ranking-2026-10-24/ (gather.py → data.json).
Run from anywhere: MPLCONFIGDIR=/tmp/gecko-mpl python3 -I docs/papers/layout-ranking/figures.py
"""
import re, sys
from pathlib import Path
import numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.colors import LinearSegmentedColormap, TwoSlopeNorm

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'bench/layout-ranking-2026-10-24'))
import analyze as A  # noqa: E402  (tables, ranks, symbol-set optima)

STYLE = (ROOT / 'web/src/styles.css').read_text()
def token(name):
    return re.search(r'--' + name + r':\s*(#[0-9a-fA-F]+)', STYLE)[1]
INK, MUTED, HAIR, BLUE = [token(n) for n in ('ink', 'muted', 'hair', 'accent')]
GREY = '#9aa3ad'; ORANGE = '#d9822b'
plt.rcParams.update({'font.family': 'DejaVu Sans', 'font.size': 6.5, 'text.color': INK,
    'axes.labelcolor': MUTED, 'xtick.color': MUTED, 'ytick.color': MUTED,
    'axes.edgecolor': HAIR, 'axes.spines.top': False, 'axes.spines.right': False,
    'pdf.fonttype': 42, 'axes.axisbelow': True, 'legend.frameon': False, 'legend.fontsize': 6})
(HERE / 'assets').mkdir(exist_ok=True)

def frame(h, w=6.3):
    fig, ax = plt.subplots(figsize=(w, h), layout='constrained')
    ax.grid(color=HAIR, linewidth=.5, alpha=.65)
    ax.tick_params(length=0, pad=3, labelsize=6.5)
    return fig, ax
def save(fig, name):
    fig.savefig(HERE / 'assets' / (name + '.pdf'), metadata={'CreationDate': None, 'ModDate': None})
    plt.close(fig)

main = sorted([r for r in A.L if r['name'] in {m['name'] for m in A.main}], key=lambda r: r['score'])

# 1. Score now vs. after steepest-descent polish (local optimum reached from the layout).
fig, ax = frame(4.3)
rows = [r for r in main if r['name'] != 'QWERTY'][::-1]
y = np.arange(len(rows))
for i, r in enumerate(rows):
    ax.plot([r['polished_score'], r['score']], [i, i], color=GREY, lw=1, zorder=1)
ax.scatter([r['score'] for r in rows], y, s=14, color=BLUE, zorder=3, label='as published')
ax.scatter([r['polished_score'] for r in rows], y, s=14, facecolor='white', edgecolor=ORANGE, lw=1.1, zorder=3,
           label='after polish (local optimum)')
ax.axvline(8.074, color=MUTED, lw=.6, ls='--'); ax.text(8.12, len(rows) - .6, 'GECKO optimum 8.07', color=MUTED, fontsize=6)
ax.set_yticks(y, [r['name'] + (' †' if r['geom'].endswith('angle') else '') for r in rows])
ax.set_xlabel('score under the evidence default profile (lower is better)')
ax.legend(loc='center right')
save(fig, 'polish')

# 2. Speed vs comfort judges (neither is the score itself).
fig, ax = frame(3.2)
pts = [r for r in main if r['name'] != 'QWERTY']
CLUSTER = {'Gallium', 'Galite', 'Graphite', 'Maya'}
OFF = {'Dhorf': (-10, -8), 'Noctum': (3, 2), 'Nerps': (-16, 3), 'CTGAP': (-24, -3), 'Colemak': (-6, -8),
       'Colemak-DH': (-38, 2), 'Canary (angle)': (3, 3), 'ISRT': (3, -7), 'Recurva': (3, 5), 'Stronk': (-24, -7), 'Hands Down (ref)': (3, 1)}
for r in pts:
    c = r['costs']
    col = ORANGE if r['src'] == 'gecko' else (GREY if r['src'] in ('classic',) else BLUE)
    ax.scatter(c['E05'], c['E01'], s=16, color=col, edgecolor='white', lw=.5, zorder=3)
    if r['name'] in CLUSTER:
        continue
    ax.annotate(r['name'], (c['E05'], c['E01']), xytext=OFF.get(r['name'], (3, 2)), textcoords='offset points',
                fontsize=5.3, color=INK)
ax.annotate('Gallium, Galite,\nGraphite, Maya', (5.2, 6.83), xytext=(-48, -14), textcoords='offset points', fontsize=5.3,
            color=INK, arrowprops=dict(arrowstyle='-', color=GREY, lw=.5))
ax.set_xlabel('E05 measured discomfort, Engram (% , lower is better)')
ax.set_ylabel('E01 predicted typing time (% of median interval)')
save(fig, 'judges')

# 3. Fingerprint: weighted cost minus the median modern layout, per metric (blue = better than median).
METS = ['B01', 'E05', 'B09', 'B14', 'B08', 'T04', 'T03', 'T07', 'M04', 'M01', 'B10', 'M02']
top = [r for r in main if r['name'] != 'QWERTY'][:24]
M = np.array([[r['diff'][m] for m in METS] for r in top])
cmap = LinearSegmentedColormap.from_list('d', [BLUE, '#f2f2f0', ORANGE])
fig, ax = plt.subplots(figsize=(6.3, 4.4), layout='constrained')
lim = 2.5
im = ax.imshow(np.clip(M, -lim, lim), cmap=cmap, norm=TwoSlopeNorm(0, -lim, lim), aspect='auto')
ax.set_xticks(range(len(METS)), [A.NAMES[m] for m in METS], rotation=35, ha='right')
ax.set_yticks(range(len(top)), [r['name'] for r in top])
ax.tick_params(length=0, labelsize=6.3)
for s in ax.spines.values(): s.set_visible(False)
for i in range(len(top)):
    for j in range(len(METS)):
        v = M[i, j]
        if abs(v) >= .4:
            ax.text(j, i, f'{v:+.1f}', ha='center', va='center', fontsize=5, color=INK)
cb = fig.colorbar(im, ax=ax, shrink=.6, pad=.01); cb.outline.set_visible(False)
cb.ax.tick_params(labelsize=5.5, length=0); cb.set_label('score points vs median (clipped ±2.5)', fontsize=6, color=MUTED)
save(fig, 'fingerprint')
