---
group: community
kind: community
read: 2026-10-20
status: read
feeds: [L05, fingering]
take: No single stat; corpus dependence; alt-fingering; thumb letters
---

# Community design notes: Hands Down, Gallium, Graphite, Semimak

Short lessons from layout authors. Use as qualitative checks, not as numbers to optimize.

## Sources

- Hands Down (Alan Reiser): https://sites.google.com/alanreiser.com/handsdown (design notes,
  statistics, FAQ). Neu variant ≈ 0.95 % SFB; some variants put a letter on the thumb.
- Gallium (GalileoBlues): https://github.com/GalileoBlues/Gallium (row- and col-stag variants).
- Graphite: https://github.com/rdavison/graphite-layout. Sturdy: https://oxey.dev/sturdy/
- Semimak reflections: see `genkey-semimak.md`.
- Ian Douglas's KLA analyzer (used by Hands Down): https://kla.keyboard-design.com

## Lessons

- **No single statistic.** Hands Down balances many metrics and tests on real hands; Gallium
  deliberately lowers rolls to bring alternation and rolls closer and to balance hands.
- **Corpus dependence is real.** Gallium is fine on the 10k-word list and worse on the 450k list
  (`hy py ph pf` SFBs). Small changes like swapping `x/q`, `j/z` fix rare-pair SFBs at no cost.
- **Variants per board.** Several authors ship row-stagger and column-stagger versions; the
  geometry changes the best layout.
- **Alt-fingering** (pressing a key with a non-default finger) is common among fast typists and
  hides some SFBs; a strict fingering model overstates them.
- **Thumb letters** (Hands Down Gold: `t`; Alt-ex: `e`) cut finger movement a lot but are a
  big learning cost; keep thumb letters as an explicit option.

## How to use in GECKO

- Robustness metric L05 over several corpora (prose, word lists 10k/450k, chat, code).
- Per-geometry optimization is expected; do not expect one layout to win everywhere.
- An optional "alt-fingering" relaxation: allow a small set of keys to have two fingers and
  take the cheaper per bigram (later; it breaks the fixed slot→finger table).
