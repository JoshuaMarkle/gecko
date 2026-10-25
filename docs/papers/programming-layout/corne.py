"""Shared drawing for corne-36 layered layouts (programming and Vim reports)."""
import json, re, tomllib
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.colors import LinearSegmentedColormap
from matplotlib.patches import FancyBboxPatch

ROOT = Path(__file__).resolve().parents[3]
STYLE = (ROOT / 'web/src/styles.css').read_text()
def token(name):
    return re.search(r'--' + name + r':\s*(#[0-9a-fA-F]+)', STYLE)[1]
INK, MUTED, HAIR, BLUE = [token(n) for n in ('ink', 'muted', 'hair', 'accent')]
GREY = '#9aa3ad'; ORANGE = '#d9822b'; PAPER = '#f2f2f0'
plt.rcParams.update({'font.family': 'DejaVu Sans', 'font.size': 6.5, 'text.color': INK,
    'axes.labelcolor': MUTED, 'xtick.color': MUTED, 'ytick.color': MUTED,
    'axes.edgecolor': HAIR, 'axes.spines.top': False, 'axes.spines.right': False,
    'pdf.fonttype': 42, 'axes.axisbelow': True, 'legend.frameon': False, 'legend.fontsize': 6})
SEQ = LinearSegmentedColormap.from_list('s', ['#ffffff', '#cfe0ff', BLUE])

GEOM = tomllib.load(open(ROOT / 'data/geometries/corne-36.toml', 'rb'))['key']
POS = [(k['pos'][0], k['pos'][1]) for k in GEOM]
GLYPH = {'␣': 'spc', '⌫': 'bsp', '⏎': 'ent', '⇧': 'shift', '①': 'sym', '②': 'num', '⎋': 'esc', '⇥': 'tab', '·': ''}
LAYER_KEYS = {'①': 1, '②': 2}


def freq_of(corpus):
    """Per-symbol share (per mille) of a corpus, from `gecko corpus info --json` if available."""
    import subprocess
    out = subprocess.run([str(ROOT / 'target/release/gecko'), '--json', 'corpus', 'info', corpus],
                         capture_output=True, text=True, cwd=ROOT, check=True).stdout
    d = json.loads(out)
    return {x['symbol']: x['share'] for x in d['symbols_per_mille']}


def layer_slots(chars, n=36):
    return [chars[i:i + n] for i in range(0, len(chars), n)]


def board(ax, keys, title=None, shade=None, vmax=None, hold=None, sub=None):
    """keys: 36 glyphs. shade: per-key value (frequency) or None. hold: slot of the held layer key."""
    vmax = vmax or (max(shade) if shade else 1) or 1
    for i, ((x, y), c) in enumerate(zip(POS, keys)):
        v = shade[i] / vmax if shade else 0
        face = '#e8e8e6' if i == hold else SEQ(min(v, 1))
        ax.add_patch(FancyBboxPatch((x + .06, -y - .94), .88, .88, boxstyle='round,pad=0,rounding_size=.12',
                                    facecolor=face, edgecolor=HAIR, lw=.6))
        g = GLYPH.get(c, c)
        g = g.upper() if len(g) == 1 and g.isalpha() else g
        dark = v > .6
        ax.text(x + .5, -y - .47, g, ha='center', va='center', fontsize=7.5 if len(g) == 1 else 5,
                color='white' if dark else (MUTED if len(g) > 1 else INK))
    ax.set_xlim(-.1, 13.1); ax.set_ylim(-4.9, .45 if title else .05); ax.set_aspect('equal'); ax.axis('off')
    if title:
        ax.text(0, .2, title, fontsize=7, color=INK, fontweight='bold', va='bottom')
    if sub:
        ax.text(13, .2, sub, fontsize=6, color=MUTED, va='bottom', ha='right')


def layers_of(rows):
    """Run rows (4 per layer: 10, 10, 10, 6) → one 36-glyph string per layer."""
    return [''.join(rows[i:i + 3]) + rows[i + 3] for i in range(0, len(rows), 4)]


TOK = {'{enter}': '⏎', '{space}': '␣', '{bspc}': '⌫', '{shift}': '⇧', '{layer1}': '①', '{layer2}': '②', '{esc}': '⎋'}


def toml_rows(path):
    """A layered layout TOML → run-style rows (base then each layer)."""
    t = tomllib.load(open(path, 'rb'))
    def clean(r):
        for k, v in TOK.items():
            r = r.replace(k, v)
        return r
    return [clean(r) for r in t['rows']] + [clean(r) for ly in t.get('layer', []) for r in ly['rows']]


def design(rows, F, title):
    """Base and every layer with three or more keys, two boards per row, shaded by keystroke share F."""
    import numpy as np
    names = ['base', 'layer ①', 'layer ②']
    lys = layers_of(rows)
    used, extra = [], []
    for i, ly in enumerate(lys):
        keys = [c for c in ly if c not in '·']
        if i == 0 or len(keys) >= 3:
            used.append((names[i], ly))
        elif keys:
            extra.append(f"{names[i]} holds only {' '.join(keys)}")
    nr = (len(used) + 1) // 2
    fig, axs = plt.subplots(nr, 2, figsize=(6.3, 1.75 * nr), layout='constrained')
    axs = list(np.atleast_1d(axs).flat)
    vmax = max(F.get(c, 0) for c in used[0][1])
    for ax, (lab, ly) in zip(axs, used):
        hold = None
        if lab != 'base':
            key = '①' if '①' in lab else '②'
            hold = used[0][1].index(key) if key in used[0][1] else None
        board(ax, ly, lab, shade=[F.get(c, 0) for c in ly], vmax=vmax / 3 if lab != 'base' else vmax, hold=hold)
    for ax in axs[len(used):]:
        ax.axis('off')
    fig.suptitle(title + ('  ·  ' + '; '.join(extra) if extra else ''), x=0.01, ha='left', fontsize=7, color=INK)
    return fig
