"""Figures for the inverse-fit study. Data: bench/inverse-fit-2026-10-25/ (results.json from analyze.py,
roll_timing.json, judges.json).
Run from anywhere: MPLCONFIGDIR=/tmp/gecko-mpl python3 -I docs/papers/inverse-fit/figures.py
"""
import json, sys
from pathlib import Path
import numpy as np
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'programming-layout'))
from corne import ROOT, INK, MUTED, HAIR, BLUE, GREY, ORANGE, plt  # noqa: E402
from matplotlib.colors import LinearSegmentedColormap  # noqa: E402

BENCH = ROOT / 'bench/inverse-fit-2026-10-25'
(HERE / 'assets').mkdir(exist_ok=True)
R = json.load(open(BENCH / 'results.json'))
S = R['0.5']
RT = json.load(open(BENCH / 'roll_timing.json'))
JG = {x['name']: x for x in json.load(open(BENCH / 'judges.json'))}
DIV = LinearSegmentedColormap.from_list('d', [BLUE, '#ffffff', ORANGE])
LABEL = {'sfb': 'SFB', 'sfs': 'SFS (skipgram)', 'sfs_far': 'far SFS', 'stretch': 'stretch', 'movement': 'movement',
         'skip_stretch': 'skip stretch', 'scissor': 'scissors', 'half_scissor': 'half scissors', 'hand_switch': 'hand alternation',
         'pinky_ring': 'pinky–ring', 'row_jump': 'row jump', 'effort': 'key effort', 'home_row': 'home row',
         'finger_target': 'finger load', 'hand_balance': 'hand balance', 'alternate': 'trigram alternates',
         'inroll': 'rolls in', 'outroll': 'rolls out', 'redirect': 'redirects', 'bad_redirect': 'bad redirects',
         'engram': 'Engram comfort'}


def save(fig, name):
    fig.savefig(HERE / 'assets' / (name + '.pdf'), metadata={'CreationDate': None, 'ModDate': None})
    plt.close(fig)


# 1. Fitted multipliers: layouts (rows) × weights (columns), log scale; ✕ = the layout says little about it.
rows = [x for x in S['rows'] if x['kind'] != 'gecko'] + [x for x in S['rows'] if x['name'] == 'GECKO prose (apos)']
order = [s['id'] for s in S['modern_summary']]
M = np.array([[np.log(x['mult'][k]) for k in order] for x in rows])
fig, ax = plt.subplots(figsize=(6.3, 3.6), layout='constrained')
im = ax.imshow(M, cmap=DIV, vmin=-np.log(16), vmax=np.log(16), aspect='auto')
for i, x in enumerate(rows):
    for j, k in enumerate(order):
        if not x['informative'][k]:
            ax.text(j, i, '·', ha='center', va='center', fontsize=7, color=MUTED)
ax.set_xticks(range(len(order)), [LABEL.get(k, k) for k in order], rotation=55, ha='right', fontsize=6)
ax.set_yticks(range(len(rows)), [f"{x['name']}  ({x['fit_gap_pct']:.1f} %)" for x in rows], fontsize=6)
ax.tick_params(length=0)
for s in ax.spines.values():
    s.set_visible(False)
ax.axhline(5.5, color=INK, lw=.6)
ax.axhline(len(rows) - 1.5, color=INK, lw=.6)
cb = fig.colorbar(im, ax=ax, shrink=.7, ticks=np.log([1 / 16, 1 / 4, 1, 4, 16]))
cb.ax.set_yticklabels(['×1/16', '×1/4', '×1', '×4', '×16'], fontsize=6)
cb.outline.set_visible(False)
save(fig, 'multipliers')

# 2. Inward vs outward: timing (bigram, practice-adjusted), Engram comfort, fitted in/out ratio.
fig, axs = plt.subplots(1, 3, figsize=(6.3, 2.0), layout='constrained', gridspec_kw={'width_ratios': [1, 1, 1.2]})
cats = ['all', 'adjacent', 'skip', 'same_row', 'cross_row', 'pinky', 'no_pinky']
names = ['all', 'adjacent fingers', 'skips a finger', 'same row', 'across rows', 'with pinky', 'no pinky']
for ax, src, title, conv in ((axs[0], RT['bigram'], 'Time: inward − outward (%)', lambda v: 100 * (np.exp(v) - 1)),
                             (axs[1], RT['engram'], 'Comfort: P(inward preferred)', None)):
    y = np.arange(len(cats))[::-1]
    for yi, c in zip(y, cats):
        if c not in src:
            continue
        v = src[c]
        if conv:
            m, lo, hi = conv(v['mean']), conv(v['lo']), conv(v['hi'])
        else:
            f = lambda s: 1 / (1 + np.exp(s))  # stored as −(in − out) strength
            m, lo, hi = f(v['mean']), f(v['hi']), f(v['lo'])
        ax.plot([lo, hi], [yi, yi], color=MUTED, lw=1)
        ax.plot(m, yi, 'o', ms=3, color=ORANGE if c == 'all' else BLUE)
    ax.axvline(0 if conv else .5, color=INK, lw=.6)
    ax.set_yticks(y, names, fontsize=6)
    ax.set_title(title, fontsize=6.5, color=INK)
    ax.grid(axis='x', color=HAIR, lw=.5)
axs[0].set_xlabel('< 0: inward faster (Aalto, 95 % CI)', fontsize=5.6)
axs[1].set_xlabel('> 0.5: inward preferred (Engram, 95 % CI)', fontsize=5.6)
ax = axs[2]
mod = sorted([x for x in S['rows'] if x['kind'] in ('modern', 'classic')], key=lambda x: x['mult']['inroll'] / x['mult']['outroll'])
y = np.arange(len(mod))
ax.barh(y, [np.log2(x['mult']['inroll'] / x['mult']['outroll']) for x in mod], color=[BLUE if x['kind'] == 'modern' else GREY for x in mod], height=.65)
ax.set_yticks(y, [x['name'] for x in mod], fontsize=5.4)
ax.axvline(0, color=INK, lw=.6)
ax.set_xticks([-1, 0, 1, 2], ['×½', '×1', '×2', '×4'])
ax.set_title('Fitted rolls-in ÷ rolls-out weight', fontsize=6.5, color=INK)
ax.grid(axis='x', color=HAIR, lw=.5)
save(fig, 'rolls')

# 3. Cross matrix among the modern layouts (same symbol set or not): gap under each other's weights.
top = ['Galite', 'Graphite', 'Gallium', 'Dhorf', 'Semimak-JQ', 'Sturdy', 'Focal', 'GECKO prose (apos)']
names = S['cross']['layouts']
idx = [names.index(n) for n in top]
G = np.array(S['cross']['gap_pct'])[np.ix_(idx, idx)]
fig, ax = plt.subplots(figsize=(4.6, 3.0), layout='constrained')
im = ax.imshow(np.log10(1 + G), cmap=LinearSegmentedColormap.from_list('s', ['#ffffff', '#cfe0ff', BLUE]), vmin=0, vmax=2.3)
for i in range(len(top)):
    for j in range(len(top)):
        ax.text(j, i, f'{G[i, j]:.0f}' if G[i, j] >= 10 else f'{G[i, j]:.1f}', ha='center', va='center', fontsize=5.6,
                color='#ffffff' if G[i, j] > 60 else INK)
short = [n.replace('GECKO prose (apos)', 'GECKO') for n in top]
ax.set_xticks(range(len(top)), short, rotation=40, ha='right', fontsize=6)
ax.set_yticks(range(len(top)), short, fontsize=6)
ax.set_xlabel('weights fitted to …'); ax.set_ylabel('layout scored')
ax.tick_params(length=0)
for s in ax.spines.values():
    s.set_visible(False)
save(fig, 'cross')

# 4. The price of inward rolls: default-profile score, rolls and judges as the inroll weight grows.
P = R['price']
k = [x['k'] for x in P]
fig, axs = plt.subplots(1, 3, figsize=(6.3, 1.9), layout='constrained')
base = P[0]['default_score']
axs[0].plot(k, [100 * (x['default_score'] / base - 1) for x in P], 'o-', color=BLUE, ms=3)
axs[0].set_title('Score under default (% above ×1)', fontsize=6.5, color=INK)
axs[1].plot(k, [x['inroll'] for x in P], 'o-', color=ORANGE, ms=3, label='rolls in')
axs[1].plot(k, [x['outroll'] for x in P], 'o-', color=BLUE, ms=3, label='rolls out')
axs[1].axhspan(20, 33, color=GREY, alpha=.18, lw=0)
axs[1].text(1.05, 26, 'community rolls in', fontsize=5.2, color=MUTED)
axs[1].legend(fontsize=5.4)
axs[1].set_title('Trigram rolls (%)', fontsize=6.5, color=INK)
jb = [JG[f'inroll x{x["k"]}']['J_B'] for x in P]
axs[2].plot(k, [x['typing_time'] for x in P], 'o-', color=BLUE, ms=3, label='typing_time (fitted model)')
ax2 = axs[2].twinx()
ax2.plot(k, jb, 's--', color=ORANGE, ms=3, label='held-out J_B (% vs QWERTY)')
ax2.tick_params(length=0, labelsize=6, colors=MUTED)
ax2.spines['right'].set_visible(True)
axs[2].set_title('Predicted time: model vs held-out judge', fontsize=6.5, color=INK)
h1, l1 = axs[2].get_legend_handles_labels(); h2, l2 = ax2.get_legend_handles_labels()
axs[2].legend(h1 + h2, l1 + l2, fontsize=5, loc='upper left')
for ax in axs:
    ax.set_xscale('log', base=2)
    ax.set_xticks(k, [f'×{v}' for v in k])
    ax.grid(color=HAIR, lw=.5)
    ax.set_xlabel('rolls-in weight', fontsize=6)
save(fig, 'price')
print('figures written to', HERE / 'assets')
