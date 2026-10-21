# Representation

Three independent inputs: **geometry** (physical board), **layout** (symbols on keys),
**corpus** (n-gram statistics). Parameter lists: `research/parameters.md`.

## Geometry

- TOML in `data/geometries/`. One `[[key]]` per slot, listed row by row, with
  `pos[3], rot[3], size[2], finger, row, col, home, stretch, effort` (hand comes from finger;
  `role` comes with symbol layers). Boards: `ansi-30`, `ortho-30`, `corne-36` (3x5+3),
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
- Positions are always 3D. Flat boards use z = 0 and only z-rotation.
- KLE JSON importer for any existing board (planned).
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
  (zero frequency, may repeat, unpinned unless `--pin '{none}'`). Text input (layout rows,
  `raw:`, `--pin`) also takes `{space}`, `{enter}`, `{bspc}`, `{tab}`, `{esc}`, `{none}`.
  The scorer gives repeated `·` distinct symbol ids.
- Text format: rows of symbols matching the geometry's rows (`data/layouts/*.toml`), e.g.
  ```
  geometry = "ansi-30"
  rows = ["qwfpbjluy;", "arstgmneio", "zxcdvkh,./"]
  ```
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
  skip2, skip3, tri. `GKCORP1` files (no skip2/skip3) still load with those tables zero.
  Mixes are weighted sums of normalized tables.
- Tokenization: lowercase; curly quotes → `'`, dashes → `-`; whitespace runs → one space
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
- Bundled corpora (`gecko compare --corpora english,shai,mt-10k,mt-450k` for the L05 report):

  | Name | Source | Size | QWERTY SFB |
  |---|---|---|---|
  | `english` | 10 Project Gutenberg novels (raw text in `data/corpora/src/`, not committed) | 7.5M chars incl. space | 6.63 % |
  | `shai` | cmini `corpora/shai/words.json` (iweb-based English; the community default) | 520M | 6.70 % |
  | `mt-10k` | cmini `corpora/english-10k` (Monkeytype 10k word list, each word once) | 82k | 8.40 % |
  | `mt-450k` | cmini `corpora/english-450k` (Monkeytype 450k list) | 4.8M | 8.30 % |

  cmini data from https://github.com/Apsu/cmini (GPL-3.0; only aggregated counts are
  committed). Word lists weight every word once, as a Monkeytype test samples them.
- Uppercase folded; Shift not modeled yet.
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
