"""Figures for the cost-function validity paper. Data: bench/cost-validity-2026-10-22/results.json
(aggregates produced by that folder's scripts; no synthetic values).
Run from anywhere: MPLCONFIGDIR=/tmp/gecko-mpl python3 docs/papers/cost-validity/figures.py
"""
import json, re
from pathlib import Path
import numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
R = json.loads((ROOT / 'bench/cost-validity-2026-10-22/results.json').read_text())
STYLE = (ROOT / 'web/src/styles.css').read_text()
def token(name):
    return re.search(r'--' + name + r':\s*(#[0-9a-fA-F]+)', STYLE)[1]
INK, MUTED, HAIR, BLUE = [token(n) for n in ('ink', 'muted', 'hair', 'accent')]
GREY = '#9aa3ad'
plt.rcParams.update({'font.family': 'DejaVu Sans', 'font.size': 6.5, 'text.color': INK,
    'axes.labelcolor': MUTED, 'xtick.color': MUTED, 'ytick.color': MUTED,
    'axes.edgecolor': HAIR, 'axes.spines.top': False, 'axes.spines.right': False,
    'pdf.fonttype': 42, 'axes.axisbelow': True, 'legend.frameon': False, 'legend.fontsize': 6})

def frame(h=2.2, w=3.4):
    fig, ax = plt.subplots(figsize=(w, h), layout='constrained')
    ax.grid(color=HAIR, linewidth=.5, alpha=.65)
    ax.tick_params(length=0, pad=3, labelsize=6.5)
    return fig, ax
def save(fig, name):
    fig.savefig(HERE / 'assets' / (name + '.pdf'), metadata={'CreationDate': None, 'ModDate': None})
    plt.close(fig)

# 1. Variance share of the score by metric, per layout set.
V = R['varshare']; sets = [('random', 'random (3000)'), ('cmini', 'community (1904)'), ('good', f"good community ({V['n_good']})"), ('refs', 'references (10)')]
mets = ['B01', 'B14', 'E05', 'S01', 'B11', 'T06', 'S02']
fig, ax = frame(2.1)
x = np.arange(len(mets)); bw = .2
for k, (s, lab) in enumerate(sets):
    ax.bar(x + (k - 1.5) * bw, [100 * V[s][m] for m in mets], bw, label=lab,
           color=[GREY, '#c3cad2', BLUE, INK][k], linewidth=0)
ax.set_xticks(x, mets); ax.set_ylabel('share of score variance (%)')
ax.legend(ncol=2, loc='upper right')
save(fig, 'varshare')

# 2. Timing: binned partial residuals, GECKO pair cost vs relative IKI, by typist group.
T = R['timing']; P = R['participants']
fig, ax = frame(2.2)
for g, col, lab in (('touch_fast', BLUE, f"touch, ≥70 WPM ({P['touch_fast']:,} people)"), ('touch', INK, f"touch ({P['touch']:,})"), ('few', GREY, f"1–4 fingers ({P['few']:,})")):
    b = np.array(T[g]['bins'])
    ax.errorbar(b[:, 0], b[:, 1], yerr=1.96 * b[:, 2], color=col, marker='o', ms=2.5, lw=.9, capsize=1.5,
                label=f"{lab}: ρ={T[g]['partial']:+.2f}")
ax.set_xlabel('GECKO pair cost (residual after log bigram frequency)')
ax.set_ylabel('relative IKI (residual)')
ax.legend(loc='lower right')
save(fig, 'timing')

# 3. Weight robustness of the reference ranking.
M = R['robust']['mc']; sig = [m['sigma'] for m in M]
fig, ax = frame(2.0)
ax.plot(sig, [m['p_classic'] for m in M], color=INK, marker='o', ms=2.5, lw=.9, label='every classic beats QWERTY')
ax.plot(sig, [m['p_modern'] for m in M], color=BLUE, marker='o', ms=2.5, lw=.9, label='every modern beats every classic')
ax.plot(sig, [m['tau_med'] for m in M], color=GREY, marker='s', ms=2.5, lw=.9, ls='--', label='median Kendall τ vs default order')
pf = R['robust']['prior_free']
ax.scatter([2.35], [pf['p_modern']], color=BLUE, marker='D', s=10); ax.scatter([2.35], [pf['p_classic']], color=INK, marker='D', s=10)
ax.text(2.33, pf['p_modern'] + .05, 'equal\nweights', fontsize=5.5, color=MUTED, ha='right')
ax.set_xlabel('σ of log-normal weight perturbation'); ax.set_ylabel('probability / τ'); ax.set_ylim(0, 1.05)
ax.legend(loc='lower left')
save(fig, 'robust')

# 4. Weight sweep: re-optimized layouts vs the default optimum.
fig, ax = frame(2.2)
for s, col, lab in (('mc1.0', GREY, 'σ = 1.0'), ('mc0.5', BLUE, 'σ = 0.5')):
    if s not in R['mc']: continue
    pts = R['mc'][s]['points']
    ax.scatter([p['slots'] for p in pts], [max(p['regret'], 1e-3) for p in pts], s=6, color=col, alpha=.75, linewidths=0, label=f"{lab} (n={len(pts)})")
oat = R['oat']
for k, v in oat.items():
    d = v.get('drop')
    if d and d['regret'] > .05:
        ax.scatter([d['slots']], [d['regret']], s=12, marker='x', color=INK, linewidths=.8)
        if k in ('B01', 'B14', 'E05'):
            ax.annotate(f'drop {k}', (d['slots'], d['regret']), fontsize=5.5, color=MUTED, xytext=(3, 1), textcoords='offset points')
ax.set_yscale('log'); ax.set_xlabel('keys moved vs default optimum (mirror-invariant, of 30)')
ax.set_ylabel('regret (points)'); ax.scatter([], [], s=12, marker='x', color=INK, label='one metric dropped'); ax.legend(loc='upper left')
save(fig, 'sweep')

# 5. Weight shares: default profile vs fitted to the time judge.
F = R['timefit']; keys = sorted(set(k for k in F['share'] if F['share'][k] > .004 or F['default_share'].get(k, 0) > .004),
                               key=lambda k: -(F['default_share'].get(k, 0) + F['share'][k]))
fig, ax = frame(2.6)
y = np.arange(len(keys))
ax.barh(y + .2, [100 * F['default_share'].get(k, 0) for k in keys], .4, color=GREY, label='default profile', linewidth=0)
ax.barh(y - .2, [100 * F['share'][k] for k in keys], .4, color=BLUE, label=f"fitted to time judge (ρ={F['rho']:.3f})", linewidth=0)
ax.set_yticks(y, keys); ax.invert_yaxis(); ax.set_xlabel('share of total weight (%)'); ax.legend(loc='lower right')
save(fig, 'weights')

# 6. SFB vs stretch among good layouts.
S = R['sfb_stretch']
fig, ax = frame(2.2)
ax.scatter([s['B01'] for s in S], [s['B14'] for s in S], s=3, color=GREY, alpha=.5, linewidths=0, label='community layouts, score < 30')
for s in S:
    if not s['name'].startswith('cmini:') and s['B01'] < 2:
        ax.scatter([s['B01']], [s['B14']], s=10, color=INK); ax.annotate(s['name'], (s['B01'], s['B14']), fontsize=5.5, xytext=(3, 2), textcoords='offset points')
d = R['default_opt']['raw']; ax.scatter([d['B01']], [d['B14']], s=14, color=BLUE, marker='*')
ax.annotate('GECKO optimum', (d['B01'], d['B14']), fontsize=5.5, color=BLUE, xytext=(3, -6), textcoords='offset points')
ax.set_xlabel('SFB (%)'); ax.set_ylabel('stretch B14 (u per 100 bigrams)'); ax.set_xlim(0.3, 2.0); ax.legend(loc='upper right')
save(fig, 'tradeoff')

# Generated LaTeX: tables and numbers that depend on the sweep and the profile runs.
def f(x, d=2): return f'{x:.{d}f}'
G = []
NAMES = {'B01': 'SFB', 'S01': 'SFS', 'S02': 'far SFS', 'B03': 'finger speed', 'B14': 'stretch', 'B11': 'movement',
         'S04': 'skip stretch', 'M01': 'effort', 'M02': 'load error', 'M04': 'hand balance', 'T03': 'rolls in',
         'T04': 'rolls out', 'T01': 'alternates', 'T06': 'redirects', 'T07': 'bad redirects', 'E05': 'Engram comfort'}
rows = []
for k, v in R['oat'].items():
    def cell(t):
        x = v.get(t)
        return '--' if x is None else f"{x['slots']} / {f(x['regret'], 3)}"
    d = v.get('drop', {})
    rows.append(f"{k} & {cell('drop')} & {cell('half')} & {cell('double')} & {f(d.get('B01', float('nan')))} & {f(d.get('dtime', float('nan')), 2)}\\\\")
G.append('\\newcommand{\\oattable}{' + '\n'.join(rows) + '}')
J = R.get('judges', [])
jr = []
for j in J:
    jr.append(f"{j['name'].replace('_', '-')} & {f(j['default'])} & {f(100 * j['time_pct'], 0)} & {f(j['B01'])} & {f(j['B14'])} & {f(j['E05'])} & {f(j['M05'], 1)}\\\\")
G.append('\\newcommand{\\judgetable}{' + '\n'.join(jr) + '}')
mc = R['mc']
def mcstat(s):
    if s not in mc: return ['--'] * 6
    P = mc[s]['points']; sl = np.array([p['slots'] for p in P]); rg = np.array([p['regret'] for p in P]); dt = np.array([p['dtime'] for p in P])
    return [str(mc[s]['n']), str(mc[s]['distinct']), f(np.median(sl), 0), f(np.median(rg)), f(np.percentile(rg, 90)), f(np.mean(dt < 0) * 100, 0)]
for s, tag in (('mc0.5', 'A'), ('mc1.0', 'B')):
    for name, val in zip(('N', 'Distinct', 'Slots', 'Regret', 'RegretNinety', 'Faster'), mcstat(s)):
        G.append(f'\\newcommand{{\\mc{tag}{name}}}{{{val}}}')
sa = R['seed_agree']; G.append(f"\\newcommand{{\\seedagree}}{{{sa.get('true', sa.get('True', 0))}}}\\newcommand{{\\seedtotal}}{{{sum(sa.values())}}}")
(HERE / 'generated.tex').write_text('% Generated by figures.py from bench/cost-validity-2026-10-22/results.json. Do not edit.\n' + '\n'.join(G) + '\n')
print('figures and generated.tex written')
