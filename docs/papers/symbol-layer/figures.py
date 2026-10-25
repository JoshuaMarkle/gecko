"""Figures for the symbol-layer study. Data: bench/symbol-layer-2026-10-25/ (scores.json, paths/,
tables/, langs.json) and data/layouts/layered/corne-sym-*.toml.
Run from anywhere: MPLCONFIGDIR=/tmp/gecko-mpl python3 -I docs/papers/symbol-layer/figures.py
"""
import json, sys
from pathlib import Path
import numpy as np
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'programming-layout'))
from corne import ROOT, INK, MUTED, HAIR, BLUE, GREY, ORANGE, SEQ, plt, board, freq_of, toml_rows, layers_of, design  # noqa: E402

BENCH = ROOT / 'bench/symbol-layer-2026-10-25'
(HERE / 'assets').mkdir(exist_ok=True)
S = json.load(open(BENCH / 'scores.json'))
TIME = 0.043  # % typing time per keystroke per score point (programming report)
REFS = {'getreuer': 'Getreuer v2', 'miryoku': 'Miryoku', 'neo3': 'Neo layer 3', 'ergol': 'Ergo-L', 'seniply': 'Seniply',
        'shelzuuz': 'ShelZuuz', 'beakl15': 'BEAKL 15', 'elixir': 'Elixir (Pomerleau)', 'sunaku': 'Sunaku (3×5 core)',
        'hietala': 'Hietala T-34', 'moonlander': 'Moonlander default', 'us-layer': 'US-style'}
MISSING = json.load(open(BENCH / 'layouts/refs.json'))


def save(fig, name):
    fig.savefig(HERE / 'assets' / (name + '.pdf'), metadata={'CreationDate': None, 'ModDate': None})
    plt.close(fig)


def sc(key):
    return S[key]['free']['score']


# 1. Ranking: published layers vs GECKO, digits on their own Num layer, < > on the layer (ltgt frames).
fig, axs = plt.subplots(1, 2, figsize=(6.3, 3.0), layout='constrained')
for ax, tag, title in ((axs[0], 'ltgt', 'programming'), (axs[1], 'ltgt-vim', 'Vim programming')):
    best = sc(f'layouts/best/{tag}-free.toml')
    rows = [(f'GECKO free optimum', sc(f'layouts/best/{tag}-free.toml'), BLUE),
            (f'GECKO human optimum', sc(f'layouts/best/{tag}-human.toml'), BLUE),
            (f'GECKO recommended (tuned)', sc(f'layouts/tuned/{tag}-tuned.toml'), ORANGE)]
    for k, n in REFS.items():
        miss = MISSING[f'{tag}/{k}']['missing'].replace('⏎', '')
        rows.append((n + (f'  (+{len(miss)} placed)' if miss else ''), sc(f'layouts/{tag}/{k}.c.toml'), '#c9ced4' if miss else GREY))
    rows.sort(key=lambda r: r[1])
    y = np.arange(len(rows))[::-1]
    gap = [(s - best) * TIME for _, s, _ in rows]
    ax.barh(y, gap, color=[c for *_, c in rows], height=.66)
    for yi, g, (_, s, _) in zip(y, gap, rows):
        ax.text(g + .012, yi, f'{s:.1f}', va='center', fontsize=5, color=MUTED)
    ax.set_yticks(y, [n for n, *_ in rows], fontsize=5.6)
    ax.set_xlim(0, max(gap) * 1.18)
    ax.grid(axis='x', color=HAIR, linewidth=.5, alpha=.65); ax.tick_params(length=0, labelsize=5.6)
    ax.set_title(title, fontsize=6.5, loc='left', color=INK)
    ax.set_xlabel('% time above the free optimum (score at bar end)', fontsize=5.6)
save(fig, 'ranking')

# 2. The recommended layers, shaded by keystroke share.
for name, corpus, title in (('corne-sym-prog', 'programmer-typed', 'Recommended programming layer'),
                            ('corne-sym-vim', 'vim-programmer-typed', 'Recommended Vim layer')):
    rows = toml_rows(ROOT / f'data/layouts/layered/{name}.toml')
    tag = 'ltgt' if name.endswith('prog') else 'ltgt-vim'
    save(design(rows, freq_of(corpus), f"{title} ({name}) · score {sc(f'layouts/tuned/{tag}-tuned.toml'):.2f}"), name)

# 3. Published layers as encoded (ltgt frame, symbols a design lacks placed by search), Sym layer only.
F = freq_of('programmer-typed')
show = ['getreuer', 'neo3', 'ergol', 'shelzuuz', 'seniply', 'miryoku']
fig, axs = plt.subplots(3, 2, figsize=(6.3, 4.9), layout='constrained')
for ax, k in zip(axs.flat, show):
    rows = toml_rows(BENCH / f'layouts/ltgt/{k}.c.toml')
    sym = layers_of(rows)[1]
    base = layers_of(rows)[0]
    board(ax, sym, f'{REFS[k]} · {sc(f"layouts/ltgt/{k}.c.toml"):.1f}', shade=[F.get(c, 0) for c in sym],
          vmax=max(F.get(c, 0) for c in sym), hold=base.index('①'))
save(fig, 'published')

# 4. The choices that matter: each configuration's best human layout vs the recommended one.
fig, axs = plt.subplots(1, 2, figsize=(6.3, 2.1), layout='constrained')
for ax, (v, title) in zip(axs, (('', 'programming'), ('-vim', 'Vim programming'))):
    ref = sc(f'layouts/tuned/ltgt{v}-tuned.toml')
    cfg = [('< > on Shift+, . (human)', f'layouts/best/us{v}-human.toml', GREY),
           ('recommended: < > on Sym, tuned', f'layouts/tuned/ltgt{v}-tuned.toml', ORANGE),
           ('human optimum', f'layouts/best/ltgt{v}-human.toml', BLUE),
           ('no rules (free optimum)', f'layouts/best/ltgt{v}-free.toml', BLUE),
           ('digits on Sym, numpad (human)', f'layouts/best/ltgt{v}-digits-pad-human.toml', '#7fb08a'),
           ('digits on Sym, home row (human)', f'layouts/best/ltgt{v}-digits-row-human.toml', '#7fb08a'),
           ('digits on Sym, no rules', f'layouts/best/ltgt{v}-digits-{"row" if not v else "pad"}-free.toml', '#7fb08a')]
    y = np.arange(len(cfg))[::-1]
    d = [(sc(p) - ref) * TIME for _, p, _ in cfg]
    ax.barh(y, d, color=[c for *_, c in cfg], height=.62)
    ax.axvline(0, color=INK, lw=.6)
    for yi, di in zip(y, d):
        ax.text(di + (.004 if di >= 0 else -.004), yi, f'{di:+.3f}', va='center', ha='left' if di >= 0 else 'right', fontsize=5, color=MUTED)
    ax.set_yticks(y, [n for n, *_ in cfg], fontsize=5.6)
    ax.set_xlim(min(d) * 1.45, max(d) * 1.6 + .02)
    ax.grid(axis='x', color=HAIR, linewidth=.5, alpha=.65); ax.tick_params(length=0, labelsize=5.6)
    ax.set_title(title, fontsize=6.5, loc='left'); ax.set_xlabel('% typing time vs the recommended layer', fontsize=5.6)
save(fig, 'choices')

# 5. Flatness: every single swap from the human optimum, and greedy edits of published layers.
fig, axs = plt.subplots(1, 2, figsize=(6.3, 2.0), layout='constrained')
for f, lab, col in (('ltgt-human-free', 'programming', BLUE), ('ltgt-vim-human-free', 'Vim', ORANGE)):
    t = json.load(open(BENCH / f'tables/{f}.json'))
    d = np.array([r['delta'] for r in t['swaps']]) * TIME
    axs[0].hist(np.clip(d, -0.01, 0.2), bins=np.linspace(-0.01, 0.2, 43), histtype='step', color=col, lw=1, label=f'{lab} ({len(d)} swaps)')
axs[0].set_xlabel('% typing time added by one swap (clipped at 0.2)', fontsize=5.6); axs[0].set_ylabel('swaps', fontsize=5.6)
axs[0].legend(fontsize=5.4); axs[0].tick_params(length=0, labelsize=5.6); axs[0].grid(color=HAIR, linewidth=.5, alpha=.65)
best = sc('layouts/best/ltgt-free.toml')
for k, col in (('getreuer', GREY), ('neo3', BLUE), ('hietala', ORANGE)):
    p = json.load(open(BENCH / f'paths/ltgt-{k}-free.json'))['path']
    axs[1].plot([q['step'] for q in p], [(q['score'] - best) * TIME for q in p], '-o', ms=2.2, lw=1, color=col, label=REFS[k])
for v, lab in ((sc('layouts/best/ltgt-human.toml'), 'human optimum'), (sc('layouts/tuned/ltgt-tuned.toml'), 'recommended')):
    axs[1].axhline((v - best) * TIME, color=MUTED, lw=.6, ls='--')
    axs[1].text(6.2, (v - best) * TIME + .001, lab, fontsize=5, color=MUTED, va='bottom')
axs[1].set_xlim(-.2, 8.6); axs[1].set_ylim(0, None)
axs[1].set_xlabel('greedy swaps (programming, ltgt frame)', fontsize=5.6); axs[1].set_ylabel('% time above free optimum', fontsize=5.6)
axs[1].legend(fontsize=5.4, loc='upper right'); axs[1].tick_params(length=0, labelsize=5.6); axs[1].grid(color=HAIR, linewidth=.5, alpha=.65)
save(fig, 'swaps')

# 6. Digits on the Sym layer (no Num layer for digits): the human optima, Sym layer only.
fig, axs = plt.subplots(2, 2, figsize=(6.3, 3.4), layout='constrained')
for row, (f, corpus, title) in zip(axs, (('ltgt-digits-row-human', 'programmer-typed', 'programming, digits on the home row'),
                                         ('ltgt-vim-digits-pad-human', 'vim-programmer-typed', 'Vim, digits as a numpad'))):
    Fd = freq_of(corpus)
    lys = layers_of(toml_rows(BENCH / f'layouts/best/{f}.toml'))
    for ax, i, lab in ((row[0], 1, f'{title} · {sc(f"layouts/best/{f}.toml"):.2f}'), (row[1], 2, 'layer ② (overflow)')):
        board(ax, lys[i], lab, shade=[Fd.get(c, 0) for c in lys[i]], vmax=max(Fd.get(c, 0) for c in lys[1]),
              hold=lys[0].index('①' if i == 1 else '②'))
save(fig, 'digits')

# 7. Languages: gap to the best of these layouts per typed-code corpus (ltgt frame).
Lg = json.load(open(BENCH / 'langs.json'))['ltgt']
cols = ['code-python-typed', 'code-rust-typed', 'code-js-typed', 'code-c-typed', 'code-shell-typed', 'code-lua-typed', 'markdown']
names = {'ltgt-free.toml': 'GECKO free optimum', 'ltgt-human.toml': 'GECKO human optimum', 'ltgt-tuned.toml': 'recommended (tuned)'}
names.update({f'{k}.c.toml': n for k, n in REFS.items()})
order = sorted(Lg, key=lambda n: sum(Lg[n][c] for c in cols))
M = np.array([[(Lg[n][c] - min(Lg[m][c] for m in Lg)) * TIME for c in cols] for n in order])
fig, ax = plt.subplots(figsize=(4.6, 2.6), layout='constrained')
VM = 0.4
ax.imshow(np.minimum(M, VM), cmap=SEQ, vmin=0, vmax=VM, aspect='auto')
for i in range(M.shape[0]):
    for j in range(M.shape[1]):
        ax.text(j, i, f'{M[i, j]:.2f}', ha='center', va='center', fontsize=5, color='white' if M[i, j] > .8 * VM else INK)
ax.set_xticks(range(len(cols)), [c.replace('code-', '').replace('-typed', '') for c in cols])
ax.set_yticks(range(len(order)), [names[n] for n in order])
ax.tick_params(length=0, labelsize=5.6); [s.set_visible(False) for s in ax.spines.values()]
ax.set_xlabel('% typing time above the best of these layouts on each corpus', fontsize=5.6)
save(fig, 'langs')
print('figures written to', HERE / 'assets')
