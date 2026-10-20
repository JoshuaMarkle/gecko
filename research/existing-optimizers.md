# Existing optimizers and analyzers

What each tool does, how it judges a layout, how it searches, and what GECKO takes from it.

## Summary

| Tool | Lang | Judging model | Search | Take for GECKO |
|---|---|---|---|---|
| carpalx | Perl | Triad effort: base + penalty + stroke path | Simulated annealing | Effort profile; SA schedule; triad weighting |
| Oxeylyzer | Rust | SFB/DSFB/fspeed, scissors, LSB, trigram classes, heatmap | Generate + improve from random, pins | Modern metric set; trigram classification; pins |
| genkey | Go | Weighted metrics, finger speed (Semimak) | Generator + "improve" | Speed-weighted finger travel |
| Keyboard Layout Optimizer (KLO, dariogoetz) | Rust | Many metrics (ArneBab/Neo lineage), layers, modifiers | SA and GA | Layer/modifier model; ngram expansion for layers; YAML configs |
| evolve-keyboard-layout (ArneBab) | Python | Neo community metrics | Evolutionary | Origin of KLO metrics |
| Kühlmak | Rust | Rolls, WLSB, scissors, SFB, contortions, RSS balance, constraints | Multi-threaded SA + rank-sum multi-objective | Soft targets, RSS balance, rank-sum ranking, constraint terms |
| keynergy | Rust lib | Library for custom metrics + n-gram stats | n/a | API ideas |
| Feit / Oulasvirta / Karrenbauer (academic) | — | Assignment-problem objectives (ergonomics, familiarity, mnemonics) | Integer programming, lower bounds | Exact bounds for small/sparse QAP; French standard case study |
| Cosmos, Dactyl-Manuform, ergogen | TS / Clojure / JS | Physical geometry generation | Manual/parametric | Geometry description, hand scans, collision checks |

## carpalx

- Scores **triads** (3-char substrings). Effort per triad combines three components,
  each weighted (kb, kp, ks), and the original optimized layouts used a 1:1:1 ratio
  of base : penalty : stroke.
  - **Base effort:** depends on finger travel distance to each key.
  - **Penalty:** hand, row, and finger penalties (off-home rows, weak fingers).
  - **Stroke path:** hand alternation, row jumping, finger rolling across the triad.
- Triad composition is geometric:
  `effort = k1·e(x)·[1 + e(y)·(k2 + k3·e(z))] + k4·path(x,y,z)`
  (from carpalx source comments, via the antibracket mirror). Default k values: **(verify)**.
- Search: simulated annealing with exponential cooling `T(i) = T0·exp(-k·i/N)`.
  The author stresses that T0 must be scaled to the range of effort scores. QWERTY
  scores about 9 and optimized layouts about 4.5–5 under carpalx's model (mod_01).
- Weakness: no explicit SFB/skipgram handling. Modern layouts beat carpalx outputs on
  SFB and rolls. Use as one **profile** for comparison, not as the default.

## Oxeylyzer

- Analyzer + generator (CLI REPL and Tauri GUI). 30 keys per language from a config file.
- Metrics:
  - **Heatmap:** penalizes frequent keys in bad positions; preset per `keyboard_type`
    (Ansi, Iso, JIS, Rowstag, Ansi/Iso Angle, Ortho, Colstag).
  - **Fspeed:** weighted sum of SFBs, DSFBs, and weaker skipgrams (up to 3 chars in
    between), accounting for distance and finger strength.
  - **Lateral penalty:** multiplier on lateral distance inside fspeed (default 1.0).
  - **DSFB ratio:** default 0.11 relative to SFB.
  - **Scissors:** adjacent fingers jumping two rows (hand-tuned exceptions).
  - **Trigram classes:** inroll, outroll, onehand, alternate, alternate-SFS, redirect,
    bad redirect (no index finger).
  - **Max finger use:** soft cap on any finger's share.
- Search: generate from random or improve an existing layout. Pins lock keys (pinning
  vowels speeds generation a lot). Uses the top ~1000 trigrams during generation.

## genkey (semilin)

- Analyzer/generator used to create Semimak. Customizable weights, heatmaps, interactive
  analysis, layout "improve" mode.
- Known for **finger speed**: distance-weighted same-finger usage scaled per finger.

## Keyboard Layout Optimizer (dariogoetz) and ArneBab's evolve

- Metric groups:
  - Key/finger: key costs, finger repeats, finger balance, hand disbalance.
  - Bigram: **movement pattern** (cost per finger pair, direction, row change),
    no hand switch after an unbalancing key.
  - Trigram: irregularity (both bigrams bad), secondary bigrams (1st+3rd key),
    no hand switch with direction change.
  - Layout-only: badly positioned shortcut keys.
  - Learnability: similar letters / letter groups placed inconsistently.
  - Compatibility: re-implemented KLAnext metrics; word-based metrics (top 30k words
    typed without finger repeats or only on home row).
- Layers and modifiers (hold, one-shot, long-press, home-row mods). Higher-layer chars
  are handled by **expanding n-grams with the modifier presses**. Good model for our
  symbol-layer phase.
- Search: SA (best from scratch, many starts) and GA (good for staying near a start).
- Speed claim: ~100 ms per full evaluation with trigrams on large corpora. This is the
  "full re-evaluation" cost GECKO avoids with delta evaluation.

## Kühlmak (fxkuehl)

- Metrics per 1000 keystrokes: down/up rolls, weighted LSB (by inverse finger distance),
  scissors, SFB (travel ×4), alternating skipgrams (disjoint SFB ×2), same-hand trigrams
  split into rolls / redirects / **contortions**, travel, effort, hand imbalance.
- **Balance via root-sum-of-squares:** per-finger costs are squared, summed, and rooted,
  so imbalance is penalized without a separate balance metric.
- **Soft targets:** each stat can have a target; a global factor boosts weight beyond
  target and relaxes it below.
- **Constraints** as an added penalty: ZXCV placement, non-alpha in fixed spots,
  similarity to a reference layout, homing keys, keys allowed per row.
- Search: multi-threaded SA from random starts; schedule length trades accuracy
  for variety. Hundreds of thousands of layouts per run.
- **Rank-sum multi-objective selection:** generate many layouts, rank each objective,
  sum ranks. No per-objective weights needed. Disproportionate sacrifices rank worse.

## Academic work

- Keyboard layout is a generalized **Quadratic Assignment Problem** (Dell'Amico et al.
  2009, single-finger keyboard). Trigram terms make it cubic.
- Feit (Aalto, 2018 dissertation, with Oulasvirta): assignment-problem objectives for
  text input including ergonomics, familiarity, and mnemonics; contributed to the
  **French keyboard standard** (special characters, 100+ symbols).
- John & Karrenbauer (MOTOR 2019): iterative method for **sparse QAP** giving a
  hierarchy of **lower bounds** plus heuristic solutions. Useful to tell how far a
  layout is from optimal on small or sparse problems.
- Dhakal et al. (CHI 2018), 136M keystrokes from 168k typists: bigrams typed by
  different hands or fingers predict speed better than letter repetitions; rollover
  typing is common. A source for **empirical timing costs**.

## Physical keyboard generators

- **Cosmos** (rianadon, TypeScript): web configurator for curved split keyboards,
  stepless curvature, thumb cluster presets, flags intersecting keycaps, supports
  hand scans. Geometry calc separate from CAD (OpenCascade) modeling.
- **Dactyl / Dactyl-Manuform** (Clojure → OpenSCAD): parametric curved columns.
- **ergogen** (YAML): zones → columns → rows with stagger, splay, rotation. A clean
  declarative model for "points" that our geometry format can mirror.

## Sources

- https://github.com/O-X-E-Y/oxeylyzer
- https://github.com/kuuuube/genkey (mirror of semilin/genkey)
- https://github.com/dariogoetz/keyboard_layout_optimizer
- https://github.com/fxkuehl/kuehlmak
- https://docs.rs/keynergy
- https://mkweb.bcgsc.ca/carpalx/ (carpalx docs; unreachable during research)
- https://github.com/pianohacker/antibracket/blob/master/generation/carpalx/effort/README
- https://forum.colemak.com/post/10473/ (carpalx author on SA parameters)
- https://gitlab.freedesktop.org/xkeyboard-config/xkeyboard-config/-/merge_requests/725 (Graphite stats)
- https://aaltodoc.aalto.fi/items/5c7322e4-b490-419d-9a1a-d848f5a851d6 (Feit dissertation)
- https://resources.mpi-inf.mpg.de/keyboardoptimization/MOTOR2019/MOTOR2019.pdf
- https://userinterfaces.aalto.fi/136Mkeystrokes/
- https://github.com/rianadon/Cosmos-Keyboards
