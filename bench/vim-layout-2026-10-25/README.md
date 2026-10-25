# The best layout for a Vim programmer, 2026-10-25

Write-up: `docs/papers/vim-layout/` (`.md` + PDF). corne-36, BLS 40 M evaluations, 4 threads,
6 seeds per cell; seeds driver and the calibrated layer cost from
`bench/programming-layout-2026-10-25/` (README there).

| file | what |
|---|---|
| `profiles/vim-{free,pairs,row,fingers}.toml` | `prog-cal` with thumb zone `␣ ⌫` only (Enter and Esc compete for the other thumb keys) and the h j k l policy as a constraint (`vim` 100; `context/vim.md`) |
| `layouts/us-layer-esc.toml` | start: the programming US-style layer with Esc on the Sym layer |
| `layouts/us-layer-esc-qhjkl.toml` | same with h j k l on their QWERTY keys (pinned in `vim-qwerty`) |
| `grid.sh` | policies on `vim-programmer-typed` (22 % Vim), then the Vim share 0 / 11 / 33 % with `pairs` |
| `grid2.sh` | 3 seeds × 160 M per policy from its best 40 M layout (`variant.py` → `layouts/best-<policy>.toml`) |
| `analyze.py` → `results.json` | every cell and two references scored under `vim-free` on five corpora; where `⎋ : / h j k l ⏎ w b e` sit; `best_of`: each policy's better cell |
| `layouts/prog-esc.toml` | the programming optimum (40 M) with Esc on its empty layer |
| `runs/` | `config.json` + `best.json` of each policy's 160 M optimum (pairs `…108c69fa`, free `…c76aaf99`, row `…abf06e6d`, fingers `…f03d7e81`, QWERTY `…2b91eb3e`) |

The 40 M runs predate the layered-start fix; their `config.json` start was completed with
`../programming-layout-2026-10-25/fix_start.py`.

## Results

Scored without the h j k l term: free 174.00, pairs 174.08, fingers 174.15, row 174.20, QWERTY spots
175.23; programming optimum + Esc 182.21; US-style start 192.74. Details in the paper.

Corpora: `vim-programmer-typed` = prose 35, Markdown 8, `code-typed` 35, `vim` 22, Backspace 5.6 %;
`vimshare-11`, `vimshare-33` scale the other parts to make room (`gecko corpus mix`, descriptions
in the `.mix.toml`). The `vim` component is synthetic (`context/vim.md`).
