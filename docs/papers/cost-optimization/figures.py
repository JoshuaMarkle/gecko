"""Figures for the cost-function optimization paper. Data: bench/costfit-2026-10-24/results/ (outputs of
that folder's scripts) and data/profiles/.
Run from anywhere: MPLCONFIGDIR=/tmp/gecko-mpl python3 docs/papers/cost-optimization/figures.py
"""
import json, re, tomllib
from pathlib import Path
import numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
RES = ROOT / 'bench/costfit-2026-10-24/results'
J = lambda name: json.loads((RES / name).read_text())
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

def frame(h=2.2, w=3.4):
    fig, ax = plt.subplots(figsize=(w, h), layout='constrained')
    ax.grid(color=HAIR, linewidth=.5, alpha=.65)
    ax.tick_params(length=0, pad=3, labelsize=6.5)
    return fig, ax
def save(fig, name):
    fig.savefig(HERE / 'assets' / (name + '.pdf'), metadata={'CreationDate': None, 'ModDate': None})
    plt.close(fig)

# 1. Timing coefficients with 90 % bootstrap intervals, as % change of the interval.
C = J('final_coefs.json')
REWARD = {'B08', 'T03', 'T04', 'T01'}
NAMES = {'B01': 'SFB', 'B05': 'scissors', 'B06': 'half scissors', 'B08': 'hand alternation', 'B09': 'pinky–ring',
         'B10': 'row jump (per key)', 'B14': 'stretch (per unit)', 'E05': 'Engram discomfort', 'M01': 'key effort (per unit)',
         'T03': 'roll in', 'T04': 'roll out', 'T07': 'bad redirect', 'S01': 'same-finger skipgram', 'S04': 'skip stretch',
         'T01': 'trigram alternate', 'T06': 'redirect'}
rows = []
for part, lab in (('pairs', 'pair'), ('trigrams', '3rd key')):
    for m, b in C[part]['beta'].items():
        lo, hi, _ = C[part]['ci'][m]; s = -1 if m in REWARD else 1
        pct = lambda x: 100 * (np.exp(s * x) - 1)
        rows.append((f"{NAMES[m]} ({lab})", pct(b), sorted([pct(lo), pct(hi)])))
rows.sort(key=lambda r: r[1])
fig, ax = frame(2.6)
y = np.arange(len(rows))
ax.errorbar([r[1] for r in rows], y, xerr=[[r[1] - r[2][0] for r in rows], [r[2][1] - r[1] for r in rows]],
            fmt='o', ms=2.5, color=BLUE, ecolor=GREY, elinewidth=.8, capsize=0)
ax.axvline(0, color=MUTED, lw=.6)
ax.set_yticks(y, [r[0] for r in rows]); ax.set_xlabel('effect on keystroke interval (%)')
save(fig, 'coefs')

# 2. Coverage: share of a layout's bigram mass on key pairs QWERTY typists type < 30 times.
cov = J('coverage.json')['refs']
order = sorted(cov, key=cov.get)
fig, ax = frame(1.9)
ax.bar(range(len(order)), [100 * cov[k] for k in order], color=[GREY if k == 'qwerty' else BLUE for k in order], width=.65)
ax.set_xticks(range(len(order)), order, rotation=35, ha='right'); ax.set_ylabel('bigram mass on rare pairs (%)')
save(fig, 'coverage')

# 3. Speed/comfort frontier: re-evaluated candidates of the weight search, plus references.
pool = J('s7_pool.json'); s8 = J('s8_judges.json'); sdT = J('s6_a50_best.json')['sdT']
fig, ax = frame(2.5)
pts = [(r['T'] * sdT * 100, r['E'] * 1.206, k) for k, r in pool.items()]
ax.scatter([p[0] for p in pts if p[2] != 'default'], [p[1] for p in pts if p[2] != 'default'], s=9, color=BLUE, alpha=.55,
           label='optima of searched weights')
d = pool['default']; ch = pool['a75:top2']
ax.scatter([d['T'] * sdT * 100], [d['E'] * 1.206], s=26, marker='s', color=GREY, label='optimum of previous default')
ax.scatter([ch['T'] * sdT * 100], [ch['E'] * 1.206], s=40, marker='*', color=ORANGE, label='optimum of new default')
for k in ('graphite', 'galite-reference', 'sturdy', 'semimak', 'canary', 'dvorak'):
    r = s8[k]; ax.scatter([r['T_B']], [r['E05']], s=8, marker='x', color=INK, lw=.7)
    ax.annotate(k, (r['T_B'], r['E05']), fontsize=5.5, color=MUTED, xytext=(3, -7 if k == 'galite-reference' else 1), textcoords='offset points')
ax.set_xlabel('time model, half B (% per keystroke, lower = faster)'); ax.set_ylabel('Engram discomfort E05')
ax.legend(loc='upper left')
save(fig, 'frontier')

# 4. CMA-ES convergence: best-so-far objective per generation, each run with its own alpha.
fig, ax = frame(1.9)
for tag, lab, col in (('a25', 'α 0.25', GREY), ('a50', 'α 0.5', BLUE), ('a75', 'α 0.75', ORANGE), ('a50free', 'α 0.5, no tier limit', INK)):
    log = J(f's6_{tag}.log.json'); g = max(r['gen'] for r in log) + 1
    best = np.minimum.accumulate([min(r['f'] for r in log if r['gen'] == k) for k in range(g)])
    ax.plot(range(g), best - best[0], color=col, lw=1, label=lab)
ax.set_xlabel('generation (16 inner optimizations each)'); ax.set_ylabel('best objective − generation 0')
ax.legend()
save(fig, 'cma')

# 5. Weights: previous default (balanced) vs new default, spread units.
old = tomllib.loads((ROOT / 'data/profiles/balanced.toml').read_text())['weights']
new = tomllib.loads((ROOT / 'data/profiles/default.toml').read_text())['weights']
ms = sorted(set(old) | set(new), key=lambda m: -max(old.get(m, 0), new.get(m, 0)))
fig, ax = frame(2.0, 7.0)
x = np.arange(len(ms)); bw = .38
ax.bar(x - bw / 2, [old.get(m, 0) for m in ms], bw, color=GREY, label='previous default (balanced)')
ax.bar(x + bw / 2, [new.get(m, 0) for m in ms], bw, color=BLUE, label='new default')
ax.set_xticks(x, ms); ax.set_ylabel('weight (spread units)'); ax.legend()
save(fig, 'weights')
print('figures written to', HERE / 'assets')
