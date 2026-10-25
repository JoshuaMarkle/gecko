# Keysets: which symbols a search places

A search permutes the symbols of its start layout. A **keyset** builds that symbol set and a
start layout from the board and the corpus, so you pick a *kind* of layout instead of
hand-writing one (`crates/gecko-core/src/keyset.rs`, `gecko keyset`, `optimize --keyset`).

```
gecko keyset alpha|shift|layer -g corne-36 -c programmer     # preview; --place, --out <name>
gecko optimize --keyset shift  -g corne-36 -c programmer --evals 40000000
```

## Modes

| Mode | Base layer | Other symbols |
|---|---|---|
| `alpha` | 26 letters + special keys; every other key `·` (empty, zero frequency, so the search leaves the worst keys empty) | not on the layout (their n-grams drop out) |
| `shift` | letters + symbol keys, each carrying a **Shift pair** (`-_`, `[{`, …); the most frequent pairs (base + shifted share) fill the free keys | pairs that did not fit |
| `layer` | letters + special keys + a fixed layer key; the most frequent symbols fill the free base keys, then the layer | symbols past the layer's room |

- **Shift pairs** move as one key. `Layout.shifted` (`shifted = ["-_", "[{"]` in TOML; runs store
  them in `config.json` like adaptive rules) makes the scorer type each shifted symbol as its
  base key with Shift (`Corpus::keyed`), so a pair's frequency is both symbols' and the shifted
  one costs a Shift press (only with a `⇧` key). `--pairs natural` (default; US: ``-_ =+ [{ ]}
  \| ;: '" ,< .> /? `~``, plus `1!`… with `--digits`), `frequency` (k most frequent symbols on
  base keys, the next k shifted, paired by rank), or your own: `--pairs "-_ =+ ({ )}"`.
- **Digits** count as symbols in `layer` mode, not in `shift` (`--digits`, `--no-digits`).
- `--symbols N` caps the symbols placed (pair keys in `shift`, all symbols in `layer`).
- `--layer-mode hold|oneshot|toggle` sets how the layer is reached ([objective](objective.md#layers-y01-y02-y03)).
- Scores of different modes are not comparable: frequencies are normalized over the symbols on
  the layout (`alpha` drops punctuation, `layer` adds layer costs).

## Special keys

- Boards with thumb keys get Space `␣`, Backspace `⌫`, Enter `⏎`, Shift `⇧` and Ctrl `⎈` by
  default, plus Tab `⇥` and Esc `⎋` when keys are left (on the layer in `layer` mode). Boards
  without thumb keys (ansi-30, ortho-30) get none: their Space bar and modifiers sit off the
  modelled keys. `--keys space,bspc,enter,shift,shift,ctrl` sets them explicitly (repeat
  `shift` for one per hand).
- Space, Backspace, Enter, Tab and Esc move like symbols. **Shift, Ctrl and layer keys are fixed**
  during a search (the scorer's tables depend on them), so `optimize --keyset` first runs a
  **placement search** (`keyset::place_fixed`, skip with `--no-place`): anneal the layout
  (`--place-evals`, default 2M), then try every fixed key on every candidate key (thumb keys,
  or every key on boards without them), with the displaced symbol taking its old key, each
  followed by a steepest descent; keep the best improving move, re-anneal, repeat (3 rounds
  at most). Layer symbols stranded by a moved hold key move to free layer keys.
- **Key zones (`zones`).** Without a constraint the search treats Space like a letter: on corne-36 it
  put Space on an index home key and letters on the thumbs. Keysets therefore set profile
  `thumb_zone` = the layout's `␣⌫⏎` (letters, digits and punctuation off the thumbs, those
  three on them) and, with a layer, `base_letters` (letters and the zone keys on the base
  layer), with `zones` weighted 100 (a constraint). `--thumb-letters` drops the thumb zone.
  Symbols are then placed only on non-thumb keys, so corne-36 `shift` gets 4 pair keys and Tab
  on the sixth thumb.
- Keysets weight **`layer_switch`** (layer, Shift and Ctrl presses) at the `code` profile's weight (1,
  scale 2) when the profile leaves it at 0, so the modifiers cost more than their key's effort.
  The CLI prints every profile change; the run's `config.json` records the profile used.

## Modelled keys

- **Shift**: per press, the best `⇧` key's effort (`effort`) and a `layer_switch` press, doubled on the shifted
  key's finger ([objective](objective.md#layers-y01-y02-y03)). Needs case counts (`GKCORP3`) or
  Shift pairs.
- **Ctrl** (`⎈`, `{ctrl}`): profile `ctrl` shortcut presses per keystroke (default 0.01, an
  assumption) spread over `profile::CTRL_SHORTCUTS` (c v 20 %, z 12 %, s 10 %, …, an estimate),
  costed like Shift's press (effort + `layer_switch`). Folded into the symbol-on-slot table, so the hot loop
  is unchanged. Only layouts with a `⎈` key pay it.
- **Backspace**: text has none. `gecko corpus edits <corpus> --share 0.056 --out <name>` adds `⌫`
  from a typing-error model fitted on the Aalto 136M keystrokes study (4,000 participants,
  2.8M keys): `⌫` is 5.6 % of keystrokes in 3.0 runs per 100 keys, runs of 1/2/3 keys 55/25/10 %
  (geometric, continuation 0.45). Each run follows as many wrong keys, then the text resumes.
- **Enter**: counts with `⏎` in the charset. Prose sources are hard-wrapped, so `corpus build
  --paragraphs` makes a line break a space and a blank line one `⏎`.

## Corpora for keysets

| Corpus | What | Why |
|---|---|---|
| `writer` | English prose (web, chat, books) with every ASCII symbol, digits, case, `⏎`, `⌫` 5.6 % | Writing-first layouts |
| `programmer` | `writer`'s prose 45 + Markdown 10 + code (Rust, Python, JS/TS, C, shell, Lua) 45, `⌫` | Symbol layers and pairs for coding |
| `vim-programmer` | `programmer` + 22 % Vim sessions (`⎋`, motions, `:`), `⌫` | Vim users; see [vim.md](vim.md) for h j k l policies |

Languages are recipes over component corpora ([representation.md](representation.md#corpus));
make your own mix in the web Language tab or with `gecko corpus mix`. The retired
`english-full-bspc` (Results below) is in `data/corpora/legacy/`.

The reference corpora (`english`, `shai`, `reddit`, `french`) fold shifted punctuation into its US key already (`:` is `;` + Shift), so
on them `shift` mode can only choose among `, . / ; ' -`.

## Not modelled

Ctrl usage is not measured (typing tests have none); one `⎈`/`⇧` cost table per key, not per
hand posture; a pair key's shifted symbol cannot sit on another layer; the web editor's live
score ignores Shift pairs (runs and `/api/score` with `shifted` carry them).

## Results

corne-36, `english-full-bspc`, profile `default` (+ the keyset changes above), SA 40M evaluations,
seed 1, placement on (2M evaluations per settle). "Placement off" is the same run with
`--no-place` (fixed keys stay on the heuristic start keys).

| Mode | Run | Score | Placement off | Thumbs (L outer → R outer) |
|---|---|---|---|---|
| `alpha` | `20261024-211112-sa-30ac26bd` | 18.063 | 18.120 | `⎋ ⌫ ⇧ ⎈ ␣ ⏎` |
| `shift` | `20261024-211141-sa-1a596d4e` | 20.128 | 20.193 | `⇥ ⌫ ⇧ ⎈ ␣ ⏎` |
| `layer` | `20261024-211219-sa-d2b21e1a` | 20.585 | 20.592 | `⇧ ⌫ ① ⎈ ␣ ⏎` |
| `layer`, `-p code` | `20261024-211259-sa-957cf07a` | 20.952 | | `⏎ ␣ ① ⇧ ⌫ ⎈` |

```
alpha                      shift (⇧ ,< .> -_ '")      layer (base; [sym] below)
j u o f · z v n d w        w l m d b ' x u o j        w l m d b < x o u j
i a e s g l h r t p        n r t s g p h e a i        n r t s g p h e a i
y x · k · q m ⇥ b c        q z v f c k y - . ,        q z v f c k y * . ,
⎋ ⌫ ⇧ ⎈ ␣ ⏎                ⇥ ⌫ ⇧ ⎈ ␣ ⏎                ⇧ ⌫ ① ⎈ ␣ ⏎
```

- Space, Backspace and Enter stay on the thumbs (`zones` = 0); Space lands on the right middle
  thumb, Backspace on the left middle one. `alpha` leaves a top-row middle key and two bottom-row keys empty.
- The placement search moves Shift and Ctrl from the outer thumbs (the heuristic start) to the
  inner ones. It is worth only 0.06 at these weights (Shift ≈ 2.4 presses per 100 keys from
  capitals and pairs, Ctrl 1 per 100): inside the 0.4 near-tie band, so the thumb arrangement is
  a tier, not an optimum. It costs about 10 s per run (24 candidates × descent, 2 re-anneals).
- `shift` mode picks `,< .> -_ '"` for the 4 free keys; `/?` and `;:` (0.4 % each) are left out.
- `layer` with the default profile keeps letters, `, .` on the base and moves two rare symbols
  (`< *`) onto the remaining base keys while `' -` take cheap layer thumb keys: the default
  profile does not weight `symbol_bigram` and a layer thumb key costs little. With `-p code` (`symbol_bigram` weighted)
  on prose, `. , '` join `"` on the layer to keep dialogue bigrams (`."`, `,"`) on one layer,
  and digits take base keys: `code`'s `symbol_bigram` suits code, not dialogue-heavy prose.
- ansi-30 (no thumbs, no modifiers): `alpha` 24.516 (`20261024-210635-sa-edd119e9`), `shift`
  27.643 (`20261024-210654-sa-d9afa02f`, pairs `,< .> -_ '"`).
