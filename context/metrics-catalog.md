# Metrics catalog

The master list of ways to judge a layout. Each metric has a stable **ID**, a short snake_case
name (e.g. `sfb`) that code, profiles, the CLI and JSON use. IDs use the words layout people
already use (SFB, SFS, LSB, scissor, redirect). **Old** is the metric's code before 2026-10-24
(`B01`, …): still accepted wherever an ID is read (profiles, geometries, runs, atlases, CLI
flags), and kept in `gecko list metrics`. Older runs, `bench/`, `research/` and `docs/papers/`
use the old codes; this table translates them.

**Order** = the n-gram data the metric needs, which decides how it folds into the scorer (see
`context/objective.md`). **Dir:** `−` penalize, `+` reward, `±` target value, `info` report only.
**Retired** = kept only so old runs and profiles still score (`retired` in `gecko list metrics`).

Notation: `f(x)` corpus frequency; keys `i,j,k`; fingers LP LR LM LI LT | RT RI RM RR RP.
"Same hand, different finger" is SH.

## Same finger

| ID | Old | Order | Dir | Definition / notes |
|---|---|---|---|---|
| `sfb` | B01 | bi | − | Same finger, different key. The single most important modern metric. |
| `sfs` | S01 | skip | − | Same finger with one key between (space too). |
| `sfs_far` | S02 | skip-far | − | Same finger at gaps 2 and 3, decayed per extra key (profile `sfb.skip_decay`, Oxeylyzer). |
| `sft` | T08 | tri | − | All three keys on one finger. |
| `alt_sfs` | T02 | tri | − | Alternating trigram (L-R-L) whose keys 1 and 3 share a finger. |
| `repeat` | B02 | bi | info | Same key twice. Mostly unavoidable; reported, not optimized. |
| `sfb_distance` | B03 | bi | retired | Σ f·g(dist) × finger factor (Oxeylyzer/genkey fspeed). Retired 2026-10-22: keystroke timing shows no distance effect beyond `sfb`. |
| `sfs_distance` | S03 | skip | retired | Distance-weighted `sfs`, like `sfb_distance`. Retired with it. |

## Stretches and scissors

| ID | Old | Order | Dir | Definition / notes |
|---|---|---|---|---|
| `stretch` | B14 | bi | − | Oxeylyzer 2 formula on key positions + finger length offsets; covers LSB, scissors and crossings on any geometry. Per 100 bigrams. |
| `skip_stretch` | S04 | skip | − | `stretch` between keys 1 and 3 of a trigram (Kühlmak's disjoint LSB / scissors). S05 (KLO secondary bigram) built the same table and is an alias of this one. |
| `contortion` | T09 | tri | − | Both steps of a trigram have `stretch`; cost = their sum (Kühlmak). |
| `lsb` | B04 | bi | − | Lateral stretch: index on a center-column key next to the middle finger. |
| `scissor` | B05 | bi | − | Adjacent fingers, same hand, two-row jump (QWERTY `ex`, `cr`). |
| `half_scissor` | B06 | bi | − | Adjacent fingers, one-row jump with the shorter finger on the upper row (length order pinky < index < ring < middle). |

## Motion

| ID | Old | Order | Dir | Definition / notes |
|---|---|---|---|---|
| `row_jump` | B10 | bi | − | 100·\|Δlift\| per same-hand bigram, thumbs excluded; lift = y minus the y of the finger's home keys (the hand's when the finger has none), so a column stagger along finger length is no jump. |
| `pinky_ring` | B09 | bi | − | Pinky–ring bigrams on one hand. |
| `movement` | B11 | bi | − | KLO-style cost per same-hand finger pair: `pair · direction · (0.1 + rows² + lateral)`. Pair = KLO `finger_switch_factor`, ring pairs × Häger-Ross coupling; direction ×(1+2·Δlength) for short-up/long-down scissors; lateral = columns beyond finger spacing. |
| `irregular` | T10 | tri | − | Both bigrams of the trigram are bad (KLO): `sfb`, `lsb`, `scissor` or `half_scissor`. |

## Rolls and alternation

Trigram classes (`alternate` … `thumb_alt`, `alt_sfs`, `sft`) are exclusive: every trigram has
exactly one, so they sum to 100 % with "other" (contains an SFB or a repeat).

| ID | Old | Order | Dir | Definition / notes |
|---|---|---|---|---|
| `hand_switch` | B08 | bi | + | Consecutive keys on different hands. |
| `bigram_inroll` | B07 | bi | + | SH bigram moving toward the index finger. Outward = same-hand − `sfb` − in. |
| `alternate` | T01 | tri | + | Hands L-R-L or R-L-R, keys 1 and 3 on different fingers. |
| `inroll` | T03 | tri | + | Two SH keys (not SFB) rolling toward the thumb + one other-hand key. |
| `outroll` | T04 | tri | + | Same, rolling away from the thumb. |
| `onehand` | T05 | tri | info | Three SH keys in one direction. |
| `redirect` | T06 | tri | − | Three SH keys changing direction (QWERTY `ads`). |
| `bad_redirect` | T07 | tri | − | Redirect without the index finger. |
| `thumb_roll` | T12 | tri | + | Trigram with a thumb key next to a same-hand key (other finger). |
| `thumb_alt` | T13 | tri | + | Trigram with a thumb where every step into or out of the thumb changes hands. |

## Keys and load

| ID | Old | Order | Dir | Definition / notes |
|---|---|---|---|---|
| `effort` | M01 | mono | − | Σ f(c)·effort(slot): geometry base effort (carpalx, Oxeylyzer heatmap) plus profile `row_offset` per finger (KLO asymmetry), layer key effort, and Shift/Ctrl key effort per press. |
| `home_row` | M05 | mono | + | Share of keystrokes on the home row. |
| `center_cols` | M06 | mono | − | Share on the index stretch columns (QWERTY T G B Y H N). |
| `finger_target` | M02 | load | ± | RMS gap between finger loads and the profile's `finger_target` distribution. |
| `finger_evenness` | M03 | load | − | √Σ share² over non-thumb fingers (Kühlmak); 35.4 = perfectly even. |
| `finger_peak` | M07 | load | − | Highest non-thumb share as % of its `finger_target` (100 = none over target). |
| `hand_balance` | M04 | load | ± | Left-hand share; scored as squared distance from 50 %. |
| `thumb_load` | M09 | load | ± | Thumb share; scored as squared distance from profile `thumb_target`. |

## Hand model (physical geometry)

Per keystroke, from the hand model (`context/hand-model.md`).

| ID | Old | Order | Dir | Definition / notes |
|---|---|---|---|---|
| `reach` | G01 | mono | − | Joint deviation from the natural curl + splay + fingertip shortfall, hand placed on the home keys. |
| `press_angle` | G05 | mono | − | (angle / 45°)² between the fingertip's press direction and the keycap normal (fingers: distal phalanx within the finger plane; thumbs: push along CMC → tip). |
| `wrist` | G03 | mono | − | Resting hand vs forearm (elbows from profile `hand.body`; split halves aligned with the forearm): ulnar/radial deviation, extension, pronation, squared over comfortable ranges, plus the deviation of sideways out-of-reach keys. |
| `thumb_arc` | G06 | mono | − | (key-plane distance from the tip's CMC flexion sweep, cut to the 73 mm operational workspace / 10 mm)² per thumb key; 0 on finger keys. `--thumb-keys auto` sizes clusters to the arc. |

## Layers and symbols

| ID | Old | Order | Dir | Definition / notes |
|---|---|---|---|---|
| `layer_switch` | Y01 | bi | − | Layer-key presses (hold, toggle, one-shot) expanded into the n-grams, plus Shift and Ctrl presses; double when the modifier's finger also types the key. |
| `hrm_misfire` | Y04 | bi | − | % of bigrams whose first key is a home-row mod (geometry `hrm`, default = home keys outside the stretch columns) and whose second is another finger of the same hand. |
| `symbol_bigram` | Y02 | sym-bi | − | Code-symbol bigrams (`()`, `->`, `!=`, …) typed with one finger on two keys or across a layer change, % of symbol bigrams. |
| `brackets` | Y03 | pairs | − | Open/close pairs `() [] {} <>` adjacent or mirrored on one layer, by symbol frequency; reversed order = half cost. |

## Placement (layout-level, no n-grams)

| ID | Old | Order | Dir | Definition / notes |
|---|---|---|---|---|
| `shortcuts` | L01 | place | − | Z X C V (and A S Q W at half weight): 100 off the left hand, up to 50 on it by distance from the QWERTY spot. Per shortcut letter. |
| `familiar` | L02 | place | ± | Distance from profile `familiar_to = "qwerty"`, per keystroke: 0 on the reference key, 50 same finger, 75 same hand, 100 otherwise. |
| `vim` | L03 | pairs | − | h j k l keep the profile `vim` shape: `row` in order on one row, `pairs` h←l and j←k on neighbours, `fingers` four fingers of one hand. % of the policy's pairs broken (reversed neighbours half). [vim.md](vim.md). |
| `zones` | L04 | place | − | Count of keys out of their zone (soft pins): profile `thumb_zone` (those symbols on thumb keys, letters/digits/punctuation off them) and `base_letters` (letters off layer keys). Keysets weight it 100. Fixed positions remain `--pin`. |

## Comfort and timing models

Judges fitted to measured data. Weight fits (`gecko profile fit`, `calibrate::rank`) keep them
out of the features (`metrics::is_model`).

| ID | Old | Order | Dir | Definition / notes |
|---|---|---|---|---|
| `engram` | E05 | bi | − | Mean (1 − comfort) from a Bradley–Terry fit of Engram's raw pairwise choices (74 % held-out accuracy vs 59 % for the formula). Geometry `comfort.engram`. In the default profile. |
| `engram_formula` | E04 | bi | − | Same, from Engram's hand-built pair formula. Geometry `comfort.engram_formula`. |
| `typing_time` | E01 | bi | − | Σ predicted extra interval over bigrams, % of a median keystroke interval, from a geometric model fitted to fast touch typists (Aalto). A judge: weight 0 in bundled profiles (`gecko judge`; `context/objective.md`). |

## Not yet metrics

Ideas and non-scored checks. They get an ID when implemented as a metric. The old code is
given so older notes can be traced.

| Old | Idea | Status |
|---|---|---|
| M08 | carpalx-style row/finger/hand penalty per press | covered by `effort` + `row_offset` |
| B12 | Empirical inter-key interval model per key/finger pair | done as `typing_time` |
| B13 | No switch after an off-home key (KLO) | open |
| T11 | Full carpalx triad effort, as a separate profile | open |
| W01 | Same-hand run length distribution | open |
| W02 | Share of top-N words on the home row / with no SFB / one-handed | open |
| W03 | Variance of predicted intervals within words | research |
| E02 | Timing model fit to the user's own keystroke logs | research |
| E03 | Profile weights fit to measured speed/comfort | a command: `gecko profile fit --target engram` / `likes` |
| L05 | Score spread across corpora | a report: `gecko compare --corpora` |
| L06 | Score on a held-out corpus | a report: `--holdout` |
| L07 | Related symbols grouped or mirrored | open; `vim` and `brackets` cover Vim keys and brackets |
| G02 | Euclidean fingertip travel between key tops | open |
| G04 | Keycap/switch collision | a hard check (`context/geometry.md`) |
| G07 | Manufacturability | a feasibility check, profile `[build]` (`context/geometry.md`) |

## Aggregation methods

| Method | Use |
|---|---|
| Weighted sum of normalized metrics | Default single objective during search. |
| Soft targets | Weight grows past a target, shrinks below (Kühlmak). |
| Root-sum-square per finger/hand | Penalizes imbalance implicitly. |
| Hard constraints / penalty terms | Pins, collisions, required positions. |
| Rank-sum across objectives | Pick the best trade-off from many generated layouts without weights (`gecko select`). |
| Pareto front (NSGA-II) | Show trade-offs explicitly. |
| Lexicographic | E.g. minimize SFB first, then everything else as tie-breaker. |
