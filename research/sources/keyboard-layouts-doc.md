---
group: community
kind: community
read: 2026-10-20
status: read
feeds: [definitions, validation]
take: Shared metric definitions; distance examples; design rules
---

# The Keyboard Layouts Doc (ec0vid, 2nd edition) and the AKL community

Google Doc, community reference for metrics and layout theory:
https://docs.google.com/document/d/1_a5Nzbkwyk1o0bvTctZrtgsee9jSP-6I0q3A0_9Mzm0
(3rd edition linked from its top). Community: Alt Keyboard Layouts (AKL) Discord,
r/KeyboardLayouts. Short intro: semilin, "What makes a keyboard layout good?" (2023).

## Definitions the community uses (match these names in reports)

- **SFB:** same finger, two different keys in a row. Index columns count as one finger.
- **SFS / DSFB:** same finger with ≥1 key between (skip-1, skip-2 …). Fewer keys between and
  longer distance = worse.
- **LSB:** adjacent fingers, keys far apart laterally (index center column + middle, QWERTY `be`).
- **Scissor:** adjacent fingers, row jump (full = 2 rows, half = 1 row with the "wrong" finger
  reaching).
- **Trigram classes:** alternate (L-R-L), roll (2 same-hand + 1 other, in or out),
  onehand (3 same-hand, monotonic), redirect (3 same-hand, direction change; "bad" if no index).
  Computed on trigrams so `dfdfdf` is not counted as many rolls.
- Alternation and rolling trade off; high rolling brings onehands and redirects with it.
- **Distance / finger speed:** movement from the previous key on that finger, not from home
  (`refer` on QWERTY: second e costs nothing). Weight SFB distance 1, skip-1 0.5, then halve.

## Numbers worth keeping

- Distances on row-stagger (top row offset 0.25 u, bottom 0.5 u from home): `rg` 1.6 u,
  `my` 2.66 u, `vt` 2.02 u, `fg` 1.0, `fr` 1.03, `fv` 1.12, `ft` 1.25, `fb` 1.8, `jn` 1.12,
  `jy` 1.6. Left and right center-column diagonals are **not symmetric** on row-stagger.
- SFB: QWERTY 6.6 %, Workman 3.1 %, Dvorak 2.6 %, Colemak ≈ 1.6 % (1 % without punctuation).
  "Above 2 % is high, near 1 % ideal." Pinky SFBs should be near zero.
- QWERTY column `ecd` alone adds 2.7 % SFB.
- Finger-speed weights (Oxeylyzer default): index 5.5, middle 4.8, ring 3.6, pinky 1.4.
- Vowel-hand rule for low redirects: all vowels on one hand; `h` rolls into vowels 83:1, so
  high-roll low-redirect layouts put `h` with the vowels; pairs to avoid on one finger:
  `ly`, `pl`, `mp`, `bl`, `by`.
- Space-thumb trigrams change roll/redirect stats; top space trigrams ` th` 1.78 %, `he ` 1.03 %.

## Design principles (from sections 6–10)

- Layouts are grouped by **stacks** (vowel and consonant blocks) and which consonants share
  the vowel hand.
- Do not put frequent letters in the inner (center) columns of the vowel hand (LSBs).
- Angle mod (ANSI): shift left bottom row one key left so the ring finger takes `z`-position
  work; changes fingering, so layouts must declare their fingering.
- **Do not rank layouts by one overall score**; analyzers disagree on totals. Compare stats.

## How to use in GECKO

- Use these names and definitions in `gecko compare` output so users can compare with cmini.
- The distance examples are a unit test for `ansi-30` geometry (pair distances).
- Add an angle-mod fingering for `ansi-30` (bottom-left keys shift one finger).
