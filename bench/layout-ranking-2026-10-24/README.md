# Modern layout ranking, 2026-10-24

31 named layouts (7 classics + the 27 most-liked cmini layouts, minus Vylet and Curtsy for
missing symbols) ranked under the evidence-fitted `default` profile, ansi-30 / english
(angle-mod layouts on ansi-30-angle). Write-up: `docs/papers/layout-ranking/` (`.md` + PDF).

| file | what |
|---|---|
| `gather.py` | scores (`gecko features`) under `default` and `balanced`, other corpora, steepest-descent swap polish → `data.json` |
| `analyze.py` | ranks, symbol sets, gap to same-set optimum, metric fingerprint; run it to print the tables |
| `opt-*.json` | GECKO optimum per symbol set and board (BLS 25 M, seed 1; runs `20261024-2321*`, `20261024-2326*`) |
| `robust.json` | `gecko compare --robust --draws 2000 --seed 1` over the ansi-30 layouts |

Headline: Gallium 11.81, Galite 11.85, Maya 11.93, Graphite 12.13 (a statistical tie); GECKO
optimum 8.07. Every layout improves under swap polish (best swap 4–20 %); inward-roll layouts
lose most to the profile's slight outward-roll preference.
