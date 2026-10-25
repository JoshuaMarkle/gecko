---
group: community
kind: community
read: 2026-10-25
status: read
feeds: [layer_switch, symbol_bigram, brackets, keysets]
take: Eleven published symbol layers (3×5 grids), the principles they share, and an editor-typing corpus model; no measured study of layer cost or layer learnability exists
---

# Symbol layers: published designs, their principles, and a code-symbol optimizer

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

## More designs (read 2026-10-25)

Grids as 3×5 per hand (left pinky → index | right index → pinky), from the primary sources;
encoded in `bench/symbol-layer-2026-10-25/refs.py`.

- **Neo 2, layer 3** (German, since ~2004; reused by KOY, Bone, AdNW, Mine). Read from
  xkeyboard-config `symbols/de`, `neo_base`. A whole-board layer reached by a pinky mod; `…` on
  `q`, `@` outside the 3×5:
  ```
  … _ [ ] ^    ! < > = &
  \ / { } *    ? ( ) - :
  # $ | ~ `    + % " ' ;
  ```
  Brackets adjacent in order; `<>` above `()`; `!=` and `->` rolls. No ASCII left on other layers.
- **Ergo-L AltGr layer** (French, 2023; `Nuclear-Squid/ErgoL`, `keymaps/fr/ergol.toml`; Arsenik
  reuses it). "Optimized for code", "no common uncomfortable sequence":
  ```
  ^ < > $ %    @ & * ' `
  { ( ) } =    \ + - / "
  ~ [ ] _ #    | ! ; : ?
  ```
  Bracket families nested on the left hand (`{( )}`, `[ ]` below), arithmetic on right home.
- **Seniply** (SteveP, 34 keys; `stevep99/seniply`, US image): left home row holds mods, so
  symbols sit above and below; shifted digits in order on the left top row (learnable):
  ```
  ! @ # $ %    = ` : ; +
  ⌥ ⌘ ⇧ ⌃ ^    * ( { [ -
  · · \ | &    ~ ) } ] _
  ```
  **Each bracket pair on one finger, opener on the home row, closer below**: editors auto-close,
  so `(` is typed far more than `)` and the pair is rarely consecutive.
- **ShelZuuz** (Reddit, C++ on 3×5, via Getreuer's survey): `_` twice (both ring fingers hold the layer):
  ```
  ` ~ # & |    ^ { } [ ]
  ! _ : = $    @ ( ) _ ;
  % ? * + \    / - < > "
  ```
  `();` an outward roll on the right home row; `->` `<-` on the bottom row; `{}` stacked above `()`;
  `< > ;` at their QWERTY spots for learning; `+ \` mirror `/ -`.
- **Sunaku, Glorious Engrammer** (Glove80, "several hundreds of layout iterations over 9 years";
  `sunaku/glove80-keymaps`). One-handed (left), held by the right thumb, for Vim and shell. Its
  3×5 core is `[ { } ] ?` / `^ = _ $ *` / `< | - > /`, thumbs `% : @`; `( ) ; ,` sit on the number
  row and `! # ~ ' " + &` in an outer column and bottom row the 3×5 does not have. Principles:
  inward rolls (`()`, `[]`, `->`, `<-`, `!=`, `<=`, `~/`), Vim pairs `^ $` (line start/end) and
  `# *` (search back/forward), `%` (match) and `:` on thumbs, shell bigrams (`"$`, `!$`, `$?`, `>&`).
- **BEAKL 15** (via Getreuer): symbols in the 3×3 home block of index, middle and ring fingers;
  pinkies and centre columns avoided; `-` and `;` on the outer pinky columns:
  ```
  · < $ > ·    · [ _ ] ·
  \ ( " ) #    % { = } |
  · : * + ·    · & ^ ~ ·
  ```
- **Elixir layer** (Dusty Pomerleau, elixirforum): `<> <- -> |> += () [] {}` as inward rolls,
  so the right-hand brackets run right to left:
  ```
  ! @ # $ %    ^ & * ? '
  + = ( ) "    : ] [ } {
  < | - > \    ` _ / ~ ;
  ```
- **Jonas Hietala, T-34** (`treeman/qmk_firmware`, `keyboards/ferris/keymaps/treeman/keymap.c`).
  Symbols that are on his base (`, . / ( ) _`) keep their base keys on the layer ("important
  that the symbols on the base layer have the same positions"); `= : ; $ ' "` are combos:
  ```
  ~ + * ! ·    · # @ ^ .
  | { } - \    ` ? [ ] ⟳
  , < > % ·    / & ( ) _
  ```
  Number layer by frequency, not numpad: `6 4 0 2 | 3 1 5 7` on the home row, 8 and 9 below.
- **ZSA Moonlander / Dactyl default** ("a reasonable default", Getreuer): left hand
  `! @ [ ] |` / `# $ ( ) `` ` `` / `% ^ { } ~`, a numpad with `* + /` on the right.
- **justinmklam (2025)**: keylogged a week of work (Python + Vim) and re-placed symbols by
  count; Vim made `:` and `j k` the top keys; `_` on the bottom pinky by Shift habit. n-of-1.

## What the designs agree on

None of these is measured; they are expert judgment. Where most agree:

1. **Frequent symbols on the home row, rare ones in corners** (all). Same as letter layouts.
2. **Brackets as pairs:** adjacent and in order (Getreuer, Neo, Ergo-L, ShelZuuz, Moonlander),
   mirrored (BEAKL 15), or stacked on one finger (Seniply; relies on auto-close).
3. **Related symbols grouped:** comparisons `! < > =`, arithmetic `+ - * /`, logic `& |`, Vim
   pairs `^ $` and `# *`, quotes. Makes the layer learnable (Getreuer's colour groups, Sunaku).
4. **Common symbol bigrams as inward rolls:** `!=` `<=` `>=` `+=` `->` `=>` `();` `</` (Getreuer,
   Sunaku, Elixir); doubled symbols (`== ++ && || //`) off the pinkies (Getreuer).
5. **Keep anchors:** base punctuation at the same keys on the layer (Getreuer, Hietala); US shifted
   digits on top (Seniply, US-style) or Num positions (Miryoku); QWERTY `< > ;` (ShelZuuz).
6. **Opposite-hand hold** (Miryoku, Sunaku, Arsenik): one thumb holds, the other hand types.
7. **Numbers on their own layer, as a numpad** (Miryoku, Getreuer, Moonlander, Sunaku, Seniply);
   Hietala orders by frequency; Getreuer notes `0 1 2` dominate (Benford) but keeps digits together.
8. **Hand design over optimizers** (Getreuer): optimizer output "random-looking, hard to learn",
   and it reshuffles on small changes. GECKO answers this with `sym_groups` and `brackets` as
   constraints and pinned starts, not by giving up the search.

## How to use in GECKO

- References for corne-36 symbol layers: `bench/symbol-layer-2026-10-25/` (all designs above,
  one frame) and `bench/programming-layout-2026-10-25/` (Getreuer, Miryoku, US-style).
- Rule 3 is the `sym_groups` metric (profile list of related pairs); rule 2 is `brackets`.
- The bracket result matches GECKO's `brackets` metric: pairing costs little, so it can stay a
  constraint-like weight.
- Editor typing: `corpus build --editor autoclose,complete=K` models auto-closed block closers
  and completion; mixing with the plain corpus sets the rate (`context/cli.md`).
- Do not copy the cost weights: they are estimates; GECKO's are fitted to timing.
