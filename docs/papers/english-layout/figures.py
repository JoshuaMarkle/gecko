"""Figures for the English layout study. Data: bench/english-layout-2026-10-24/ (gather.py → data.json, sweep.json).
Run from anywhere: MPLCONFIGDIR=/tmp/gecko-mpl python3 -I docs/papers/english-layout/figures.py
"""
import json, re, tomllib
from collections import Counter
from pathlib import Path
import numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.colors import LinearSegmentedColormap, TwoSlopeNorm
from matplotlib.patches import FancyBboxPatch

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
BENCH = ROOT / 'bench/english-layout-2026-10-24'
D = json.load(open(BENCH / 'data.json'))
L = {x['name']: x for x in D['layouts']}

STYLE = (ROOT / 'web/src/styles.css').read_text()
def token(name):
    return re.search(r'--' + name + r':\s*(#[0-9a-fA-F]+)', STYLE)[1]
INK, MUTED, HAIR, BLUE = [token(n) for n in ('ink', 'muted', 'hair', 'accent')]
GREY = '#9aa3ad'; ORANGE = '#d9822b'; PAPER = '#f2f2f0'
plt.rcParams.update({'font.family': 'DejaVu Sans', 'font.size': 6.5, 'text.color': INK,
    'axes.labelcolor': MUTED, 'xtick.color': MUTED, 'ytick.color': MUTED,
    'axes.edgecolor': HAIR, 'axes.spines.top': False, 'axes.spines.right': False,
    'pdf.fonttype': 42, 'axes.axisbelow': True, 'legend.frameon': False, 'legend.fontsize': 6})
(HERE / 'assets').mkdir(exist_ok=True)
SEQ = LinearSegmentedColormap.from_list('s', ['#ffffff', '#cfe0ff', BLUE])
DIV = LinearSegmentedColormap.from_list('d', [BLUE, PAPER, ORANGE])

def frame(h, w=6.3):
    fig, ax = plt.subplots(figsize=(w, h), layout='constrained')
    ax.grid(color=HAIR, linewidth=.5, alpha=.65)
    ax.tick_params(length=0, pad=3, labelsize=6.5)
    return fig, ax
def save(fig, name):
    fig.savefig(HERE / 'assets' / (name + '.pdf'), metadata={'CreationDate': None, 'ModDate': None})
    plt.close(fig)

GEOM = tomllib.load(open(ROOT / 'data/geometries/ansi-30.toml', 'rb'))['key']
POS = [(k['pos'][0], k['pos'][1]) for k in GEOM]
FINGER = [k['finger'] for k in GEOM]
# Letter frequency on prose (per mille), for shading keys.
CJ = json.load(open(Path.home() / 'projects/gecko-workspace/costfit/data/corpus_prose.json'))
MONO = dict(zip(CJ['charset'], np.array(CJ['mono'], float) / sum(CJ['mono']) * 1000))

def keyboard(ax, chars, title=None, values=None, vmax=None, cmap=SEQ, labels=None, sub=None):
    """Draw a 30-key ansi board. values: per-key shade (default: letter frequency)."""
    vals = values if values is not None else [MONO.get(c, 0) for c in chars]
    vmax = vmax or max(vals)
    for i, ((x, y), c) in enumerate(zip(POS, chars)):
        col = cmap(min(vals[i] / vmax, 1))
        ax.add_patch(FancyBboxPatch((x + .06, -y - .94), .88, .88, boxstyle='round,pad=0,rounding_size=.12',
                                    facecolor=col, edgecolor=HAIR, lw=.6))
        dark = vals[i] / vmax > .6
        ax.text(x + .5, -y - .47, c.upper() if c.isalpha() else c, ha='center', va='center', fontsize=8,
                color='white' if dark else INK, fontweight='bold')
        s = labels[i] if labels is not None else None
        if s:
            ax.text(x + .88, -y - .86, s, ha='right', va='bottom', fontsize=4.2, color='white' if dark else MUTED)
    # hand split
    ax.plot([5.0, 5.0], [.05, -0.95], color=MUTED, lw=.5, ls=':')
    ax.set_xlim(-.1, 10.9); ax.set_ylim(-3.1, .45 if title else .05); ax.set_aspect('equal'); ax.axis('off')
    if title:
        ax.text(0, .2, title, fontsize=7, color=INK, fontweight='bold', va='bottom')
    if sub:
        ax.text(10.8, .2, sub, fontsize=6, color=MUTED, va='bottom', ha='right')

def nice(n):
    return n.replace('GECKO prose (apos)', 'GECKO optimum').replace(' (ref)', '')

named = [x for x in D['layouts'] if x['kind'] in ('classic', 'modern')]
gecko = [x for x in D['layouts'] if x['kind'] == 'gecko']
OPT = L['GECKO prose (apos)']
best_set = {x['set']: x['scores']['prose'] for x in gecko}

# 1. Four keyboards: optimum, best human, Colemak-DH, QWERTY.
best_human = min(named, key=lambda x: x['scores']['prose'] - best_set[x['set']])
pick = [OPT, best_human, L['Colemak-DH'], L['QWERTY']]
fig, axs = plt.subplots(2, 2, figsize=(6.3, 2.75), layout='constrained')
for ax, x in zip(axs.flat, pick):
    keyboard(ax, x['chars'], nice(x['name']), vmax=95, sub=f"score {x['scores']['prose']:.2f}")
save(fig, 'keyboards')

# 2. Ranking: gap to the optimum of the layout's own symbol set (the fair comparison).
rows = sorted(named, key=lambda x: x['scores']['prose'] / best_set[x['set']])
fig, ax = frame(3.4)
y = np.arange(len(rows))[::-1]
gap = [100 * (x['scores']['prose'] / best_set[x['set']] - 1) for x in rows]
cols = [GREY if x['kind'] == 'classic' else BLUE for x in rows]
ax.barh(y, gap, color=cols, height=.62)
for yi, g, x in zip(y, gap, rows):
    r = x['robust']
    ax.text(g + 2, yi, f"{x['scores']['prose']:.2f}  · rank {r['rank_lo']}–{r['rank_hi']}", va='center', fontsize=5.4, color=MUTED)
ax.set_yticks(y, [nice(x['name']) for x in rows])
ax.set_xscale('symlog', linthresh=100); ax.set_xlim(0, 900)
ax.set_xticks([0, 25, 50, 75, 100, 200, 400, 800], ['0', '25', '50', '75', '100', '200', '400', '800'])
ax.set_xlabel('% above the GECKO optimum for the same four punctuation keys (lower is better)')
save(fig, 'ranking')

# 3. Held-out timing judges vs measured comfort. Two practice controls give opposite answers.
fig, axs = plt.subplots(1, 2, figsize=(6.3, 2.9), layout='constrained', sharex=True)
pts = [x for x in named + [OPT] if x['name'] != 'QWERTY']
OFF = {'Graphite': (3, -6), 'Gallium': (-28, -3), 'Dhorf': (3, -5), 'Galite': (-26, 0), 'Focal': (3, 2)}
for ax, key, lab in ((axs[0], 'J_B', 'pair-frequency control (J_B)'), (axs[1], 'J_B_letters', '+ letter-frequency control (J_B+l)')):
    ax.grid(color=HAIR, linewidth=.5, alpha=.65); ax.tick_params(length=0, pad=3, labelsize=6)
    J0 = L['QWERTY'][key]
    for x in pts:
        t = 100 * (np.exp(x[key] - J0) - 1)  # % time per keystroke vs QWERTY
        col = ORANGE if x['kind'] == 'gecko' else (GREY if x['kind'] == 'classic' else BLUE)
        ax.scatter(x['costs']['engram'], t, s=16, color=col, edgecolor='white', lw=.6, zorder=3)
        ax.annotate(nice(x['name']), (x['costs']['engram'], t), xytext=OFF.get(x['name'], (3, 2)),
                    textcoords='offset points', fontsize=5)
    ax.axhline(0, color=MUTED, lw=.6, ls='--')
    ax.set_title(lab, fontsize=6.5, color=INK, loc='left')
    ax.set_xlabel('Engram measured discomfort (%)')
axs[0].set_ylabel('time per keystroke vs QWERTY (%, lower is faster)')
save(fig, 'judges')

# 4. Corpus regret: each corpus optimum (rows) scored on each corpus (columns), % above that corpus's best.
CS = D['corpora']
CS = ['prose', 'prose-web', 'prose-chat', 'prose-books']  # shai, reddit, english give the same optima
opt_rows = [OPT] + [L['GECKO ' + c] for c in CS[1:]]
extra = [x for x in sorted(named, key=lambda x: x['scores']['prose']) if x['set'] == 'apos'][:3]
R = opt_rows + extra
best_c = {c: min(x['scores'][c] for x in opt_rows) for c in CS}
M = np.array([[100 * (x['scores'][c] / best_c[c] - 1) for c in CS] for x in R])
fig, ax = plt.subplots(figsize=(6.3, 2.2), layout='constrained')
VM = max(20, M.max())
im = ax.imshow(M, cmap=SEQ, vmin=0, vmax=VM, aspect='auto')
for i in range(M.shape[0]):
    for j in range(M.shape[1]):
        ax.text(j, i, f'{M[i, j]:.1f}', ha='center', va='center', fontsize=5.6, color='white' if M[i, j] > .55 * VM else INK)
ax.set_xticks(range(len(CS)), CS); ax.set_yticks(range(len(R)), ['optimum on ' + (x.get('corpus') or 'prose') if x['kind'] != 'modern' else nice(x['name']) for x in R])
ax.tick_params(length=0, labelsize=6); [s.set_visible(False) for s in ax.spines.values()]
ax.axhline(len(opt_rows) - .5, color='white', lw=2)
ax.set_xlabel('scored on corpus (% above the best optimum on that corpus)')
save(fig, 'corpora')

# 5. Metric profile: selected metrics, optimum vs named layouts.
METS = [('sfb', 'SFB %'), ('sfs', 'SFS %'), ('hand_switch', 'hand switch %'), ('inroll', 'in-rolls %'),
        ('outroll', 'out-rolls %'), ('home_row', 'home row %'), ('scissor', 'scissors %'), ('pinky_ring', 'pinky–ring %')]
sel = [OPT, best_human] + [L[n] for n in ('Gallium', 'Sturdy', 'Canary', 'Colemak-DH', 'Dvorak', 'QWERTY') if L[n] is not best_human]
sel = sel[:8]
fig, axs = plt.subplots(2, 4, figsize=(6.3, 3.3), layout='constrained')
for ax, (m, lab) in zip(axs.flat, METS):
    v = [abs(x['costs'][m]) for x in sel]
    yy = np.arange(len(sel))[::-1]
    ax.barh(yy, v, color=[ORANGE if x['kind'] == 'gecko' else (GREY if x['kind'] == 'classic' else BLUE) for x in sel], height=.6)
    ax.set_title(lab, fontsize=6.5, color=INK, loc='left')
    ax.set_yticks(yy, [nice(x['name']) for x in sel] if ax in axs[:, 0] else [])
    ax.tick_params(length=0, labelsize=5.6); ax.grid(axis='x', color=HAIR, lw=.5)
save(fig, 'metrics')

# 6. Weight-sweep consensus: for each key, share of perturbed optima with the optimum's letter there.
SW = BENCH / 'sweep.json'
if SW.exists():
    sw = json.load(open(SW))
    opts = [r['chars'] for r in sw['sweep']['rows']]
    def mirror(c):  # left/right mirror per row
        return ''.join(c[r * 10:(r + 1) * 10][::-1] for r in range(3))
    base = OPT['chars']
    def align(c):
        m = mirror(c)
        return c if sum(a == b for a, b in zip(c, base)) >= sum(a == b for a, b in zip(m, base)) else m
    al = [align(c) for c in opts]
    share = [np.mean([c[i] == base[i] for c in al]) for i in range(30)]
    fig, (ax, tx) = plt.subplots(1, 2, figsize=(6.3, 1.75), gridspec_kw={'width_ratios': [3, 1]}, layout='constrained')
    keyboard(ax, base, None, values=[max(s - .4, 0) for s in share], vmax=.6, labels=[f'{s * 100:.0f}' for s in share])
    tx.axis('off')
    tx.text(0, .85, f'{len(al)} optima under\nperturbed weights', fontsize=6.5, color=INK, va='top')
    tx.text(0, .45, 'small number: % that\nkeep this letter here\n(mirror-aligned)', fontsize=6, color=MUTED, va='top')
    save(fig, 'consensus')
    D['_share'] = share

print('figures written to', HERE / 'assets')
