---
group: community
kind: community
read: 2026-10-25
status: read
feeds: [layer_switch, symbol_bigram, brackets, keysets]
take: Three published symbol-layer designs to score against, their principles, and an editor-typing corpus model; no measured study of layer cost exists
---

# Symbol layers: Getreuer, Miryoku, and a code-symbol optimizer

Three designs that place programming symbols off the letter keys, read 2026-10-25 at the
primary sources. A literature search (layer hold vs one-shot, chord timing, symbol-layer user
studies) found **no measured study**: every layer design below rests on its author's judgment
or an estimated cost model. GECKO's own Shift timing from the Aalto data
([programming-layout experiment](../experiments/2026-10-25-programming-layout.md)) is the only
measured layer-like cost found.

## Pascal Getreuer, "Designing a Symbol Layer" (blog, v2) and `getreuer/qmk-keymap`

https://getreuer.info/posts/keyboards/symbol-layer/ ; keymap in `getreuer.c` (Apache-2.0).

- **Data:** character frequencies over Prose, C/C++, Python, Shell, LaTeX and a mix (≥ 40K
  characters each). Top C/C++ symbols `_` 1.369 %, `*` 1.238 %, `,` 1.200 %, `(` `)` 1.151 % each;
  Python `_` 2.213 %. Code bigrams (`!=` `<=` `+=` `->` `();`) set the placement.
- **Principles:** frequent symbols on the home row, rare ones in the corners; no pinky for
  symbols that are often doubled (`==` `++` `//`); common symbol bigrams as inward rolls; keep
  `,` `.` where the base layer has them; easy to learn. He rejected automatic optimizers
  because their layers are hard to learn and reshuffle on small changes.
- **Layer (3×5 per hand, outer columns omitted), held from a thumb:**
  ```
  ` < > - |    ^ { } $ →        (→ is a macro typing "->")
  ! * / = &    # ( ) ; "
  ~ + [ ] %    @ : , . '
  ```
  Index fingers rest on `=` and `(`. Groups: comparisons `! < > =`, arithmetic `- + / *`,
  brackets adjacent `[ ]` `( )` `{ }`. Inward rolls `!=` `<=` `>=` `+=` `/=` `*=`. No
  measured result is reported.
- Number layer: a numpad on the left (`9 8 7` / `3 2 1` / `6 5 4`, 0 on a thumb).

## Miryoku (manna-harbour)

https://github.com/manna-harbour/miryoku (docs), layers in `miryoku_qmk`
`miryoku_layer_alternatives.h` (GPL-2).

- **Rules:** a layer is held by a thumb of the *opposite* hand, so one hand holds and the other
  types; Num and Sym share one arrangement on the left hand; Sym holds the *shifted* symbols of
  the Num keys (same finger positions); numerals in numpad order.
- **Num (left hand; thumbs `. 0 -`):**
  ```
  [ 7 8 9 ]
  ; 4 5 6 =
  ` 1 2 3 \
  ```
- **Sym (left hand; thumbs `( ) _`):**
  ```
  { & * ( }
  : $ % ^ +
  ~ ! @ # |
  ```
- Base keeps `' , . /` on the letter keys; `" < > ?` come from Shift. Nav (right hand): arrows
  on the home row in vi order as an option.

## hikazey/python-layout-optimizer (GitHub, 2025)

https://github.com/hikazey/python-layout-optimizer. Places the 32 ASCII symbols on a 47-key
ANSI board with the letters and digits fixed.

- **Corpus model of editor typing (the useful part):** 38.5 M weighted keystrokes of Python
  from 29 repositories; comments ×0.6, docstrings ×0.4, **closing brackets and quotes ×0.3**
  (auto-closed), **identifier characters after the 4th ×0.3** (autocomplete).
- **Cost model (estimated, not measured):** key effort + 1.5 per Shift; SFB 2.5 + 1.0 per key
  of travel; row skip 0.6; inward roll −0.3; hand switch −0.1.
- **Results:** Programmer Dvorak 1.842 per keystroke → optimized 1.700 (−7.7 %); requiring
  brackets to be adjacent or mirrored costs only 0.4 % (1.693 unconstrained). Optimizing the
  symbols of QWERTY gains 11.1 %, Colemak-DH 11.7 %. Rare symbols (`# \ | @ $ % ;`) move
  between runs. Twelve restarts agree within 0.002.

## How to use in GECKO

- Score the Getreuer and Miryoku layers as references for the corne-36 symbol layer (layouts
  in `bench/programming-layout-2026-10-25/`).
- The bracket result matches GECKO's `brackets` metric: pairing costs little, so it can stay a
  constraint-like weight.
- Editor typing: `corpus build --editor autoclose,complete=K` models auto-closed block closers
  and completion; mixing with the plain corpus sets the rate (`context/cli.md`).
- Do not copy the cost weights: they are estimates; GECKO's are fitted to timing.
