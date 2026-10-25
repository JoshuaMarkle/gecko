# The best layout for a Vim programmer: h j k l policies, Esc, Vim share

| | |
|---|---|
| Date | 2026-10-25 |
| Question | On corne-36 with a symbol layer, what is the best layout for a programmer who uses Vim, what does each h j k l policy cost, where do Esc, Enter, `:` and `/` go, and how much does the Vim share matter? |
| Answer | Every policy reaches the free optimum within 0.1 % (pairs +0.05 %, fingers +0.09 %, row +0.11 %); QWERTY spots cost 0.7 % (0.05 % of time). Esc goes on a base thumb and Enter onto the symbol layer in every optimum with Vim in the text; `:` `/` on the layer's home row. The 22 % optimum is the best found at 11–33 % Vim and 0.3–0.4 % behind without Vim; the programming optimum with Esc on a layer is 4.7 % behind on the Vim mix. |
| Command | `bench/vim-layout-2026-10-25/README.md` (`grid.sh`, `grid2.sh`, `analyze.py`) |
| Data | `vim-programmer-typed` (22 % synthetic Vim), `vimshare-11`, `vimshare-33`, `programmer-typed`; profiles `vim-*` (`prog-cal` with thumb zone `␣ ⌫` and the policy at weight 100) |
| Artifacts | `bench/vim-layout-2026-10-25/` (`results.json`, `seeds-*.json`, `runs/`), paper `docs/papers/vim-layout/` |

## Findings

- **Search noise:** at 40 M evaluations seeds spread 3–5 % and the policies looked 1–3 % apart;
  3 × 160 M per policy agree within 0.2–0.9 % and the policies within 0.1 %. Policy costs need
  long searches.
- **Enter on the layer** relies on free layer releases (the Shift timing found a +0.40 release
  effect, confounded and not used). Without Vim, Enter returns to the thumb.
- **Negative / open:** the Vim keystrokes are synthetic; no population data on Vim keystrokes
  were found (single-user samples only).
