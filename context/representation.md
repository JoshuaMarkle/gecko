# Representation

Three independent inputs: **geometry** (physical board), **layout** (symbols on keys),
**corpus** (n-gram statistics). Parameter lists: `research/parameters.md`.

## Geometry

- TOML in `data/geometries/`. One `[[key]]` per slot, listed row by row, with
  `pos[3], rot[3], size[2], finger, row, col, home, stretch, effort` (hand comes from finger;
  `role` comes with symbol layers). Boards: `ansi-30`, `ortho-30`.
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
- Text format: rows of symbols matching the geometry's rows (`data/layouts/*.toml`), e.g.
  ```
  geometry = "ansi-30"
  rows = ["qwfpbjluy;", "arstgmneio", "zxcdvkh,./"]
  ```
- Stable hash = hash(geometry hash, sym array). Used to de-duplicate results.
- Layers (phase 5): `sym[layer][slot]`; modifiers are keys with a `layer` role.

## Corpus

- Symbols mapped to ids `0..S`. S ≈ 30 now, ≤ 64 with layers.
- Tables (f32, normalized to sum 1): mono `[S]`, bi `[S×S]`, skip1..3 `[S×S]`,
  tri `[S×S×S]` (dense up to S=64 = 1 MB; switch to top-K sparse beyond).
- Built by `gecko corpus build` into `data/corpora/<name>.gkc` (magic `GKCORP1`, name,
  charset, u64 counts for mono, bi, skip, tri). Mixes are weighted sums of normalized tables.
- Tokenization: lowercase; curly quotes → `'`, dashes → `-`; whitespace runs → one space;
  other characters break n-grams. Bigrams/trigrams count only charset runs; skipgrams count
  `x _ z` whatever the middle is. Project Gutenberg headers are stripped.
- Default charset: `a–z , . / ; ' -` (32). Bundled corpus `english`: 10 Project Gutenberg
  novels, 6.1M characters (raw text in `data/corpora/src/`, not committed).
- Uppercase folded by default; Shift modeled only once layers exist.

## Results

A run directory `runs/<timestamp>-<algo>-<short-hash>/` contains:
- `config.json` (params, input names + hashes, full profile, start layout, pins, version)
- `best.json` (resolved params, top-K layouts with full metric breakdown, evals, speed)
- `progress.jsonl` (elapsed, evals, best score and layout, every ~250 ms)
