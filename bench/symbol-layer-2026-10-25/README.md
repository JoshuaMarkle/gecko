# Symbol layers for programming and Vim, 2026-10-25

Write-up: `research/experiments/2026-10-25-symbol-layer.md`; report and PDF `docs/papers/symbol-layer/`; designs and principles in
`research/sources/symbol-layers.md`. corne-36, BLS, 4 threads per run.

## Frame

Colemak-DH letters, base punctuation `' , . /`, thumbs `⏎ ␣ ⇧ | Sym ⌫ Num` (Vim: `⎋` instead of
`⏎`, which goes on the Sym layer). Sym is held by the right inner thumb, Num by the right outer
one, so the right thumb keys of both layers belong to the holder. Digits: a fixed numpad on
Num's left hand (`789/456/123`, 0 on the left middle thumb). Only the Sym symbols move:
letters, base punctuation, digits and the base thumbs are pinned, and Num's other keys hold
zero-frequency Greek fillers (pinned), so no symbol can move there.

| frame | Shift pairs on the base | Sym layer symbols |
|---|---|---|
| `us`, `us-vim` | `'" ,< .> /?` | the other 24 ASCII symbols (+ `⏎` for Vim) |
| `ltgt`, `ltgt-vim` | `'" /?` (plain `,` `.`) | those plus `< >` (26) |

Profiles: `free-prog` = `prog-cal` (programming study: measured layer cost 26.8); `free-vim` the
same with thumb zone `␣⌫`. `human-*` add `brackets` 100 and `sym_groups` 100 (scales 1):
groups `+- +* != &| -_ :; ^$ ~\`` (Vim adds `#*`). Score lower is better; one point ≈ 0.043 %
typing time per keystroke.

## Files

| file | what |
|---|---|
| `refs.py` → `layouts/<frame>/<name>.toml`, `layouts/refs.json` | eleven published Sym layers + US-style in each frame; symbols a design lacks are listed in `refs.json` |
| `study.py complete` → `layouts/<frame>/<name>.c.toml` | missing symbols placed by search (2M BLS) with the design's own symbols pinned |
| `study.py search`, `grid.sh` | 4 frames × free/human, 6 seeds × 10M → `seeds-<cell>.json`, `layouts/best/<cell>.toml` |
| `digits.py`, `grid2.sh`, `grid4.sh` | digits on the Sym layer (`row`: home row 1–0; `pad`: numpad left hand), Num open for overflow; `us*` 10M, `ltgt*` 20M |
| `grid3.sh` | Num open to symbols (numpad pinned): the search merges the layers (seeds did not converge) |
| `tune.py table / swaps / path` | price of every single swap (`tables/`), hand edits (`layouts/tuned/`), greedy edits of published layers (`paths/`) |
| `study.py score` → `scores.json`; `analyze.py` → `tables.md` | every layout under both profiles; tables per frame |
| `langs.py` → `langs.json` | layouts on each typed-code corpus |
| `runs/` | `config.json` + `best.json` of each cell's best run (`runs/index.json`); `gecko verify run:<id>` re-runs (checked: `ltgt-human`) |

## Results (`tables.md`)

| frame | free optimum | human | tuned | best published (complete) | digits on Sym, human |
|---|---|---|---|---|---|
| us | 160.31 | 160.40 | | 162.09 ShelZuuz | row 156.60, pad 156.68 |
| us-vim | 191.44 | 191.54 | | 193.41 ShelZuuz | pad 186.91, row 187.21 |
| ltgt | 156.35 | 156.48 | 156.67 (`! =`, `\ #`) | 158.16 US-style | row 153.34, pad 153.66 |
| ltgt-vim | 187.38 | 187.63 | 187.74 (`! =`, `% @`) | 189.46 ShelZuuz | pad 183.94, row 184.20 |

Seeds per cell agree within 0.01–0.1 points (free) and ≤ 0.8 (human and digits cells: often 1 of 6 at the best); `grid3.sh` cells did not converge (spread up to 4).
Tuned layouts are `data/layouts/layered/corne-sym-prog.toml` and `corne-sym-vim.toml`.
