"""Rebuild illustrative figures: MPLCONFIGDIR=/tmp/gecko-mpl python generate_figures.py.
Requires numpy and matplotlib. No GECKO results are represented by these data.
"""
from pathlib import Path
import re
import numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

HERE = Path(__file__).resolve().parent
STYLE = (HERE.parents[2] / 'web/src/styles.css').read_text()
def token(name):
    return re.search(r'--' + name + r':\s*(#[0-9a-fA-F]+)', STYLE)[1]
INK, MUTED, HAIR, BLUE = [token(n) for n in ('ink', 'muted', 'hair', 'accent')]
plt.rcParams.update({'font.family': 'DejaVu Sans', 'font.size': 10, 'text.color': INK,
    'axes.labelcolor': MUTED, 'xtick.color': MUTED, 'ytick.color': MUTED,
    'axes.edgecolor': HAIR, 'axes.spines.top': False, 'axes.spines.right': False,
    'pdf.fonttype': 42, 'svg.fonttype': 'none', 'svg.hashsalt': 'gecko-paper-sample', 'axes.axisbelow': True})
rng = np.random.default_rng(42)
def frame():
    fig, ax = plt.subplots(figsize=(7.2, 3.8), layout='constrained')
    ax.grid(color=HAIR, linewidth=.5, alpha=.65)
    ax.tick_params(length=0, pad=8)
    return fig, ax
def save(fig, name):
    fig.savefig(HERE / 'assets' / (name + '.svg'), metadata={'Date': None})
    # Size PDF figures for a paper column so labels remain readable at final scale.
    fig.set_size_inches(3.4, 2.4)
    for ax in fig.axes:
        ax.tick_params(labelsize=6.5, pad=4)
        ax.xaxis.label.set_size(7)
        ax.yaxis.label.set_size(7)
        if ax.get_legend():
            ax.legend(loc='lower left', bbox_to_anchor=(0, 1.02), ncols=2,
                      frameon=False, borderaxespad=0, fontsize=6.5)
            for label in ax.get_legend().get_texts():
                label.set_fontsize(6.5)
    fig.savefig(HERE / 'assets' / (name + '.pdf'), metadata={'CreationDate': None, 'ModDate': None})
    plt.close(fig)

# Actual PCA of a seeded synthetic feature matrix; no invented variance labels.
z = np.vstack([rng.normal(loc=c, scale=.55, size=(60, 2)) for c in [(-2, 0), (1, 2), (2, -1)]])
x = z @ rng.normal(size=(2, 6)) + rng.normal(scale=.35, size=(180, 6))
x = (x - x.mean(axis=0)) / x.std(axis=0)
u, s, vt = np.linalg.svd(x, full_matrices=False)
pc = u[:, :2] * s[:2]
variance = s*s / (s*s).sum() * 100
fig, ax = frame()
ax.scatter(*pc.T, s=23, color=MUTED, alpha=.5, linewidths=0, label='Synthetic candidates')
ax.scatter(*pc[24], s=65, color=BLUE, edgecolors='white', linewidths=1.5, label='Selected example', zorder=4)
ax.set(xlabel=f'PC1 · {variance[0]:.1f}% of variance', ylabel=f'PC2 · {variance[1]:.1f}% of variance')
ax.legend(frameon=False, fontsize=9, loc='upper left')
save(fig, 'pca')

fig, ax = frame()
t = np.linspace(0, 30, 121)
for name, rate, floor, color, dash in [('Search A', .20, 3.15, BLUE, '-'), ('Search B', .13, 3.40, INK, '--'), ('Search C', .085, 3.62, MUTED, ':')]:
    y = np.minimum.accumulate(floor + 2.5*np.exp(-rate*t) + rng.uniform(-.045, .045, len(t)))
    ax.plot(t, y, label=name, color=color, linewidth=1.8, linestyle=dash)
ax.set(xlabel='Elapsed time (s)', ylabel='Best score · lower is better', xlim=(0,30))
ax.legend(frameon=False, fontsize=9)
save(fig, 'convergence')

fig, ax = frame()
labels = ['Same-finger', 'Travel', 'Redirects', 'Load imbalance']
y = np.arange(4)
ax.barh(y+.16, [100]*4, height=.27, color=HAIR, label='Reference')
ax.barh(y-.16, [68, 77, 91, 83], height=.27, color=BLUE, label='Selected example')
ax.set(yticks=y, yticklabels=labels, xlabel='Normalized cost · reference = 100', xlim=(0,120))
ax.invert_yaxis()
ax.legend(frameon=False, fontsize=9, loc='lower right')
save(fig, 'metrics')
