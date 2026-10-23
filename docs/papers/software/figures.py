"""Figures for the software paper. Data figures use repo data only (no synthetic values);
template figures (section at the end) are watermarked layouts for planned results and plot
illustrative shapes, never numbers. Replace each with its data figure when the run exists.
Run from anywhere: MPLCONFIGDIR=/tmp/gecko-mpl python3 docs/papers/software/figures.py
Sources: bench/memetic-2026-10-21/leaderboard.json; bench/software-paper-2026-10-21/ (metric-count sweep,
thread scaling, the SA and QD runs, Graphite score); numbers copied from
context/algorithms/bounds.md (run 20261021-171218-sa-a3306812) and context/objective.md
(geometry rebuild table); `gecko compare` output of 2026-10-21 (default profile, ansi-30, english);
context/algorithms/bounds.md "Certified sub-problems" and context/algorithms/geometry-search.md
(board + layout search tables).
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
box(0, 1.9, 2.25, 1.0, 'layout\noptimizers')
box(2.45, 1.9, 2.25, 1.0, 'bounds, certify,\natlas, QD')
box(4.9, 1.9, 2.25, 1.0, 'board search\n→ STL')
box(7.35, 1.9, 2.25, 1.0, 'runs/\n(verifiable)')
box(0, 0, 9.6, 1.0, 'CLI (--json)  ·  gecko serve (HTTP, SSE)  ·  WASM in the web UI')
for x in (1.3, 4.5, 7.7): arrow(x, 7.5, x, 6.5)
for x in (1.3, 4.5, 7.7): arrow(x, 5.5, 3.0 if x < 4 else (5.0 if x < 6 else 7.0), 4.8)
for x in (1.12, 3.57, 6.02): arrow(max(x, 2.0), 3.7, x, 2.9)
arrow(8.0, 3.7, 8.47, 2.9)
for x in (1.12, 3.57, 6.02, 8.47): arrow(x, 1.9, x, 1.0)
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

# 10. Certified sub-problems: time to prove the optimum against free symbols (bounds.md, B7).
k = [10, 12, 14, 16, 18, 20, 22]
full_t = [0.01, 0.1, 0.5, 3.3, 12.7, 74, None]
pair_t = [0.005, 0.005, 0.1, 0.6, 2.2, 10, 66]
fig, ax = frame(2.0)
ax.plot(k[:6], full_t[:6], color=BLUE, lw=.9, marker='o', ms=3, label='full objective')
ax.plot(k, pair_t, color=MUTED, lw=.9, marker='s', ms=3, label='pair part')
ax.scatter([22], [300], marker='x', s=16, color=BLUE, linewidths=.9)
ax.text(21.6, 160, 'stopped\nat 300 s', fontsize=5.6, color=INK, ha='right', va='center')
ax.set_yscale('log')
ax.set(xlabel='Free symbols k (others fixed at the run\'s layout)', ylabel='Time to certify (s)', xticks=k)
ax.legend(frameon=False, fontsize=6.2, loc='upper left')
save(fig, 'certify')

# 11. Board + layout search: start board vs best board, equal layout effort (geometry-search.md).
runs = [('2D', 11.63, 10.53), ('2D, alt.', 11.70, 10.46), ('3D', 11.56, 7.24),
        ('3D, bounded', 11.63, 7.27), ('3D, priors', 11.66, 7.48)]
fig, ax = frame(2.0)
x = np.arange(len(runs))
ax.bar(x - .18, [r[1] for r in runs], width=.34, color=HAIR, label='start board (Corne)')
ax.bar(x + .18, [r[2] for r in runs], width=.34, color=BLUE, label='best board found')
for xi, r in zip(x, runs):
    ax.text(xi + .18, r[2] + .25, f'{100 * (r[2] - r[1]) / r[1]:.0f}%', ha='center', fontsize=5.8, color=INK)
ax.set(xticks=x, ylabel='Score · lower is better', ylim=(0, 16))
ax.set_xticklabels([r[0] for r in runs], fontsize=6)
ax.legend(frameon=False, fontsize=6.2, loc='upper right', ncol=2)
save(fig, 'geomsearch')

# 12. The web UI's Home flowchart (web/src/components/views/FlowChart.tsx): same boxes, positions
#     and links, drawn at text width. Box lines show the bundled defaults, not live state.
FW, FH = 1120, 470
POS = {'board': (16, 16, 196, 84), 'hand': (16, 116, 196, 84), 'profile': (16, 216, 196, 84), 'language': (16, 364, 196, 84),
       'cost': (268, 102, 200, 112), 'freq': (268, 364, 200, 84), 'score': (520, 216, 190, 112), 'summary': (520, 380, 190, 68),
       'layout': (764, 16, 176, 84), 'shape': (764, 116, 176, 84), 'both': (764, 216, 176, 84), 'explore': (764, 316, 176, 84),
       'runs': (984, 102, 120, 84), 'export': (984, 242, 120, 84)}
BOX = {'board': ('Board', ['corne-36 · 36 keys', 'Parametric: shape moves'], 'ok'),
       'hand': ('Hand', ['Size preset or measured', 'Used in optimization'], 'ok'),
       'profile': ('Profile', ['hand · weights in spreads', '48 metrics available'], 'ok'),
       'language': ('Language', ['english', 'or a mix of corpora'], 'ok'),
       'cost': ('Physical cost', ['Effort per key · finger load', 'Reach · press angle · wrist', 'Movement · rolls · trigrams'], None),
       'freq': ('Frequencies', ['Characters · bigrams', 'Trigrams · skipgrams'], None),
       'score': ('Scoring model', ['Frequency × physical cost', 'Profile weights', 'Lower is better'], None),
       'summary': ('Summary', ['Every input of the next run'], None),
       'layout': ('Layout', ['Layout for the board', 'SA, BLS, tabu, GA, …'], 'ok'),
       'shape': ('Board', ['Board for the layout', 'CMA-ES over shape'], 'ok'),
       'both': ('Both', ['Board and layout', 'together (bilevel)'], 'ok'),
       'explore': ('Explore', ['Map of good layouts', 'atlas · QD · fronts'], 'ok'),
       'runs': ('Runs', ['Saved', 'Verifiable'], None),
       'export': ('Export', ['Image · files', 'STL'], None)}
SOFT, OK = token('soft'), token('success')
fig = plt.figure(figsize=(6.77, 6.77 * FH / FW))
ax = fig.add_axes((0, 0, 1, 1)); ax.set_xlim(0, FW); ax.set_ylim(FH, 0); ax.axis('off')
def fpath(pts, arrow=True, dashed=False):
    xs, ys = zip(*pts)
    ax.plot(xs, ys, color=MUTED, lw=.7, ls=(0, (2, 2)) if dashed else '-', solid_joinstyle='round', zorder=1)
    if arrow:
        ax.annotate('', pts[-1], (pts[-1][0] - 6, pts[-1][1]) if pts[-1][1] == pts[-2][1] else (pts[-1][0], pts[-1][1] - 6),
                    arrowprops=dict(arrowstyle='-|>,head_length=.35,head_width=.18', color=MUTED, lw=.7, shrinkA=0, shrinkB=0))
cy = lambda k: POS[k][1] + POS[k][3] / 2
rt = lambda k: POS[k][0] + POS[k][2]
fpath([(rt('board'), cy('board')), (240, cy('board')), (240, 140), (268, 140)])
fpath([(rt('hand'), cy('hand')), (268, cy('hand'))])
fpath([(rt('profile'), cy('profile')), (240, cy('profile')), (240, 188), (268, 188)])
fpath([(rt('language'), cy('language')), (268, cy('language'))])
fpath([(rt('cost'), cy('cost')), (494, cy('cost')), (494, 256), (520, 256)])
fpath([(rt('freq'), cy('freq')), (494, cy('freq')), (494, 292), (520, 292)])
b1, b2, OPTS = 737, 962, ['layout', 'shape', 'both', 'explore']
fpath([(rt('score'), cy('score')), (b1, cy('score'))], arrow=False)
fpath([(b1, cy('layout')), (b1, cy('explore'))], arrow=False)
for o in OPTS:
    fpath([(b1, cy(o)), (POS[o][0], cy(o))])
    fpath([(rt(o), cy(o)), (b2, cy(o))], arrow=False)
fpath([(b2, cy('layout')), (b2, cy('explore'))], arrow=False)
fpath([(b2, cy('runs')), (POS['runs'][0], cy('runs'))])
fpath([(POS['runs'][0] + 60, POS['runs'][1] + POS['runs'][3]), (POS['runs'][0] + 60, POS['export'][1])])
fpath([(POS['score'][0] + 95, POS['score'][1] + POS['score'][3]), (POS['score'][0] + 95, POS['summary'][1])], arrow=False, dashed=True)
for k, (x, y, w, h) in POS.items():
    title, lines, state = BOX[k]
    derived = k in ('cost', 'freq', 'score')
    ax.add_patch(FancyBboxPatch((x, y), w, h, boxstyle='round,pad=0,rounding_size=6', fc=SOFT if derived else 'white', ec=HAIR, lw=.7, zorder=2))
    if state: ax.add_patch(plt.Rectangle((x, y + 6), 3, h - 12, fc=OK, ec='none', zorder=3))
    ax.text(x + 12, y + 21, title, fontsize=6.6, fontweight='bold', color=INK, va='baseline', zorder=3)
    for i, l in enumerate(lines[:int((h - 30) // 17)]):
        ax.text(x + 12, y + 40 + i * 17, l, fontsize=5.6, color=MUTED, va='baseline', zorder=3)
for x, t in [(16, 'Inputs'), (268, 'Compiled'), (520, 'Score'), (764, 'Optimize'), (984, 'Saved')]:
    ax.text(x, FH - 4, t, fontsize=5.8, color=token('faint'), va='baseline')
save(fig, 'flowchart')

# ---- Template figures: layout of a planned result, illustrative shapes only, watermarked. ----
def template(ax):
    ax.text(.5, .5, 'TEMPLATE\nillustrative, not data', transform=ax.transAxes, ha='center', va='center',
            fontsize=13, color=HAIR, alpha=.9, rotation=18, fontweight='bold', zorder=0)
GHOST = dict(color=MUTED, lw=.9)

# T1. Performance profiles / time-to-target ECDF (#45): fraction of (seed, instance) pairs that
#     reach a target within a time. Shapes are logistic curves, not measurements.
fig, ax = frame(2.0); template(ax)
t = np.logspace(-1, 2, 200)
for i, (name, mid, ls) in enumerate([('BLS', 2, '-'), ('SA', 3, '--'), ('Memetic', 6, '-.'), ('Tabu', 15, ':')]):
    ax.plot(t, 1 / (1 + (mid / t) ** 2.2), ls=ls, label=name, color=BLUE if i == 0 else MUTED, lw=.9)
ax.set_xscale('log')
ax.set(xlabel='Wall time (s)', ylabel='Runs within 0.5% of best known', ylim=(0, 1.02))
ax.legend(frameon=False, fontsize=6.2, loc='lower right')
save(fig, 'tpl-ttt')

# T2. Local-optima network (#100): nodes are optima, edges are kicks, size = basin.
fig, ax = plt.subplots(figsize=(3.4, 2.2), layout='constrained'); ax.axis('off'); template(ax)
g = np.random.default_rng(3)
pts = np.concatenate([g.normal(c, .45, (14, 2)) for c in [(-1.6, .2), (0, -.4), (1.7, .5)]])
for _ in range(60):
    a, b = g.integers(0, len(pts), 2)
    if np.linalg.norm(pts[a] - pts[b]) < 1.3:
        ax.plot(*zip(pts[a], pts[b]), color=HAIR, lw=.5, zorder=1)
ax.scatter(pts[:, 0], pts[:, 1], s=g.uniform(4, 30, len(pts)), color=MUTED, linewidths=0, zorder=2)
ax.scatter([0], [-.4], s=40, color=BLUE, zorder=3); ax.text(.12, -.75, 'funnel bottom', fontsize=5.8, color=INK)
save(fig, 'tpl-lon')

# T3. Robust design over hand sizes (#103): cost of each optimum across a hand-size range.
fig, ax = frame(2.0); template(ax)
h = np.linspace(160, 215, 60)
ax.plot(h, 1 + .0012 * (h - 175) ** 2, ls='--', **GHOST, label='optimum for 175 mm')
ax.plot(h, 1 + .0012 * (h - 200) ** 2, ls=':', **GHOST, label='optimum for 200 mm')
ax.plot(h, 1.12 + .0004 * (h - 188) ** 2, color=BLUE, lw=1, label='robust (mean over sizes)')
ax.set(xlabel='Hand length (mm)', ylabel='Relative score', ylim=(.9, 2.0))
ax.legend(frameon=False, fontsize=6, loc='upper center')
save(fig, 'tpl-handsizes')

# T4. Adaptive top-K n-gram scoring (#55): speed and score loss against K.
fig, ax = frame(2.0); template(ax)
kk = np.logspace(2, 4.3, 40)
ax.plot(kk, 6.4 * (2e4 / kk) ** .45, color=BLUE, lw=1, label='swap deltas / s (millions)')
ax.plot(kk, 3 * np.exp(-kk / 1500), ls='--', **GHOST, label='score loss after confirmation (%)')
ax.set_xscale('log'); ax.set(xlabel='Trigrams kept (K)', ylim=(0, 20))
ax.legend(frameon=False, fontsize=6, loc='upper right')
save(fig, 'tpl-topk')

# T5. Free-form 6-DoF refinement after parametric search (#70, #74): best score against boards.
fig, ax = frame(2.0); template(ax)
n = np.linspace(0, 1, 100)
ax.plot(n * 20, 7.5 + 4 * np.exp(-8 * n), **GHOST, ls='--', label='parametric (34 params)')
ax.plot(20 + n * 20, 7.5 - .9 * (1 - np.exp(-5 * n)), color=BLUE, lw=1, label='free-form, CMA-ES')
ax.plot(20 + n * 20, 7.5 - 1.0 * (1 - np.exp(-14 * n)), color=BLUE, lw=1, ls=':', label='free-form, surrogate gradient')
ax.set(xlabel='Boards evaluated (thousands)', ylabel='Best score', ylim=(6, 12))
ax.legend(frameon=False, fontsize=6, loc='upper right')
save(fig, 'tpl-freeform')

print('figures written to', HERE / 'assets')
