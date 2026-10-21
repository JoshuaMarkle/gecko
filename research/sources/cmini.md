# cmini (AKL Discord bot) and its layout database

Python Discord bot, GPL-3. https://github.com/Apsu/cmini (fork of Dainternetdude/amini).
The community's shared reference for "what are this layout's stats".

## Key ideas

- **Trigram classification by finger-combo lookup:** `table.json` maps every finger triple
  (e.g. `LI-RM-LR`, thumbs as `TB`) to a class: alternate, roll-in, roll-out, oneh-in,
  oneh-out, redirect, bad-redirect, sfb, sft, dsfb-alt, dsfb-red, sfR (repeat), unknown.
  1025 entries. Simple and exactly reproducible; a good cross-check for GECKO's T-classes.
- SFB = same finger, different letter, excluding space; percentages over all bigrams.
- Fingermaps are per layout (`row`, `col`, `finger`), so angle-mod and thumb letters work.

## Data worth reusing

- `layouts/`: **≈4100 community layouts** as JSON (`name`, `board` = ortho/stagger/angle,
  `keys: {char: {row, col, finger}}`). A large reference set for ranking tests, for
  learning "what community layouts look like", and for duplicate detection.
- `corpora/`: precomputed `monograms/bigrams/trigrams/words.json` for english-200/1k/5k/10k/
  450k (Monkeytype word lists), shai, monkeyracer, mt-quotes, reddit, movie, keymash,
  gutenberg, akl, french, dutch, and more.
- `likes.json`, `authors.json`: rough popularity signal.

## How to use in GECKO

- Importer done: `gecko layouts import-cmini` → `data/community/cmini.json` (30-key core,
  finger-map check, thumb-letter layouts skipped); `gecko layouts likes` correlates GECKO rank
  with likes (results in `context/objective.md` Calibration).
- Use the 450k vs 10k word-list corpora as a **robustness test** (L05): Gallium does well on 10k
  and worse on 450k (more `hy py ph pf`), see `community-design-notes.md`.
- Validate GECKO's trigram classes against `table.json` on a few layouts.
- License: the bot code is GPL; layouts and n-gram counts are data. Record attribution.
