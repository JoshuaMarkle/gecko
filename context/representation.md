# Representation

Three independent inputs: **geometry** (physical board), **layout** (symbols on keys),
**corpus** (n-gram statistics). Parameter lists: `context/parameters.md`.

## Geometry

- Parametric flat boards: `*.param.toml` zones/rows/columns generate the same `Geometry`
  shape, with stagger, splay, split spread/angle and thumb arcs. See [geometry.md](geometry.md).
  `Store` prefers explicit `name.toml`, then `name.param.toml`; direct parameter paths or
  `name.param` select the generated version. Listing de-duplicates the base name.
- TOML in `data/geometries/`. One `[[key]]` per slot, listed row by row, with
  `pos[3], rot[3], size[2], finger, row, col, home, stretch, effort` (hand comes from finger;
  `role` comes with symbol layers). `effort` and `stretch` are optional: a key without
  `effort`, or a board that marks no key `stretch`, gets them derived from key positions on
  load ([geometry.md](geometry.md#derived-effort-and-stretch-g4)); parametric columns may omit
  `effort` too. Optional `hrm` (bool) marks home-row mod keys for Y04;
  unset = `home` keys on a finger outside the stretch columns (`Geometry::hrm`). Optional
  `alt = [{ finger = "LR", effort = 2.8 }, …]` lists other fingers that may press the key (C11;
  effort derived when missing; not on home keys). Scoring uses `finger`/`effort`; only
  `optimize --fingering` reads `alt` ([objective.md](objective.md#fingering-search-c11)).
  Boards: `ansi-30` (and `ansi-30-angle`, angle-mod fingering; `ansi-30-alt`, angle-mod
  alternatives), `ortho-30`, `corne-36` (3x5+3),
  `sweep-34` (3x5+2), `lily58` (58), `kyria` (50), `ergodox` (76), and
  `dactyl-manuform-5x6` (64). Split boards use pinned QMK flat configurator projections;
  source KLE, provenance and limitations: `data/geometries/sources/README.md`. Corne/Sweep
  stagger now matches those inputs. Dactyl is 3D: its slots follow QMK, its poses come from
  the Dactyl-Manuform generator's math (`sources/dactyl_manuform.py`): concave keywell,
  15° tent, six-key thumb fan.
- Thumb keys use fingers `LT`/`RT` on row 3 (larger boards also have filled auxiliary keys
  on that row; number rows are -1). Optional `fill` per key (token
  name or one char): what the key gets when a layout lists only the keys without a fill, so
  every 30-key reference layout fits all bundled split boards (`Layout::fit`; the CLI and server
  apply it). Thumb fills follow Miryoku: `⎋ ␣ ⇥ | ⏎ ⌫ ·` (36), `␣ ⇥ | ⏎ ⌫` (34). Larger boards use the same five nonempty thumb fills as Corne,
  with all other non-core keys filled `·`; these are comparison defaults, not factory maps.
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
  unrotated top-left corner at the bottom of the keycap; rotation is about the keycap's
  centre (as the 3D viewer draws it). `rot` convention, keycap top / normal and the points
  metrics measure between on 3D boards: [geometry.md](geometry.md#3d-shape) (`gecko_core::pose`).
- Keycaps may not overlap: loading a flat board fails on any overlapping pair
  ([geometry.md](geometry.md#collision-g04), `Geometry::collisions`).
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
  - Rows: unrotated keys sharing an x center form columns, numbered top-down; adjacent
    columns with the same 3+ key count keep that numbering even above 0.5u stagger. Other
    columns shift to match the left neighbour (column stagger); other keys join the row within 0.5u in y.
  - ≥ 4 rows: the last is the thumb row (`LT`/`RT`, row 3) and home is two above it; 3 rows:
    home is the middle one. Row numbers count from the top alpha row (0); a number row is -1.
  - Home row keys under 1.5u split into hands at a gap ≥ 1.5u, else by count (left gets the
    smaller half). From the inside: index stretch (if ≥ 5 per hand), index, middle, ring,
    pinky home, extra pinky columns. Every other key takes the nearest home column's finger
    (ties go left, so ANSI's B is left index); `col` is the position in its row.
  - Efforts: ortho-30's table by row and finger, +1 per row beyond top/bottom, +0.8 per unit
    outside the pinky column; thumbs 1.0 + 0.5 per unit from the hand's rest thumb (nearest to
    half a key inside the index home). No `fill`s: add them for 30-key layouts to fit.
  - `kle::export` writes one KLE row per key (2D only). The original four boards round-trip
    export → import with the same fingers, rows and columns; larger boards retain physical
    keys but need their reviewed annotations restored after import.
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
  `{enter}`, `{bspc}`, `{tab}`, `{esc}`, `{none}`, `{shift}`, `{left}`, `{down}`, `{up}`, `{right}` (arrows `←↓↑→`).
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
- Community layouts: `gecko layouts import-cmini <cmini checkout>` writes one bundle,
  `data/community/cmini.json` (schema 1; one layout per line: `name, author, board, likes,
  rows`, plus `dup_of`/`mirror` and a `skipped` list with `<kind>: <detail>` reasons). Kept
  out of `data/layouts/` so `compare` and `list layouts` stay on the curated references; use
  them as `cmini:<name>`. Only the 30-key core (rows 0–2, cols 0–9) is kept, as written;
  a layout is skipped if a letter sits off the core (thumb letters, a 31st column), a letter
  is missing, or its fingering is non-standard (angle boards may use the angle-mod bottom row;
  `layouts likes` scores them on `<geometry>-angle`, e.g. `ansi-30-angle`, when it exists).
  Empty core keys become `·`. Duplicates: same 30 keys or their left–right mirror; the
  group's representative has the most likes. Likes = number of users in `likes.json`.
- Scoring flattens a layered layout onto `Geometry::layered`: the base keys, then each layer's
  usable keys as virtual keys (`layer`, `phys` set; `layer_keys` = holders, `layer_modes`). Metrics see each
  symbol at its physical key (layer copies of one key are the same key: repeats, not SFBs);
  Y01/Y02 add the layer costs. The flat char string (base, then layer 1's usable keys, …) is
  what runs, `raw:`, `/api/score` and WASM use; `Layout::from_flat` / `fit` split it back
  using the `①②③` positions. Layer keys never move (`Ctx::new`). ≤ 128 slots in all.
- Adaptive keys (`magic.rs`): a key whose output depends on the previous symbol. TOML:
  ```
  rows = ['vmlcpb{magic}uo,', …]
  [[magic]]
  key = "{magic}"          # ★; `{repeat}` = ↻; any glyph not in the corpus charset
  rules = "ao cy dy eu"    # previous → output pairs (tokens allowed: `{space}t`)
  repeat = false           # true: output the previous symbol when it has no rule
  ```
  At most one rule per previous symbol per key; the key's glyph must be on the layout. Rules
  read the previous *output* symbol and the typist always uses the key where a rule applies;
  with several keys the first that produces the symbol wins. Not modeled: multi-symbol
  outputs (briefs such as `␣ → the`) and "magic after magic" overrides. `Layout.magic` holds
  them; the flat char string does not, so runs store them in `config.json` (`magic`) and
  `run:<id>` reattaches them; `raw:` and the web UI/WASM do not carry rules yet. Magic layouts
  live in `data/layouts/magic/` (kept out of the default `compare`, like layered ones).

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
- `data/corpora/<name>.words.tsv`: word counts beside a word-import corpus (for
  `analyze --words`), written by `corpus import` from `words.json`: a `#total\t<n>` line (all
  occurrences), then `word\tcount`, most frequent first, folded like the tokenizer and merged
  (`I` + `i`), capped at 20 000 words. Not part of the `.gkc` or its hash. Bundled for `shai`
  and `mt-10k`.
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
  | `english` | 10 Project Gutenberg novels (raw text in `data/corpora/src/`, not committed; other corpora's sources go in `data/corpora/src/<name>/`, served by `/api/corpora/{name}/sources`) | 7.5M chars incl. space | 6.63 % |
  | `english-cased` | Same 10 novels, rebuilt with the current tokenizer (`GKCORP3`: Shift tables, punctuation as shifted keys) | 7.5M chars incl. space | 6.61 % |
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
- Shift: only `english-cased` has case counts (`GKCORP3`). The other bundled corpora predate
  them (`GKCORP2`), so Shift costs nothing on them. `english` stays frozen because recorded runs
  verify against its hash; use `english-cased` when a layout places `⇧`. cmini word lists are
  lowercase, so their corpora cannot gain Shift counts.
- Adaptive-key rewrite (`magic::rewrite`, run by `Scorer::new` when the layout has keys): each
  key joins the charset and every n-gram moves to its typed form, counts ×1000. A symbol's
  form depends on the one before it, so the tables need one more symbol of context: mono and
  bigrams are exact (from bigrams/trigrams); a trigram's first symbol uses its leading
  bigram's share after a trigger (Markov); skipgram ends use `x ? z` trigrams (gap 1) or a
  Markov chain `skip(g-1)[x,y]·bi[y,z]/mono[y]` (gaps 2–3); skipgram starts use the symbol's
  overall share. Shift tables are retained using the same context shares (estimates). Against the truly rewritten text (research notes, Magic Sturdy rules) the
  score is off by 0.01 (0.06 with an extra repeat key) for a 3.1 effect.
- `code` corpus (`--charset code`: default + digits, all ASCII punctuation, `⏎`; 70 symbols):
  1:1:1 `gecko corpus mix` of Rust (serde, serde_json, regex-syntax, tokio, axum, clap_builder,
  hyper, rand, proptest from the cargo registry; 7.8M chars), Python 3.14 stdlib top-level
  modules (3.7M) and TypeScript (hls.js `src`, @use-gesture; 1.5M). Only `code.gkc` is
  committed. Rebuild: `gecko corpus build --charset code --ext rs --out code-rs <crate>/src…`
  (likewise `code-py` from `/usr/lib/python3.14/*.py`, `code-ts` from the `.ts` files, not
  `.d.ts`), then `gecko corpus mix code-rs code-py code-ts --out code`.
- `code-nav` corpus (`--charset nav` = code + `←↓↑→`; 74 symbols): `code` plus 3 % arrow
  keystrokes from a seeded cursor-movement model (`Corpus::with_nav`; text has no arrows):
  runs of mean length 3 that keep direction 85 % of the time, ← 25 / ↓ 30 / ↑ 20 / → 25 %,
  inserted at random points of a stream typed from `code`'s bigram chain, mixed in at the
  weight that gives the share. Rebuild: `gecko corpus nav code --share 0.03 --out code-nav`.
  Text with arrow glyphs also counts directly under `--charset nav`.

## Results

A run directory `runs/<timestamp>-<algo>-<short-hash>/` contains:
- `config.json` (params, input names + hashes, full profile, start layout, pins, version;
  `holdout` corpus name + hash when set)
- `best.json` (resolved params, top-K layouts with full metric breakdown, evals, speed, and
  `bound`: lower bound + optimality gap of the best layout, `context/algorithms/bounds.md`;
  with a holdout corpus, each layout's `holdout` score and a `holdout` report: start/best on
  both corpora, `retained` share of the gain, `rank_corr` Spearman over the top-K)
- `progress.jsonl` (elapsed, evals, best score and layout, every ~250 ms). Runs of
  independent workers (hc, sa, ga, bls, tabu) add `workers`, one record per
  worker that has stepped (`optimize::WorkerTrace`):
  `{evals, current, best, temp?, proposed, improving, worse, worse_accepted, chars}`.
  `current` / `chars` are the layout the worker is at (SA: current state; BLS: current
  optimum; tabu: current solution; others: their best), on the worker's fidelity level.
  `temp` is the SA temperature (absent elsewhere). The four counts are moves since the
  previous record: proposed, improving, worsening proposed, worsening accepted (SA: single
  swaps; BLS: kick + descent; zero for algorithms that do not count). Memetic, NSGA-II,
  RWSA, explore and qd records have no `workers`.

`gecko bench` writes `runs/bench/<stamp>-<algos>.json` (skip with `--no-save`): schema, id,
created, commit, time_ms, threads, seeds, geometry, corpus, profile, and `results` [{algo,
moves, mean, best, stdev, evals_per_sec, scores, runs: [{seed, score, evals, elapsed_ms,
trace: [[ms, best], …]}]}]. `trace` lists every new best with the time it was reached
(sampled every 20 ms), for time-to-target curves.

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

QD runs (`gecko qd`, `explore/qd.rs`) write a MAP-Elites grid, rewritten atomically on each
snapshot:
- `elites.bin`: same `GKATLAS1` format as `optima.bin`, one record per filled cell, sorted by
  score; `hits` = times the cell's elite was replaced, `first_ms` = when this elite entered,
  flags 0.
- `qd.json`: schema, done, evals, symbols, metric ids, profile table hash, `axes` [{name, lo,
  hi, bins, auto}], `cells`, `filled`, `coverage`, `offspring`, `added`, `improved`,
  `random_mean`, `qd_score`, `best`, and `elites` [{row (into `elites.bin`), cell, bins (per
  axis), desc (per axis), score, chars, updates}], best first. Cell index = Σ bin_k · bins^k
  (axis 0 fastest).

Raw metric vectors make the atlas re-rankable under any weights without re-searching (as
long as the profile's tables — `Profile::table_hash` — match).
