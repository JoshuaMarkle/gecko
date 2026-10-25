"""Figures for the programming layout study. Data: bench/programming-layout-2026-10-25/.
Run from anywhere: MPLCONFIGDIR=/tmp/gecko-mpl python3 -I docs/papers/programming-layout/figures.py
"""
import json, math, sys
from pathlib import Path
import numpy as np
sys.path.insert(0, str(Path(__file__).resolve().parent))
from corne import ROOT, INK, MUTED, HAIR, BLUE, GREY, ORANGE, SEQ, plt, board, freq_of, toml_rows, design  # noqa: E402

HERE = Path(__file__).resolve().parent
BENCH = ROOT / 'bench/programming-layout-2026-10-25'
(HERE / 'assets').mkdir(exist_ok=True)


def save(fig, name):
    fig.savefig(HERE / 'assets' / (name + '.pdf'), metadata={'CreationDate': None, 'ModDate': None})
    plt.close(fig)


def frame(h, w=6.3):
    fig, ax = plt.subplots(figsize=(w, h), layout='constrained')
    ax.grid(color=HAIR, linewidth=.5, alpha=.65)
    ax.tick_params(length=0, pad=3, labelsize=6.5)
    return fig, ax


# 1. Calibration: what a held Shift adds to the interval into a capital, per letter, vs an SFB.
S = json.load(open(BENCH / 'shift.json'))
C = {tuple(x['key']): x for x in S['cells']}
rows = []
for ch in 'abcdefghijklmnopqrstuvwxyz':
    a, b = C.get(('into', ch, True)), C.get(('into', ch, False))
    if a and b and a['n'] > 200:
        rows.append((ch, a['mean'] - b['mean'], 1.96 * math.sqrt(a['var'] / a['n'] + b['var'] / b['n']), a['n']))
rows.sort(key=lambda r: r[1])
fig, ax = frame(2.4)
x = np.arange(len(rows))
ax.bar(x + .0, [r[1] for r in rows], color=BLUE, width=.65, yerr=[r[2] for r in rows], error_kw=dict(lw=.6, ecolor=MUTED))
w = sum(r[1] * r[3] for r in rows) / sum(r[3] for r in rows)
ax.axhline(w, color=INK, lw=.7); ax.text(-.3, w + .03, f'Shift, weighted mean +{w:.2f} (×{math.exp(w):.2f})', ha='left', fontsize=6, bbox=dict(fc='white', ec='none', pad=1))
ax.axhline(math.log(1.17), color=ORANGE, lw=.7, ls='--')
ax.text(len(rows) - .5, math.log(1.17) - .07, 'one SFB pair +0.157 (×1.17)', ha='right', fontsize=6, color=ORANGE, bbox=dict(fc='white', ec='none', pad=1))
ax.set_xticks(x, [r[0].upper() for r in rows]); ax.set_xlim(-.6, len(rows) - .4)
ax.set_ylabel('added log interval into the letter')
ax.set_xlabel('capital after a space vs the same letter lowercase (56,358 typists, 95 % CI)')
save(fig, 'shift')

print('figures written to', HERE / 'assets')


# 2. Keyboards: base and symbol layers of the chosen designs, shaded by keystroke share.
R = json.load(open(BENCH / 'results.json'))
F = freq_of('programmer-typed')


best = R['cells']['long']
save(design(best['rows'], F, f"GECKO optimum (best of 3 × 160 M evaluations) · score {best['best']:.2f}"), 'best')
dr = R['cells']['digits-row']
save(design(dr['rows'], F, f"GECKO optimum with digits 1–0 in a row · score {dr['best']:.2f}"), 'digits')
save(design(toml_rows(BENCH / 'layouts/getreuer.toml'), F,
            f"Getreuer v2 symbol layer on Colemak-DH · score {R['layouts']['ref:getreuer']['scores']['programmer-typed']:.2f}"), 'getreuer')
save(design(toml_rows(BENCH / 'layouts/miryoku.toml'), F,
            f"Miryoku on Colemak-DH · score {R['layouts']['ref:miryoku']['scores']['programmer-typed']:.2f}"), 'miryoku')
print('keyboards written')


# 3. Ranking on programmer-typed: % above the optimum, and the same gap as typing time per keystroke.
L = R['layouts']
NAME = {'long': 'GECKO optimum', 'full-cal': 'optimum, 40 M evaluations', 'digits-row': 'optimum, digits in a row', 'layer-cal': 'optimized layers on Colemak-DH',
        'full-code': 'optimum on code only', 'full-raw': 'optimum on raw code (files)', 'full-old': 'optimum, old layer cost (1)',
        'oneshot': 'optimum re-searched with one-shot layers', 'ref:getreuer': 'Getreuer v2', 'ref:us-layer': 'US-style layer',
        'ref:miryoku': 'Miryoku'}
KIND = lambda n: GREY if n.startswith('ref:') else (ORANGE if n == 'long' else BLUE)
TIME = 0.157 / 3.626  # % time per keystroke per score point (SFB +0.157 log interval = 3.626 points per SFB %)
rows = sorted((n for n in L if n in NAME and not n.startswith('lang-')), key=lambda n: L[n]['scores']['programmer-typed'])
b0 = L['long']['scores']['programmer-typed']
fig, ax = frame(2.6)
y = np.arange(len(rows))[::-1]
gap = [100 * (L[n]['scores']['programmer-typed'] / b0 - 1) for n in rows]
ax.barh(y, gap, color=[KIND(n) for n in rows], height=.62)
for yi, g, n in zip(y, gap, rows):
    s = L[n]['scores']['programmer-typed']
    ax.text(max(g, 0) + .5, yi, f"{s:.2f}  ·  ≈ {(s - b0) * TIME:+.2f} % time",
            va='center', fontsize=5.4, color=MUTED)
ax.set_yticks(y, [NAME[n] for n in rows]); ax.set_xlim(0, 40)
ax.set_xlabel('% above the GECKO optimum on programmer-typed under prog-cal (lower is better)')
save(fig, 'ranking')

# 4. Cross-corpus: each design (rows) on each corpus (columns), % above the best of these designs on that corpus.
CS = ['programmer-typed', 'code-typed', 'code-rust-typed', 'code-python-typed', 'code-js-typed', 'prose', 'programmer']
rows = [n for n in ('long', 'full-code', 'lang-rust', 'lang-python', 'lang-js', 'full-raw', 'layer-cal',
                    'ref:getreuer', 'ref:us-layer', 'ref:miryoku') if n in L]
LBL = dict(NAME, **{f'lang-{l}': f'optimum on {l} code' for l in ('rust', 'python', 'js')})
bc = {c: min(L[n]['scores'][c] for n in rows) for c in CS}
M = np.array([[100 * (L[n]['scores'][c] / bc[c] - 1) for c in CS] for n in rows])
fig, ax = plt.subplots(figsize=(6.3, 2.5), layout='constrained')
VM = 40
ax.imshow(np.minimum(M, VM), cmap=SEQ, vmin=0, vmax=VM, aspect='auto')
for i in range(M.shape[0]):
    for j in range(M.shape[1]):
        ax.text(j, i, f'{M[i, j]:.1f}', ha='center', va='center', fontsize=5.6, color='white' if M[i, j] > .55 * VM else INK)
ax.set_xticks(range(len(CS)), ['programmer', 'all code', 'Rust', 'Python', 'JS/TS', 'prose', 'programmer\n(code files)'])
ax.set_yticks(range(len(rows)), [LBL[n] for n in rows])
ax.tick_params(length=0, labelsize=6); [s.set_visible(False) for s in ax.spines.values()]
ax.axhline(rows.index('ref:getreuer') - .5, color='white', lw=2)
ax.set_xlabel('scored on corpus, code as typed except the last column (% above the best of these designs there)')
save(fig, 'corpora')

# 5. What editor typing changes: symbol shares in code files vs code as typed.
raw, typ = freq_of('code-all'), freq_of('code-typed')
syms = sorted((c for c in raw if not c.isalnum() and c not in '␣⏎⌫⇥\u0001' and raw[c] > 2), key=lambda c: -raw[c])[:24]
fig, ax = frame(2.2)
x = np.arange(len(syms))
ax.bar(x - .2, [raw[c] for c in syms], width=.4, color=GREY, label='code files (code-all)')
ax.bar(x + .2, [typ.get(c, 0) for c in syms], width=.4, color=BLUE, label='as typed (code-typed: auto-closed line closers, identifiers completed after 4)')
ax.set_xticks(x, syms, fontsize=7.5); ax.set_xlim(-.6, len(syms) - .4)
ax.set_ylabel('share of keystrokes (‰)'); ax.legend(loc='upper right')
save(fig, 'typed')
print('ranking, corpora, typed written')
