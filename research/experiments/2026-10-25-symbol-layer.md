# Symbol layers for programming and Vim: published designs, optimum, and a human-readable layer

| | |
|---|---|
| Date | 2026-10-25 |
| Question | On corne-36 with Colemak-DH letters, which symbol layer is best for programming and for Vim programming, with digits on their own Num layer (main) or on the Sym layer; how do eleven published layers compare; and what does it cost to make the layer learnable (brackets paired, related symbols grouped)? |
| Answer | Learnability is nearly free: brackets paired and eight symbol groups as constraints cost 0.1–0.25 points (≤ 0.01 % typing time), two hand edits 0.2 more. Every good single-layer design lies 1.8–2.5 points (≤ 0.11 %) above the optimum; Seniply (stacked brackets) +4.7, Miryoku (split Sym/Num) +13.4. The two choices that matter: `< >` on the layer instead of Shift+`, .` (−4.0 points, 0.17 %) and digits on the Sym layer instead of their own layer (−3.1 to −3.8, 0.13–0.16 %). Recommended layers: `data/layouts/layered/corne-sym-prog.toml`, `corne-sym-vim.toml`. |
| Command | `bench/symbol-layer-2026-10-25/README.md` (`refs.py`, `study.py`, `grid*.sh`, `tune.py`, `analyze.py`, `langs.py`) |
| Data | `programmer-typed`, `vim-programmer-typed`; per-language typed code; profiles `free-*` (= `prog-cal`, measured layer cost) and `human-*` (+ `brackets`, `sym_groups` at 100) |
| Artifacts | `bench/symbol-layer-2026-10-25/` (`scores.json`, `tables.md`, `seeds-*.json`, `paths/`, `tables/`, `runs/`), sources in [symbol layers](../sources/symbol-layers.md) |

## Findings

- **Programming** (`corne-sym-prog`, 156.67; free optimum 156.35):
  ```
  @ < > · ·   ^ = ! & |
  * ( ) # ·   $ : _ - +
  { } [ ] ·   % ; \ ` ~       Num: numpad 7 8 9 / 4 5 6 / 1 2 3, 0 on a thumb
  ```
  **Vim** (`corne-sym-vim`, 187.74; free 187.38; Esc on the base thumb, ⏎ and `%` on Sym thumbs):
  ```
  @ < > · ·   · = ! { }
  - ( ) [ ·   & ] : ; _
  + ^ $ \ ·   | ~ ` # *
  ```
  Every optimum puts `( )` on the left home row with `< >` above; Vim optima stack `-` over `+`
  (line up / down) and keep `^ $`, `# *` adjacent unprompted.
- **Published layers** (ltgt frame, Num separate; free score, lower is better): US-style 158.16,
  ShelZuuz 158.21, Neo 3 158.31, Ergo-L 158.34, Elixir 158.66, Getreuer 158.82, BEAKL 15 159.64,
  Seniply 161.07, Miryoku 169.72 (Hietala, Sunaku, Moonlander lack symbols; completed by search).
  Six greedy swaps take Getreuer, Neo 3 or Hietala to 157.1–157.3.
- **Num layer open to symbols:** the search moves nearly all symbols onto it, i.e. merges the layers.
- **Languages:** the optima stay best on Python, Rust, JS, C, shell, Lua and Markdown; human
  variants within 2.3 points; the reference order holds.
- **Flat landscape:** from the optimum, 132 of 507 single swaps cost < 0.1 point, so a person
  can rearrange rare symbols almost freely.
- **Negative / open:** symbol comfort is not measured (costs come from QWERTY letter timing and the
  Shift-calibrated layer press); the eight groups are a judgment from the published designs; GECKO
  cannot place a symbol twice (firmware layers often duplicate `. - =` on Num); one base (Colemak-DH).
