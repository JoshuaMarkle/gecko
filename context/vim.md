# Vim: h j k l and the `vim` corpus

A Vim user types two things: text (inserts) and commands (motions, operators, `⎋`, `:`).
`vim-programmer` mixes both (prose 35, Markdown 8, code 35, Vim sessions 22, ⌫ 5.6 %). This
page covers what to do with h j k l, which a layout search would otherwise scatter.

## Policies

| Policy | How | What it keeps | Cost (below) |
|---|---|---|---|
| QWERTY spots | `optimize --from qwerty --pin hjkl` | h j k l exactly where they are: no relearning, Vim help and muscle memory still apply | +24 % |
| Row | profile `vim = "row"`, `vim` weight 100 | h j k l in order on four neighbouring keys of one row, anywhere | +12 % |
| Pairs | `vim = "pairs"`, `vim` 100 | h left of l and j left of k, each pair on neighbouring keys; the pairs may sit apart | +2 % |
| Fingers | `vim = "fingers"`, `vim` 100 | four different fingers of one hand (any rows): a chord-like cluster, no shape | +7.5 % |
| Free | no `vim` | nothing; remap motions in your config instead | baseline |

- **Row** is the classic case: the mnemonic is the order, not the keys. It costs the most
  of the shaped policies, since four letters must share a row; the search parks them on a
  bottom row.
- **Pairs** keeps the two mental axes (left/right, down/up) and lets each pair go where it
  fits. On QWERTY h–l are not neighbours, so QWERTY itself breaks half of this policy.
- **Fingers** only asks that the four keys can be pressed without moving a finger off them;
  arrows of any shape on one hand. Looser in shape than pairs, but it forces all four onto one
  hand, which costs more here.
- **Free** suits users who remap (`noremap` to the new letters' positions, or arrow keys on a
  layer). Remapping costs config upkeep and breaks other machines' Vim.

`vim` counts "% of the policy's key pairs broken" (reversed neighbours half), unweighted by
frequency, so at weight 100 any breakage outweighs every typing metric: a constraint, not a
preference. Lower weights (1–5) let the search trade it. In the web app: Profile → Shortcuts
and familiarity → Vim h j k l.

## The `vim` corpus

`gecko corpus vim` writes a synthetic corpus from a weighted grammar of normal-mode commands
(`gecko_core::vim::COMMANDS`: motions ≈ 60 %, operators with motions and text objects,
visual mode, inserts of ~12 characters typed from `--text` then `⎋`, `:w`, searches, leader
maps). Ctrl chords count as their letter key. Weights are an assumption informed by two
published single-user logs (w, j, k lead; j and k far above h and l), not a population
measurement. Built with `--text code-all`: ⎋ 3.7 %, w 4.5 %, j 4.2 %, k 3.5 % of keystrokes.

Your own logs beat the model:

```
nvim -w ~/.cache/nvim/keys.log          # append every typed key (alias it for a week)
gecko corpus vim --log ~/.cache/nvim/keys.log --out my-vim
gecko corpus mix prose:35 markdown:8 code-all:35 my-vim:22 --edits 0.056 --out my-vim-programmer
```

`-w` appends the raw keys of every session. `--log` maps them to corpus symbols (Esc `⎋`, CR
`⏎`, Tab `⇥`, Backspace `⌫`, arrows, a Ctrl chord its letter); other special keys (mouse,
focus events) break the n-grams.

## Results

2026-10-24, `vim-programmer`, ansi-30, SA 60 s × 20 threads, seeds 1 and 2; each best layout
rescored with profile `default` (no `vim`), so the numbers are what the policy costs in typing
score (lower is better). The best of the two seeds is shown. Under the weight-100 constraint SA
is noisier (row 25.5 / 24.5, pairs 22.3 / 23.6 across seeds); the free and QWERTY-pinned runs
gave the same layout on both seeds.

| Policy | Score | vs free | SFB % | `engram` comfort | `typing_time` time | Best layout (top / home / bottom) | Run |
|---|---|---|---|---|---|---|---|
| Free | 21.85 | — | 2.51 | 5.90 | 7.62 | `mflpzqyou, dsrnbgteia cxwhv/kj;.` | `20261025-011539-sa-b13d65de` |
| Pairs | 22.33 | +2 % | 2.46 | 5.87 | 7.70 | `,uoyqzhlfc aietgbnrsd ./jk;vpwxm` | `20261025-011842-sa-a654d29d` |
| Fingers | 23.49 | +7.5 % | 2.55 | 6.54 | 8.03 | `.uopyvdnlj aiecwkthrs /,;gzbfmqx` | `20261025-011943-sa-59d0c3ce` |
| Row | 24.48 | +12 % | 2.33 | 6.84 | 8.16 | `.uofyqvdmx aiecwpntsr /,;gzbhjkl` | `20261025-012329-sa-c4ef80a0` |
| QWERTY spots | 27.04 | +24 % | 2.50 | 7.20 | 8.40 | `.uocyqnswv aiedphjklt /,;gzxrfmb` | `20261025-011640-sa-2e0c3427` |
| QWERTY itself | 69.57 | | | | | | |

Reading: **pairs is nearly free** (h l and j k as two neighbour pairs fit where the search
wants those letters anyway), so it is the recommended policy for a new layout. Row costs about
half of what pinning QWERTY spots costs. Pinning h j k l at QWERTY is still far better than
QWERTY (27 vs 70): the rest of the layout carries most of the gain. `engram` is Engram comfort and
`typing_time` (both lower is better; judges, not weighted).

