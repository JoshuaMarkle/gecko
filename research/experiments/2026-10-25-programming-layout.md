# The best programming layout with a symbol layer, and what a layer press costs

| | |
|---|---|
| Date | 2026-10-25 |
| Question | On corne-36, which arrangement of letters, symbols and digits on base and layers is best for a programmer, and how do published symbol layers (Getreuer v2, Miryoku, a US-style layer) compare? What does a layer press cost? |
| Answer | A held Shift costs ×1.78 the interval into the next key (+0.578 log, 3.68 SFBs; 56,358 Aalto typists), so `layer_switch` gets weight 26.8, not 1. With it, the optimum keeps `, . ' =` on the base and puts every other symbol and digit on **one** held layer; digits 1–0 in a row on it cost +0.5 %. Getreuer's layer is within 0.5 % of an optimized layer on the same Colemak-DH letters; a US-style layer is 7 % and Miryoku 14 % behind it. Re-optimizing the letters gains 3.4 % more. All differences between good designs are under 0.3 % of typing time. |
| Command | `bench/programming-layout-2026-10-25/README.md` (`shift.py`, `refs.py`, `grid.sh`, `grid2.sh`, `analyze.py`) |
| Data | `programmer-typed` (prose 45, Markdown 10, code as typed 45, Backspace); per-language typed code; profiles `prog-cal`, `prog-old` (code + thumb zone + letters on base) |
| Artifacts | `bench/programming-layout-2026-10-25/` (`results.json`, `seeds-*.json`, `runs/`), paper `docs/papers/programming-layout/` |

## Findings

- **Layer cost (new measurement):** capital after a space vs lowercase: +0.578 (left hand
  +0.595, right +0.559); letter → `?` vs letter → `.`: +0.590. The release (+0.40) is
  confounded by names and left out. Assumes a thumb-held layer costs what Shift costs.
- **At the old weight** the optimizer empties base keys and moves `, . '` to the layer: +29 %
  under the measured cost (13.5 layer presses per 100 bigrams vs 9.9).
- **Good designs all press a layer ~9.9 times per 100 bigrams**; they differ in symbol bigrams,
  SFB and letters. One-shot layers re-searched: +30 % (a press per symbol, not per run).
- **Corpus:** typed code drops `}` and `_` by half against files. An optimum on code alone is
  48 % behind on prose; one on raw files 3.9 % behind on typed text.
- **Search:** only 1 of 6 BLS seeds reaches the best per cell (spread up to 3 %); see the paper
  for the 4× longer check.
