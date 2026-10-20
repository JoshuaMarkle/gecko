# Research synthesis

What the sources say GECKO should build next, and why. Each claim links to a summary in
[`sources/`](sources/README.md); each summary links to code, data, or the paper.
Written 2026-10-20 from 16 repositories (cloned and read) and ~20 papers and community docs.

## 1. Where GECKO stands

- SA finds the same best layout from different seeds in 10–30 s
  (`context/algorithms/simulated-annealing.md`). **The objective is now the bottleneck, not the
  search.** Most of the value in the next phase is in better cost models, calibration, and
  ways to present trade-offs.
- Every serious tool converged on the same core: SFB and SFS weighted by distance and finger,
  lateral stretch / scissors, trigram classes (rolls, alternation, redirects), finger and hand
  load. GECKO has these. The differences between tools are in *how* each is weighted and
  defined, and those choices decide which layout wins.

## 2. What the field agrees on

| Topic | Consensus | Source |
|---|---|---|
| Goal | Comfort first; speed gains from layouts are small and debated | [genkey-semimak](sources/genkey-semimak.md), [engram](sources/engram-preference-study.md) |
| SFB | The most important stat. QWERTY 6.6 %, Colemak ≈ 1.6 %, modern < 1 %. > 2 % is high; pinky SFB near 0 | [layouts-doc](sources/keyboard-layouts-doc.md) |
| SFB cost | Weight by distance and by finger (pinky worst, index best). Squared or super-linear distance | [oxeylyzer](sources/oxeylyzer.md), [genkey](sources/genkey-semimak.md), [klo](sources/klo-dariogoetz.md) |
| Skipgrams | Same-finger skipgrams matter: skip-1 ≈ ½ SFB, then halve per extra key | [genkey](sources/genkey-semimak.md), [layouts-doc](sources/keyboard-layouts-doc.md) |
| Distance | From the finger's previous key, not from home | [layouts-doc](sources/keyboard-layouts-doc.md), [kuehlmak](sources/kuehlmak.md) |
| Trigrams | Inroll ≥ outroll > onehand > alternate > redirect > bad redirect (no index) | [oxeylyzer](sources/oxeylyzer.md) v1 weights |
| Trade-off | Rolls vs alternation is a preference axis; more rolls → more redirects | [layouts-doc](sources/keyboard-layouts-doc.md) |
| Ring finger | Least independent finger; ring SFBs and rolls toward ring are bad | [hager-ross](sources/hager-ross-finger-independence.md), [kuehlmak](sources/kuehlmak.md) |
| Balance | Penalize imbalance implicitly with root-sum-square per finger/hand | [kuehlmak](sources/kuehlmak.md) |
| Ranking | Do not trust one total score; show per-metric stats and trade-offs | [layouts-doc](sources/keyboard-layouts-doc.md), [community](sources/community-design-notes.md) |
| Personal | Dexterity, hand size and preference differ; the best layout is per person and per board | [genkey](sources/genkey-semimak.md), [community](sources/community-design-notes.md) |

## 3. Objective model v2 (recommended changes, in order)

All of these fold into the existing C1/C2/C2k/C3 tables, so none slows the hot loop.

1. **Geometric stretch (new B14).** Port Oxeylyzer 2's formula: key rectangles + per-finger
   length offsets, `stretch = hypot(dx, dy) + overlap − 1.35·finger_gap`, counted when > 0.
   Replaces hand-listed LSB and scissor flags and works on any 2D/3D geometry.
   ([oxeylyzer](sources/oxeylyzer.md))
2. **SFB/SFS cost = freq × g(distance) × finger_factor[finger]**, with `g` selectable
   (linear, squared + lateral multiplier, Fitts log) and skipgram gaps 1–3 with `1/2^g`
   decay. ([genkey](sources/genkey-semimak.md), [fitts](sources/fitts-and-movement-models.md))
3. **Finger-pair movement table (B11)** for same-hand different-finger bigrams, seeded from
   KLO's values and the finger-length rule for scissor direction; ring pairs from the
   independence data. ([klo](sources/klo-dariogoetz.md), [hager-ross](sources/hager-ross-finger-independence.md))
4. **Disjoint (across-hand) versions** of SFB/scissor/stretch as skipgram tables.
   ([kuehlmak](sources/kuehlmak.md))
5. **Asymmetric row costs** per finger (top row cheap for middle/ring, bottom row cheap for
   index) — KLO's key-cost matrix and Kühlmak's board types both encode this.
6. **Spread-normalized weights** (already planned): divide each metric by its spread over
   the reference layouts, so weights read as "how many reference-spreads I trade".

## 4. Calibrate the objective with data (the ML part)

The weights are hand-tuned today. Three data sources can replace guesses, each cheap to use:

| Data | What it gives | How | Source |
|---|---|---|---|
| Engram `engram_2key_scores.csv` (1024 position pairs) and choice data | Human comfort per key pair | Load as an independent "engram-comfort" profile; fit GECKO's pair-feature weights to it (logistic / Bradley–Terry) | [engram](sources/engram-preference-study.md) |
| Aalto 136M keystrokes | Timing per bigram for 168k typists | Regress IKI on geometric pair features (same hand, finger pair, dx, dy); export a C2 time table (metric E01) | [dhakal](sources/dhakal-136m-keystrokes.md) |
| cmini's ~4100 community layouts (+ likes) | What the community builds and keeps | Ranking tests; learning-to-rank for profile weights; duplicate detection | [cmini](sources/cmini.md) |

Personal calibration for phase 9, in the web UI: a per-finger tapping test (genkey's KPS
method) and a short pairwise comfort study (Engram's method with active pair selection).
Both produce tables, not code. ([ml-approaches](sources/ml-approaches.md))

Rule from every source: learn **geometric** costs (finger pair, distance, direction), never
per-letter costs, because the data comes from QWERTY typists.

## 5. Search: what is worth adding

SA is good enough for finding the optimum of one profile. Add search only where it changes
what the user gets:

1. **Tabu (RoTS) and Breakout Local Search** as `gecko bench` competitors and as a
   cross-check that SA's optimum is the optimum. ([qap-metaheuristics](sources/qap-metaheuristics.md))
2. **Diversity of results.** MAP-Elites over 2–3 behaviour axes (roll/alternation ratio,
   index load, hand balance) or rank-sum over many short runs, so the UI shows a map of good
   but *different* layouts. Users choose by preference, which the sources say is essential.
   ([search-methods](sources/search-methods-general.md), [kuehlmak](sources/kuehlmak.md))
3. **Certificates.** A lower bound (Gilmore–Lawler / LP relaxation on the bigram part, or MIP
   on sub-problems with HiGHS / CP-SAT) to report "within x % of optimal".
   ([integer-programming](sources/integer-programming-bounds.md))
4. **Adaptive fidelity** for expensive objectives later (big symbol sets, hand-model costs):
   score on top-K n-grams, confirm on the full table before accepting a new best.
   ([kuehlmak](sources/kuehlmak.md))
5. Small wins: depth-2 swap moves for escaping local optima; mirror-symmetry
   canonicalization of results; "move to empty slot" once geometries have spare keys.
   ([oxeylyzer](sources/oxeylyzer.md), [dellamico](sources/dellamico-single-finger-qap.md))

Not worth it now: RL / neural combinatorial search (n = 30 with fast deltas favours classic
local search). ([ml-approaches](sources/ml-approaches.md))

## 6. Robustness and evaluation

- Score on several corpora (Gutenberg, shai, Monkeytype 10k and 450k, chat, code) and report
  the spread (L05). Layouts that win on one corpus can lose on another.
  ([community](sources/community-design-notes.md), [corpora](sources/corpora.md))
- Add shai or a Monkeytype list so GECKO's numbers match what users see in cmini/Oxeylyzer.
- Report **worst words** per metric (Semimak's `you're`, `I'll`), not only worst n-grams.
- Validate trigram classes against cmini's `table.json`; validate pair distances against the
  Layouts Doc examples (`rg` 1.6 u, `my` 2.66 u, `vt` 2.02 u on row-stagger).
- Fingering is a variable: support angle mod for `ansi-30`, and later optional alt-fingering.
  ([layouts-doc](sources/keyboard-layouts-doc.md), [feit-how-we-type](sources/feit-how-we-type.md))

## 7. Geometry and hand model (phases 6–8)

- **Parametric first:** ergogen/Dactyl-style parameters (per-column stagger and splay, spread,
  row/column curvature, tent, split angle, thumb cluster pose) → ~10–20 numbers for CMA-ES.
  ([ergogen-dactyl](sources/ergogen-dactyl.md), [search-methods](sources/search-methods-general.md))
- **Hand model:** Cosmos's closed-form finger IK (MCP 2 DoF + virtual link, phalanges as a
  cyclic quadrilateral) with joint limits added; segment lengths from a MediaPipe hand scan or
  from hand length/breadth. Neutral curl ≈ 5°/4°/9°. ([cosmos](sources/cosmos-keyboards.md),
  [hand-models](sources/hand-models.md))
- **Posture priors:** keyboard slope 0 to −4°, opening angle ≈ 15° total; tent as a free
  parameter with a pronation cost. ([split-ergonomics](sources/split-keyboard-ergonomics.md))
- The geometric stretch metric (§3.1) and IK reach costs make the same scorer work on every
  geometry, which is the core design rule in `context/goals.md`.
- Interop: export to ergogen points and Cosmos expert config to get a buildable board.

## 8. Implementation backlog (suggested order)

| # | Item | Phase | Effort | Sources |
|---|---|---|---|---|
| 1 | Engram comfort profile as an independent judge | 3 | S | engram |
| 2 | Geometric stretch metric B14; retire hand-listed LSB/scissor flags after comparison | 3 | M | oxeylyzer |
| 3 | SFB/SFS distance transforms + per-finger factors + skipgram decay | 3 | S | genkey, oxeylyzer |
| 4 | Extra corpora (shai, Monkeytype 10k/450k) + L05 robustness report + worst words | 3 | S | cmini, corpora |
| 5 | cmini layout importer; ranking tests over many community layouts | 3 | M | cmini |
| 6 | Spread-normalized weights; then fit weights to Engram data | 3 | M | engram, ml-approaches |
| 7 | Tabu (RoTS) + BLS in `gecko bench` | 2 | M | qap-metaheuristics |
| 8 | Result diversity: MAP-Elites or rank-sum over many runs; UI map | 3 | M | search-methods, kuehlmak |
| 9 | Lower bound / MIP oracle for certification | 3 | M | integer-programming |
| 10 | IKI model from 136M data (E01) | 9 | L | dhakal |
| 11 | Personal calibration: tapping test + pairwise comfort study in UI | 9 | M | genkey, engram |
| 12 | Parametric geometry + Cosmos-style IK reach cost | 6–7 | L | ergogen-dactyl, cosmos, hand-models |

## 9. Open questions and things to verify

- Engram study protocol details (which hands, which keyboards) before using its scores on
  column-staggered boards.
- Exact Buchholz segment ratios and joint ROM values before hard-coding the hand model.
- Whether 136M data is downloadable from the user's machine (blocked from the research env).
- carpalx defaults are taken from a mirror's config files; matches the formulas on the archived
  site but individual values are **(verify)**.
- Which skipgram definition GECKO reports (any middle char vs valid chars only) when comparing
  with other analyzers.
