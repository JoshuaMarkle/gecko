"""Figures for the software paper, from repo data only (no synthetic values).
Run from anywhere: MPLCONFIGDIR=/tmp/gecko-mpl python3 docs/papers/software/figures.py
Sources: bench/memetic-2026-10-21/leaderboard.json; bench/software-paper-2026-10-21/ (metric-count sweep,
thread scaling, the SA and QD runs, Graphite score); numbers copied from
context/algorithms/bounds.md (run 20261021-171218-sa-a3306812) and context/objective.md
(geometry rebuild table); `gecko compare` output of 2026-10-21 (default profile, ansi-30, english).
"""
import json, re
from pathlib import Path
import numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.patches import FancyBboxPatch

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
STYLE = (ROOT / 'web/src/styles.css').read_text()
def token(name):
    return re.search(r'--' + name + r':\s*(#[0-9a-fA-F]+)', STYLE)[1]
INK, MUTED, HAIR, BLUE = [token(n) for n in ('ink', 'muted', 'hair', 'accent')]
plt.rcParams.update({'font.family': 'DejaVu Sans', 'font.size': 6.5, 'text.color': INK,
    'axes.labelcolor': MUTED, 'xtick.color': MUTED, 'ytick.color': MUTED,
    'axes.edgecolor': HAIR, 'axes.spines.top': False, 'axes.spines.right': False,
    'pdf.fonttype': 42, 'axes.axisbelow': True})

def frame(h=2.2):
    fig, ax = plt.subplots(figsize=(3.4, h), layout='constrained')
    ax.grid(color=HAIR, linewidth=.5, alpha=.65)
    ax.tick_params(length=0, pad=3, labelsize=6.5)
    return fig, ax
def save(fig, name):
    fig.savefig(HERE / 'assets' / (name + '.pdf'), metadata={'CreationDate': None, 'ModDate': None})
    plt.close(fig)

# 1. Architecture / data flow.
fig, ax = plt.subplots(figsize=(3.4, 2.9), layout='constrained')
ax.set_xlim(0, 10); ax.set_ylim(0, 9.4); ax.axis('off')
def box(x, y, w, h, text, fc='white', ec=HAIR, tc=INK):
    ax.add_patch(FancyBboxPatch((x, y), w, h, boxstyle='round,pad=0.02,rounding_size=0.15', fc=fc, ec=ec, lw=.8))
    ax.text(x + w/2, y + h/2, text, ha='center', va='center', fontsize=6, color=tc)
def arrow(x0, y0, x1, y1):
    ax.annotate('', (x1, y1), (x0, y0), arrowprops=dict(arrowstyle='-|>', color=MUTED, lw=.7, shrinkA=0, shrinkB=0))
ax.text(0, 9.2, 'inputs (files)', fontsize=5.6, color=MUTED, va='center')
box(0, 7.5, 2.6, 1.3, 'text\ncorpus')
box(3.2, 7.5, 2.6, 1.3, 'board geometry\n+ hand model')
box(6.4, 7.5, 2.6, 1.3, 'profile\n(weights)')
box(0, 5.5, 2.6, 1.0, 'n-gram\ntables')
box(3.2, 5.5, 2.6, 1.0, 'cost\ntables')
box(6.4, 5.5, 2.6, 1.0, 'metric\nweights')
box(1.0, 3.7, 8.0, 1.1, 'scorer: folded tables, swap-delta state', fc=BLUE, ec=BLUE, tc='white')
box(0, 1.9, 2.9, 1.0, 'optimizers\n(8, one trait)')
box(3.4, 1.9, 2.7, 1.0, 'bounds,\natlas, QD')
box(6.6, 1.9, 2.9, 1.0, 'runs/\n(verifiable)')
box(0, 0, 9.5, 1.0, 'CLI (--json)  ·  gecko serve (HTTP, SSE)  ·  WASM in the web UI', fc='#f4f6f9' if False else 'white')
for x in (1.3, 4.5, 7.7): arrow(x, 7.5, x, 6.5)
for x in (1.3, 4.5, 7.7): arrow(x, 5.5, 3.0 if x < 4 else (5.0 if x < 6 else 7.0), 4.8)
for x in (1.45, 4.75): arrow(4.8 if x > 3 else 3.5, 3.7, x, 2.9)
arrow(6.4, 3.7, 8.0, 2.9)
for x in (1.45, 4.75, 8.05): arrow(x, 1.9, x, 1.0)
save(fig, 'architecture')

# 2. Leaderboard: per-seed scores (bench/memetic-2026-10-21).
lb = json.load(open(ROOT / 'bench/memetic-2026-10-21/leaderboard.json'))['results']
order = ['bls', 'sa', 'memetic', 'tabu']
names = {'bls': 'BLS', 'sa': 'SA', 'memetic': 'Memetic', 'tabu': 'Tabu'}
fig, ax = frame(2.0)
rng = np.random.default_rng(0)  # horizontal jitter only
for i, a in enumerate(order):
    r = next(x for x in lb if x['algo'] == a)
    xs = i + rng.uniform(-.14, .14, len(r['scores']))
    ax.scatter(xs, r['scores'], s=10, color=BLUE if a == 'bls' else MUTED, alpha=.8, linewidths=0)
    ax.hlines(r['mean'], i - .28, i + .28, color=INK, linewidth=1.2)
ax.set(xticks=range(4), xticklabels=[names[a] for a in order], ylabel='Final score · lower is better')
save(fig, 'leaderboard')

# 3. Optimality gap vs number of free symbols (context/algorithms/bounds.md).
free = [30, 16, 10]
fig, ax = frame(2.0)
x = np.arange(3)
ax.bar(x - .18, [90.3, 24.5, 12.9], width=.34, color=HAIR, label='Total objective')
ax.bar(x + .18, [70.3, 12.1, 1.7], width=.34, color=BLUE, label='Bigram part')
for xi, v in zip(x + .18, [70.3, 12.1, 1.7]):
    ax.text(xi, v + 1.5, f'{v:g}', ha='center', fontsize=6, color=INK)
ax.set(xticks=x, xticklabels=[str(f) for f in free], xlabel='Free symbols', ylabel='Optimality gap (%)', ylim=(0, 100))
ax.legend(frameon=False, fontsize=6.5, loc='upper right')
save(fig, 'bounds')

# 4. Cost-table rebuild after moving one key (context/objective.md).
boards = ['ANSI-30', 'Corne-36', 'Dactyl 5x6']
full, one = [3.1, 4.6, 19.5], [0.64, 0.78, 2.2]
fig, ax = frame(2.0)
x = np.arange(3)
ax.bar(x - .18, full, width=.34, color=HAIR, label='Full build')
ax.bar(x + .18, one, width=.34, color=BLUE, label='One-key rebuild')
ax.set(xticks=x, xticklabels=boards, ylabel='Time (ms)')
ax.legend(frameon=False, fontsize=6.5, loc='upper left')
save(fig, 'rebuild')

# 5. Reference layouts: score vs SFB (gecko compare, default profile).
ref = {'Graphite': (21.68, 1.16), 'Sturdy': (21.83, .89), 'Semimak': (24.05, .96), 'Canary': (25.85, .97),
       'Dvorak': (31.43, 2.58), 'Colemak-DH': (31.57, 1.54), 'Halmak': (33.16, 2.75),
       'Colemak': (33.67, 1.54), 'Workman': (37.66, 3.00), 'QWERTY': (69.58, 6.63)}
fig, ax = frame(2.3)
for n, (s, sfb) in ref.items():
    ax.scatter(sfb, s, s=14, color=MUTED, linewidths=0)
    dx, dy, ha = (.08, 0, 'left')
    if n == 'Colemak-DH': dy, dx = -3.4, .08
    if n == 'Colemak': dy = 1.2
    if n == 'Sturdy': dy = -2.4
    if n == 'Canary': dy = 1.4
    if n == 'Graphite': dy = -0.3
    ax.text(sfb + dx, s + dy, n, fontsize=5.8, color=INK, ha=ha, va='center')
ax.set(xlabel='Same-finger bigrams (% of bigrams)', ylabel='Score · lower is better', xlim=(0.5, 7.6), ylim=(17, 72))
save(fig, 'references')

B = ROOT / 'bench/software-paper-2026-10-21'

# 6. Swap-delta throughput against the number of enabled metrics (bench/.../metrics_scaling.py).
ms = json.load(open(B / 'metrics_scaling.json'))
fig, ax = frame(2.1)
ax.plot([r['metrics'] for r in ms], [r['delta_per_sec'] / 1e6 for r in ms], color=MUTED, lw=.8, marker='o', ms=3, mfc=BLUE, mec=BLUE)
for r, dx, dy in [(ms[1], 1.0, -.45), (ms[2], 1.2, .55), (ms[6], 1.2, .55), (ms[8], 1.2, .7)]:
    ax.annotate({'B03': 'bigram tables only', 'S02': '+ skipgram table', 'M07': '+ load terms', 'T01': '+ trigram table'}[r['last_added']],
                (r['metrics'], r['delta_per_sec'] / 1e6), (r['metrics'] + dx, r['delta_per_sec'] / 1e6 + dy), fontsize=5.8, color=INK)
ax.set(xlabel='Metrics enabled', ylabel='Swap deltas per second (millions)', ylim=(0, 5.6), xlim=(0, 37))
save(fig, 'metricsweep')

# 7. Search throughput against threads (bench/.../thread_scaling.sh).
ts = json.load(open(B / 'thread_scaling.json'))
fig, ax = frame(2.1)
th = sorted({r['threads'] for r in ts})
mean = {t: np.mean([r['evals_per_sec'] for r in ts if r['threads'] == t]) / 1e6 for t in th}
ax.plot(th, [mean[1] * t for t in th], color=HAIR, lw=.9, ls='--', label='linear from 1 thread')
ax.scatter([r['threads'] for r in ts], [r['evals_per_sec'] / 1e6 for r in ts], s=9, color=BLUE, linewidths=0, alpha=.85, zorder=3)
ax.plot(th, [mean[t] for t in th], color=BLUE, lw=.8)
ax.axvline(14, color=HAIR, lw=.6); ax.text(14.2, 0.35, '14 cores', fontsize=5.8, color=MUTED)
ax.set(xlabel='Threads', ylabel='Evaluations per second (millions)', xlim=(0, 21), ylim=(0, 11), xticks=[1, 4, 8, 12, 16, 20])
ax.legend(frameon=False, fontsize=6, loc='upper left')
save(fig, 'threads')

# 8. What the optimizer traded against Graphite: weighted per-metric score, optimized minus Graphite
#    (same profile; only metrics with a weight). Lower is better, so bars to the left are wins.
best = json.load(open(B / 'runs/20261022-031427-sa-8b3885fd/best.json'))['best'][0]['breakdown']['metrics']
gra = {m['id']: m for m in json.load(open(B / 'graphite-score.json'))['breakdown']['metrics']}
rows = [(m['name'], m['weighted'] - gra[m['id']]['weighted']) for m in best if m['weight']]
rows.sort(key=lambda r: r[1])
fig, ax = frame(2.6)
ax.grid(axis='y', visible=False)
ax.barh(range(len(rows)), [r[1] for r in rows], color=[BLUE if r[1] < 0 else MUTED for r in rows], height=.62)
ax.set_yticks(range(len(rows)), [r[0] for r in rows], fontsize=6)
ax.axvline(0, color=INK, lw=.6); ax.invert_yaxis()
ax.set(xlabel='Weighted score: optimized minus Graphite')
save(fig, 'tradeoff')

# 9. MAP-Elites map (bench/.../runs/*qd*/qd.json): best score per (roll ratio, index load) cell.
qd = json.load(open(B / 'runs/20261022-031516-qd-af3fe56a/qd.json'))
nb = [a['bins'] for a in qd['axes']]
grid = np.full((nb[1], nb[0]), np.nan)
for e in qd['elites']:
    grid[e['bins'][1], e['bins'][0]] = e['score']
from matplotlib.colors import LinearSegmentedColormap
cmap = LinearSegmentedColormap.from_list('gecko', [BLUE, '#dfe6f1'])
fig, ax = plt.subplots(figsize=(3.4, 2.5), layout='constrained')
ex = [qd['axes'][0]['lo'], qd['axes'][0]['hi'], qd['axes'][1]['lo'], qd['axes'][1]['hi']]
im = ax.imshow(grid, origin='lower', aspect='auto', cmap=cmap, extent=ex, vmin=17.5, vmax=27.5, interpolation='nearest')
b = qd['best']
ax.scatter([b['desc'][0]], [b['desc'][1]], s=22, facecolors='none', edgecolors=INK, linewidths=.9)
ax.annotate('best (17.56)', b['desc'], (b['desc'][0] + 5, b['desc'][1] - 9.5), fontsize=6, color=INK, arrowprops=dict(arrowstyle='-', color=MUTED, lw=.5))
ax.set(xlabel='Rolls (% of rolls + alternations)', ylabel='Index-finger load (%)')
ax.tick_params(length=0, labelsize=6.5)
for sp in ax.spines.values(): sp.set_visible(False)
cb = fig.colorbar(im, ax=ax, pad=.02, fraction=.05); cb.ax.tick_params(length=0, labelsize=6); cb.outline.set_visible(False)
cb.set_label('Best score in cell', fontsize=6.5, color=MUTED)
save(fig, 'qdmap')
print('figures written to', HERE / 'assets')
