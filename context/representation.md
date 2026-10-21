# Representation

Three independent inputs: **geometry** (physical board), **layout** (symbols on keys),
**corpus** (n-gram statistics). Parameter lists: `research/parameters.md`.

## Geometry

- TOML in `data/geometries/`. One `[[key]]` per slot, listed row by row, with
  `pos[3], rot[3], size[2], finger, row, col, home, stretch, effort` (hand comes from finger;
  `role` comes with symbol layers). Optional `hrm` (bool) marks home-row mod keys for Y04;
  unset = `home` keys on a finger outside the stretch columns (`Geometry::hrm`). Boards: `ansi-30`, `ortho-30`, `corne-36` (3x5+3),
  `sweep-34` (3x5+2); the split boards' stagger and thumb poses are approximate.
- Thumb keys use fingers `LT`/`RT` on their own row (row 3). Optional `fill` per key (token
  name or one char): what the key gets when a layout lists only the keys without a fill, so
  every 30-key reference layout fits `corne-36`/`sweep-34` (`Layout::fit`; the CLI and server
  apply it). Thumb fills follow Miryoku: `⎋ ␣ ⇥ | ⏎ ⌫ ·` (36), `␣ ⇥ | ⏎ ⌫` (34).
- Thumbs: trigrams with a thumb class as "other"; B05 scissors ignore thumbs; M02 compares
  non-thumb finger loads as shares of non-thumb keystrokes (unchanged when thumbs are idle).
- Optional `[comfort]` maps comfort metrics to key-pair tables: `E04 = "engram-2key"` loads
  `data/comfort/engram-2key.csv` (`a_row,a_col,b_row,b_col,score,…`, 0..1, higher = more
  comfortable, keys matched by row/col). The store loads them into `comfort_tables`, which are
  served with the geometry so the WASM scorer has them. ansi-30: E04 Engram formula, E05 fitted
  from Engram's raw choices (`research/scripts/fit_engram_bt.py`).
- `effort` is the base per-press cost. Profiles add a per-finger row offset on top
  (`row_offset`, pinky ring middle index × [above home, below home]; thumbs and home row 0):
  the default/code profiles make the top row cheaper for middle/ring and the bottom row
  cheaper for index (KLO's key-cost asymmetry), on any geometry. M01 uses `effort + offset`.
- Positions are always 3D. Flat boards use z = 0 and only z-rotation. `pos` is the
  unrotated top-left corner; rotation is about the key center (as the 3D viewer draws it).
- 2D drawing: `Geometry::flat2d()` (`flat2d.rs`) gives `{pos: [x, y], rot}` per key. Flat
  boards return their own x, y, rot z. Others: key centers projected onto the best-fit (PCA)
  plane — per hand when the halves' planes differ by > 5° (tented splits) — keeping each half's
  centroid and each key's in-plane rotation; then overlapping keys (2D OBB, SAT) are pushed
  apart along the minimum translation vector, half each, until none overlap. `/api/geometries`
  serves it as `flat`; WASM exports `flat2d(geometry_json)`.
- KLE import (`gecko geometry import-kle`, `gecko_core::kle`): the downloaded JSON or the
  editor's raw data. Position, size and z-rotation (`r` about `rx, ry`; GECKO `pos` is the
  top-left of the unrotated key, rotated about its center) are exact; decals are skipped, ISO
  Enter's second rectangle ignored. The rest is a guess to edit by hand:
  - Rows: unrotated keys sharing an x center form columns, numbered top-down and shifted to
    match the left neighbour (column stagger); other keys join the row within 0.5u in y.
  - ≥ 4 rows: the last is the thumb row (`LT`/`RT`, row 3) and home is two above it; 3 rows:
    home is the middle one. Row numbers count from the top alpha row (0); a number row is -1.
  - Home row keys under 1.5u split into hands at a gap ≥ 1.5u, else by count (left gets the
    smaller half). From the inside: index stretch (if ≥ 5 per hand), index, middle, ring,
    pinky home, extra pinky columns. Every other key takes the nearest home column's finger
    (ties go left, so ANSI's B is left index); `col` is the position in its row.
  - Efforts: ortho-30's table by row and finger, +1 per row beyond top/bottom, +0.8 per unit
    outside the pinky column; thumbs 1.0 + 0.5 per unit from the hand's rest thumb (nearest to
    half a key inside the index home). No `fill`s: add them for 30-key layouts to fit.
  - `kle::export` writes one KLE row per key (2D only); every bundled board round-trips
    export → import with the same fingers, rows and columns.
- At scorer build, precompute dense tables indexed by slot:
  - pair `[N×N]`: same finger, same hand, distance, dx, dy, finger order/direction,
    LSB flag, scissor flag, row jump.
  - triple class `[N×N×N]` (u8): trigram class (alternate, roll in/out, redirect…).
- A geometry is identified by a content hash; runs record it.

## Layout

- `pos: [u8; S]` symbol → slot and `sym: [u8; N]` slot → symbol. Swap is O(1).
- `locked: u64` bitmask (u128 or bitset once N > 64).
- Symbols are chars. Special keys are tokens with reserved glyphs so each slot stays one char
  in strings/JSON/UI: `␣` space, `⏎` enter, `⌫` bspc, `⇥` tab, `⎋` esc, and `·` = empty slot
  (zero frequency, may repeat, unpinned unless `--pin '{none}'`), `⇧` Shift (base layer only,
  may repeat, never moves). Text input (layout rows, `raw:`, `--pin`) also takes `{space}`,
  `{enter}`, `{bspc}`, `{tab}`, `{esc}`, `{none}`, `{shift}`.
  The scorer gives repeated `·` distinct symbol ids.
- Text format: rows of symbols matching the geometry's rows (`data/layouts/*.toml`), e.g.
  ```
  geometry = "ansi-30"
  rows = ["qwfpbjluy;", "arstgmneio", "zxcdvkh,./"]
  ```
- Other analyzers' files (`gecko layouts import`, `gecko_core::import`): Oxeylyzer `.dof` JSON
  and v1 grids, genkey text, cmini JSON. Mapped by physical position onto rows 0–2 × cols 0–9;
  extra columns, number and thumb rows are dropped (warning; a dropped letter is an error);
  source fingering (angle mod) is not imported. `export` writes the 30 alpha keys back.
- Stable hash = hash(geometry hash, sym array). Used to de-duplicate results.
- Layers: `chars` is the base and `layers[l-1].chars` layer l, one symbol per key
  (`sym[layer][key]`). Layer l is reached by the base key carrying its layer-key token, whose
  glyph is the layer mode: hold `①②③` (`{layer1}`…), one-shot `⓵⓶⓷` (`{os1}`…), toggle `❶❷❸`
  (`{tg1}`…). TOML: base `rows`, then one `[[layer]]` table per layer (`name`, `rows`, optional
  `mode = "hold" | "oneshot" | "toggle"`, which rewrites a `{layerN}` token; a conflicting
  `{osN}`/`{tgN}` is an error). Layered layouts live in `data/layouts/layered/` (examples
  `corne-code`, `corne-code-oneshot`, `corne-code-numtoggle`), so `compare` keeps ranking
  like-for-like layouts. A hold layer cannot use the keys of the finger holding it; one-shot and
  toggle layers only lose the layer key itself (on a toggle layer that is the key that leaves).
  Unusable keys are written `·`. The flat string carries the modes through its tokens.
- Scoring flattens a layered layout onto `Geometry::layered`: the base keys, then each layer's
  usable keys as virtual keys (`layer`, `phys` set; `layer_keys` = holders, `layer_modes`). Metrics see each
  symbol at its physical key (layer copies of one key are the same key: repeats, not SFBs);
  Y01/Y02 add the layer costs. The flat char string (base, then layer 1's usable keys, …) is
  what runs, `raw:`, `/api/score` and WASM use; `Layout::from_flat` / `fit` split it back
  using the `①②③` positions. Layer keys never move (`Ctx::new`). ≤ 128 slots in all.

## Corpus

- Symbols mapped to ids `0..S`. S ≈ 30–36 on one layer, ≤ 128 slots with layers.
- Tables (f32, normalized to sum 1 each): mono `[S]`, bi `[S×S]`, skip (gap 1), skip2, skip3
  `[S×S]`, tri `[S×S×S]` (dense up to S=64 = 1 MB; switch to top-K sparse beyond).
- `data/corpora/<name>.gkc`: magic `GKCORP2`, name, charset, u64 counts for mono, bi, skip,
  skip2, skip3, tri. `GKCORP3` (written only when the corpus has case counts, so other files
  keep their bytes and hash) appends `shift [S]` (Shift presses before symbol y: starts of runs
  of shifted symbols, hold model) and `shift_bi [S×S]` (the presses right after symbol x).
  `GKCORP1` files (no skip2/skip3) and `GKCORP2` (no Shift) still load with those tables zero.
  Mixes are weighted sums of normalized tables; Shift counts scale with mono, so presses per
  keystroke survive.
- Tokenization: lowercase, an uppercase letter counting as shifted; a character outside the
  charset whose US-QWERTY unshifted key is in it is that key, shifted (`:` → `;`, `"` → `'`,
  `?` → `/`, `!` → `1`, …; with `--charset code` they are their own symbols); curly quotes →
  `'`, dashes → `-`; whitespace runs → one space
  token, the symbol `␣` when the charset has it (else it breaks n-grams); other characters
  break n-grams. Word imports count each word as `␣word` plus the exact `x␣`, `yx␣` that
  follow it; only `x␣z` trigrams and cross-word skipgrams are estimated (independent words). Bigrams/trigrams count only charset runs; skip-g counts
  x and z with exactly g tokens between, whatever they are. Project Gutenberg headers are stripped.
- `gecko corpus build <text>` counts raw text. `gecko corpus import <dir>` reads cmini-style
  JSON counts: from `words.json` every in-word n-gram is exact, and skipgrams across a space are
  estimated assuming consecutive words are independent (spans over 3+ words are dropped);
  word imports scale skip counts ×1000 so small estimates survive rounding (tables are
  normalized separately, so this is harmless). Without `words.json` it reads
  mono/bi/trigrams.json: skip-1 comes from trigram ends (in-word only), skip-2/3 stay empty.
- Default charset: `a–z , . / ; ' - ␣` (33). Frequencies are normalized over the symbols on
  the layout, so 30-key layouts (no `␣`) score exactly as before; layouts with `␣` read as
  "% of all keystrokes incl. space" and are not directly comparable to 30-key numbers.
- Bundled corpora (`gecko compare --corpora english,shai,mt-10k,mt-450k,reddit,french` for the L05 report):

  | Name | Source | Size | QWERTY SFB |
  |---|---|---|---|
  | `english` | 10 Project Gutenberg novels (raw text in `data/corpora/src/`, not committed) | 7.5M chars incl. space | 6.63 % |
  | `shai` | cmini `corpora/shai/words.json` (iweb-based English; the community default) | 520M | 6.70 % |
  | `mt-10k` | cmini `corpora/english-10k` (Monkeytype 10k word list, each word once) | 82k | 8.40 % |
  | `mt-450k` | cmini `corpora/english-450k` (Monkeytype 450k list) | 4.8M | 8.30 % |
  | `reddit` | cmini `corpora/reddit` (Reddit comments; chat style) | 56M | 6.28 % |
  | `french` | cmini `corpora/french`, imported with `--fold` (diacritics → base letter) | 118M | 8.81 % |

  cmini data from https://github.com/Apsu/cmini (GPL-3.0; only aggregated counts are
  committed). Word lists weight every word once, as a Monkeytype test samples them.
  `--fold` (build and import) maps Latin diacritics and ligatures to ASCII first (`é` → `e`,
  `œ` → `oe`, `ß` → `ss`), so non-English text on an ASCII charset counts an accented letter
  as its base key (dead key / compose) instead of breaking the n-gram; without it French
  QWERTY SFB is 8.53 %. Accented keys of their own are future work.
- Shift: case counts need a rebuild with the current tokenizer; the bundled corpora predate
  them (`GKCORP2`), so Shift costs nothing on them until rebuilt (their raw sources are not
  committed). cmini word lists are lowercase.
- `code` corpus (`--charset code`: default + digits, all ASCII punctuation, `⏎`; 70 symbols):
  1:1:1 `gecko corpus mix` of Rust (serde, serde_json, regex-syntax, tokio, axum, clap_builder,
  hyper, rand, proptest from the cargo registry; 7.8M chars), Python 3.14 stdlib top-level
  modules (3.7M) and TypeScript (hls.js `src`, @use-gesture; 1.5M). Only `code.gkc` is
  committed. Rebuild: `gecko corpus build --charset code --ext rs --out code-rs <crate>/src…`
  (likewise `code-py` from `/usr/lib/python3.14/*.py`, `code-ts` from the `.ts` files, not
  `.d.ts`), then `gecko corpus mix code-rs code-py code-ts --out code`.

## Results

A run directory `runs/<timestamp>-<algo>-<short-hash>/` contains:
- `config.json` (params, input names + hashes, full profile, start layout, pins, version)
- `best.json` (resolved params, top-K layouts with full metric breakdown, evals, speed)
- `progress.jsonl` (elapsed, evals, best score and layout, every ~250 ms)

Explore runs add an **atlas** (`explore/atlas.rs`; little-endian, fixed-size records,
rewritten atomically on each snapshot):
- `optima.bin`: `GKATLAS1`, u32 version 1, slots S, metrics M, count N; M metric ids
  (4 bytes, zero-padded); N × {f64 score, u32 hits, u32 first_ms, u16 flags, S × u8 symbol
  ids, M × f32 raw metric values}, sorted by score. Flag 1 = constrained (end of an
  exclusion descent).
- `edges.bin`: `GKEDGES1`, u32 count; × {u32 from, u32 to, u32 count, u8 kick, u8 strength}
  (rows of `optima.bin`; kick 0 = swaps, 1 = columns).
- `samples.bin`: `GKSAMPL1`, u32 M, u32 count; × {f64 score, M × f32 raw} (random layouts).
- `atlas.json`: schema, done, evals, symbols, metric ids, mirror symmetry, profile table
  hash, counts, best, probe summary (random distribution, correlation length), FDC, funnels,
  coverage (Chao1), distinct bests.

Raw metric vectors make the atlas re-rankable under any weights without re-searching (as
long as the profile's tables — `Profile::table_hash` — match).
