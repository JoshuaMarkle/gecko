"""Figures for the Vim layout study. Data: bench/vim-layout-2026-10-25/results.json (analyze.py).
Run from anywhere: MPLCONFIGDIR=/tmp/gecko-mpl python3 -I docs/papers/vim-layout/figures.py
"""
import json, sys
from pathlib import Path
import numpy as np
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'programming-layout'))
from corne import ROOT, INK, MUTED, HAIR, BLUE, GREY, ORANGE, SEQ, plt, freq_of, design, layers_of  # noqa: E402

BENCH = ROOT / 'bench/vim-layout-2026-10-25'
(HERE / 'assets').mkdir(exist_ok=True)
R = json.load(open(BENCH / 'results.json'))
L, C = R['layouts'], R['cells']
TIME = 0.157 / 3.626  # % time per keystroke per score point (as the programming report)


def save(fig, name):
    fig.savefig(HERE / 'assets' / (name + '.pdf'), metadata={'CreationDate': None, 'ModDate': None})
    plt.close(fig)


def frame(h, w=6.3):
    fig, ax = plt.subplots(figsize=(w, h), layout='constrained')
    ax.grid(color=HAIR, linewidth=.5, alpha=.65)
    ax.tick_params(length=0, pad=3, labelsize=6.5)
    return fig, ax


# 1. What each h j k l policy costs: its optimum's score under vim-free on vim-programmer-typed.
B = R['best_of']  # policy → its best cell (40 M or 160 M)
POL = {'free': 'free (remap motions)', 'pairs': 'pairs: h l and j k neighbours', 'fingers': 'fingers: four fingers, one hand',
       'row': 'row: h j k l in order', 'qwerty': 'QWERTY spots (pinned)'}
NAME = {B[p]: n for p, n in POL.items() if p in B}
NAME.update({'ref:prog-esc': 'programming optimum + Esc', 'ref:us-layer-esc': 'Colemak-DH, US-style layer + Esc'})
rows = sorted((n for n in NAME if n in L), key=lambda n: L[n]['scores']['vim-programmer-typed'])
b0 = L[B['free']]['scores']['vim-programmer-typed']
fig, ax = frame(2.2)
y = np.arange(len(rows))[::-1]
gap = [100 * (L[n]['scores']['vim-programmer-typed'] / b0 - 1) for n in rows]
ax.barh(y, gap, color=[GREY if n.startswith('ref:') else (ORANGE if n == B['pairs'] else BLUE) for n in rows], height=.62)
for yi, g, n in zip(y, gap, rows):
    s = L[n]['scores']['vim-programmer-typed']
    ax.text(max(g, 0) + .3, yi, f"{s:.2f}  ·  ≈ {(s - b0) * TIME:+.2f} % time", va='center', fontsize=5.4, color=MUTED)
ax.set_yticks(y, [NAME[n] for n in rows]); ax.set_xlim(0, max(gap) * 1.45 + 1)
ax.set_xlabel('% above the free optimum on vim-programmer-typed, scored without the h j k l term (lower is better)')
save(fig, 'policies')

# 2. Keyboards: the recommended (pairs) optimum and the free one.
F = freq_of('vim-programmer-typed')
for tag, fname, title in ((B['pairs'], 'pairs', 'GECKO optimum for a Vim programmer, h l / j k pairs'),
                          (B['free'], 'free', 'GECKO optimum for a Vim programmer, h j k l free')):
    if tag in C:
        save(design(C[tag]['rows'], F, f"{title} · score {L[tag]['scores']['vim-programmer-typed']:.2f} (without the h j k l term)"), fname)

# 3. Vim share: optima for 0, 11, 22 (pairs, 160 M) and 33 % Vim scored on each share.
SH = [('share-00', 'programmer-typed', 0), ('share-11', 'vimshare-11', 11), (B['pairs'], 'vim-programmer-typed', 22),
      ('share-33', 'vimshare-33', 33)]
M = np.array([[100 * (L[a]['scores'][c] / min(L[x]['scores'][c] for x, _, _ in SH) - 1) for _, c, _ in SH] for a, _, _ in SH])
fig, ax = plt.subplots(figsize=(4.2, 1.7), layout='constrained')
VM = 12
ax.imshow(np.minimum(M, VM), cmap=SEQ, vmin=0, vmax=VM, aspect='auto')
for i in range(M.shape[0]):
    for j in range(M.shape[1]):
        ax.text(j, i, f'{M[i, j]:.1f}', ha='center', va='center', fontsize=6, color='white' if M[i, j] > .55 * VM else INK)
ax.set_xticks(range(len(SH)), [f'{p} % Vim' for _, _, p in SH])
ax.set_yticks(range(len(SH)), [f'optimum at {p} %' + (' (160 M)' if p == 22 else ' (40 M)') for _, _, p in SH])
ax.tick_params(length=0, labelsize=6); [s.set_visible(False) for s in ax.spines.values()]
ax.set_xlabel('scored on (% above the best of these four)')
save(fig, 'share')
print('figures written to', HERE / 'assets')
