# Metrics catalog

The master list of ways to judge a layout. IDs are stable so code, profiles, and
docs can refer to them (e.g. `B01`). **Order** = the n-gram data the metric needs,
which decides how it folds into the scorer (see `context/objective.md`).
**Dir:** `−` penalize, `+` reward, `±` target value.
**Pri:** P0 = MVP, P1 = soon after, P2 = later, P3 = research.

Notation: `f(x)` corpus frequency; keys `i,j,k`; fingers LP LR LM LI LT | RT RI RM RR RP.
"Same hand, different finger" is SH. Rates are reported per 1000 keystrokes or as %.

## 1. Monogram (per key)

| ID | Metric | Dir | Pri | Definition / notes |
|---|---|---|---|---|
| M01 | Key effort (heatmap) | − | P0 | Σ f(c)·effort(slot). Effort table per geometry (carpalx base effort, Oxeylyzer heatmap). |
| M02 | Finger load | ± | P0 | Share per finger vs a target distribution (e.g. index > middle > ring > pinky). |
| M03 | Finger balance (RSS) | − | P1 | √Σ(finger cost²): penalizes uneven load without a separate target (Kühlmak). |
| M04 | Hand balance | ± | P0 | L:R share; flag beyond 60:40. |
| M05 | Row usage | ± | P0 | Home / top / bottom / number row shares. Home row high is good. |
| M06 | Center-column usage | − | P1 | Use of index stretch columns (QWERTY T G B Y H N). |
| M07 | Max finger use | − | P1 | Soft cap on any single finger's share (pinky especially). |
| M08 | Off-home penalty | − | P1 | carpalx-style row/finger/hand penalty per press. |
| M09 | Thumb load | ± | P2 | When thumbs carry letters/space/layer keys. |

## 2. Bigram (consecutive keys)

| ID | Metric | Dir | Pri | Definition / notes |
|---|---|---|---|---|
| B01 | SFB | − | P0 | Same finger, different key. The single most important modern metric. |
| B02 | Same-key repeat (SFR) | info | P0 | Same key twice. Mostly unavoidable; report, don't optimize. |
| B03 | SFB distance (finger speed) | − | P0 | Σ f·dist(i,j)^p / strength(finger). Fspeed in Oxeylyzer/genkey. Lateral distance can get its own multiplier. |
| B04 | LSB (lateral stretch) | − | P0 | Adjacent fingers on same hand with horizontal spread ≥ threshold (usually index center column + middle). Kühlmak weights by inverse finger distance (WLSB). |
| B05 | Full scissor | − | P1 | Adjacent fingers, 2-row jump (QWERTY `ex`, `cr`). |
| B06 | Half scissor | − | P1 | Adjacent fingers, 1-row jump where the shorter finger reaches up (e.g. pinky top, ring bottom). |
| B07 | Bigram roll in / out | + | P1 | SH bigram, direction pinky→index (in) or index→pinky (out). In usually preferred. |
| B08 | Hand alternation | + | P1 | Consecutive keys on different hands. |
| B09 | Weak-finger pair | − | P1 | Pinky–ring adjacency and other awkward finger pairs. |
| B10 | Row jump (same hand) | − | P1 | Vertical distance between SH keys. |
| B11 | Movement pattern cost | − | P2 | KLO-style cost per same-hand finger pair: `pair · direction · (0.1 + rows² + lateral)`. Pair = KLO `finger_switch_factor`, ring pairs × Häger-Ross coupling; direction ×(1+2·Δlength) for short-up/long-down scissors; lateral = columns beyond finger spacing (LSB). Implemented; default weight 0.5 on top of B14. Generalizes B04–B10. |
| B12 | Predicted bigram time | − | P2 | Empirical inter-key interval (IKI) model per key/finger pair, fit from timing data. |
| B14 | Geometric stretch | − | P0 | Oxeylyzer 2 formula on key positions + finger length offsets; covers LSB, scissors, crossings on any geometry. Implemented; replaces B04/B05 in the default profile. |
| B13 | No switch after off-home key | − | P2 | Hand stays after a key that pulls it off home position (KLO). |

## 3. Skipgram (keys 1 and 3, 1–3 apart)

| ID | Metric | Dir | Pri | Definition / notes |
|---|---|---|---|---|
| S01 | SFS / DSFB | − | P0 | Same finger with one key between. Oxeylyzer default weight 0.11 vs SFB per frequency unit. |
| S02 | Decayed skipgrams | − | P1 | Same finger at gaps 2 and 3 with decaying weights (Oxeylyzer fspeed). |
| S03 | SFS distance | − | P1 | Distance-weighted S01, like B03. |
| S04 | Skip stretch (skip LSB / scissor) | − | P2 | B14 stretch on keys 1 and 3 of a trigram (Kühlmak's disjoint bigrams). Implemented; default weight 0.19 spread units (B14's per-unit weight × the S01/B01 ratio 0.12). |
| S05 | Secondary bigram comfort | − | P2 | Movement cost of 1st→3rd key in a trigram (KLO). |

## 4. Trigram

Classify every trigram into exactly one class, then weight classes. This keeps trigram
stats summing to 100% and makes reports easy to read.

| ID | Class / metric | Dir | Pri | Definition |
|---|---|---|---|---|
| T01 | Alternate | + | P1 | Hands L-R-L or R-L-R. |
| T02 | Alternate SFS | − | P1 | Alternate where keys 1 and 3 share a finger. |
| T03 | Roll in | + | P1 | Two SH keys (not SFB) + other hand, inward. |
| T04 | Roll out | + | P1 | Same, outward. |
| T05 | Onehand in / out | ± | P1 | Three SH keys, monotonic direction. Don't penalize by default. |
| T06 | Redirect | − | P1 | Three SH keys changing direction (QWERTY `ads`). |
| T07 | Bad redirect | − | P1 | Redirect with no index finger. |
| T08 | Same-finger trigram | − | P1 | Contains an SFB on both steps. |
| T09 | Contortion | − | P2 | Chained bad bigrams on one hand (Kühlmak). |
| T10 | Irregularity | − | P2 | Both bigrams of the trigram are bad (KLO). |
| T11 | carpalx triad effort | − | P2 | Full carpalx model, as a separate profile. |

## 5. Longer sequences and words

| ID | Metric | Dir | Pri | Definition |
|---|---|---|---|---|
| W01 | Same-hand run length | info | P2 | Average/distribution of consecutive same-hand presses. |
| W02 | Word coverage | ± | P2 | Share of top-N words typeable on home row / with no SFB / one-handed. |
| W03 | Rhythm consistency | − | P3 | Variance of predicted IKIs within words. |

## 6. Empirical / learned

| ID | Metric | Dir | Pri | Definition |
|---|---|---|---|---|
| E01 | Predicted typing time | − | P2 | Σ predicted IKI over corpus, from a fitted timing model (finger pair, distance, hand). |
| E02 | Personal timing cost | − | P3 | Same, fit to the user's own keystroke logs. |
| E04 | Engram formula discomfort | − | P1 | Mean (1 − comfort) over bigrams from Engram's hand-built pair formula. Implemented. |
| E05 | Engram measured discomfort | − | P1 | Same, from a Bradley–Terry fit of Engram's raw pairwise choices (74% held-out accuracy vs 59% for the formula). Implemented; in the default profile. |
| E03 | Learned weights | − | P3 | Profile weights fit so the weighted sum predicts measured speed/comfort. |

## 7. Layout-level (no frequencies)

| ID | Metric | Dir | Pri | Definition |
|---|---|---|---|---|
| L01 | Shortcut placement | − | P1 | Z X C V (and A S Q W) reachable by the left hand / near QWERTY spots. |
| L02 | Familiarity | ± | P2 | Distance from a reference layout: keys moved, keys kept on the same finger/hand. |
| L03 | Grouping / symmetry | − | P2 | Related symbols kept together or mirrored (brackets, vowels, similar letters). |
| L04 | Punctuation constraints | − | P1 | Required positions for `, . ; '` etc. Usually pins, sometimes soft. |
| L05 | Corpus robustness | − | P2 | Score spread across several corpora (prose, chat, code). Report only: `gecko compare --corpora`. |
| L06 | Held-out generalization | info | P2 | Score on a corpus not used in optimization. Guards against overfitting. |

## 8. Layers and symbols (phase: symbol layers)

| ID | Metric | Dir | Pri | Definition |
|---|---|---|---|---|
| Y01 | Layer switch cost | − | P2 | Modifier press cost (hold, one-shot, toggle) expanded into the n-grams. Implemented. |
| Y02 | Symbol n-grams | − | P2 | Code corpora bigrams such as `()`, `->`, `=>`, `!=`. |
| Y03 | Paired symbol layout | − | P2 | Open/close pairs adjacent or mirrored. |
| Y04 | Home-row-mod misfire risk | − | P3 | Fast same-hand rolls through mod keys. |

## 9. Physical geometry (phases: 2D geometry, 3D)

| ID | Metric | Dir | Pri | Definition |
|---|---|---|---|---|
| G01 | Reach cost | − | P2 | Hand-model cost to place fingertip on key (joint deviation from neutral). |
| G02 | 2D/3D travel | − | P2 | Euclidean fingertip travel between key tops (replaces grid distance). |
| G03 | Wrist posture | − | P3 | Pronation, ulnar deviation, extension from tent/splay/tilt. |
| G04 | Collision | hard | P2 | Keycaps/switches may not intersect (OBB in 2D, convex hull in 3D). |
| G05 | Press direction alignment | − | P3 | Angle between key normal and fingertip flexion direction. |
| G06 | Thumb arc fit | − | P3 | Thumb keys along the thumb's natural arc. |
| G07 | Manufacturability | hard/− | P3 | Min spacing, max angles, column counts, plate limits. |

## 10. Aggregation methods

| Method | Use |
|---|---|
| Weighted sum of normalized metrics | Default single objective during search. |
| Soft targets | Weight grows past a target, shrinks below (Kühlmak). |
| Root-sum-square per finger/hand | Penalizes imbalance implicitly. |
| Hard constraints / penalty terms | Pins, collisions, required positions. |
| Rank-sum across objectives | Pick the best trade-off from many generated layouts without weights. |
| Pareto front (NSGA-II) | Show trade-offs explicitly. |
| Lexicographic | E.g. minimize SFB first, then everything else as tie-breaker. |

## Starting profile (P0)

`M01 M02 M04 M05 B01 B03 B04 S01` with weights tuned until the scorer ranks well-known
modern layouts (Graphite, Canary, Colemak-DH, Sturdy) clearly above QWERTY, Dvorak, and
Colemak, and matches public SFB numbers (e.g. QWERTY ≈ 6.6%, Colemak ≈ 1.37%,
Graphite ≈ 1.04% on the corpus used in the Graphite stats).
