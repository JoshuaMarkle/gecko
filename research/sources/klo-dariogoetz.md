# Keyboard Layout Optimizer (dariogoetz), ArneBab lineage

Rust, GPL-3. https://github.com/dariogoetz/keyboard_layout_optimizer (web UI included).
Metrics descend from ArneBab's `evolve-keyboard-layout` (Neo/Bone community, German).
Configs worth reading: `config/evaluation/default.yml`, `config/keyboard/*.yml`,
`config/optimization/{sa,genetic}.yml`.

## Key ideas

- **Keyboard YAML:** per key `matrix_positions`, physical `positions` (mm), `hands`, `fingers`,
  `key_costs`, `unbalancing_positions` (how far a key pulls the hand off home), `symmetries`
  (mirror index), `finger_resting_positions`. Ready-made boards: crkbd, lily58, moonlander,
  ortho, ANSI/ISO.
- **Per-key cost matrix (crkbd example):**
  ```
  top    25 20  9  5 11 17 | 17 11  5  9 20 25
  home    9  5  3  2  2  8 |  8  2  2  3  5  9
  bottom 25 10 11 11  7 11 | 11  7 11 11 10 25
  thumbs           6  3  6 |  6  3  6
  ```
  Note the asymmetry: top row is cheap for middle/ring, bottom row cheap for index.
- **Finger repeats (SFB):** weight 780; finger factors thumb 1.2, index 0.8, middle 1.0,
  ring 1.1, pinky 1.2; stretch (outward) 1.2, curl (inward) 1.1, lateral 1.5; same key 0.25.
- **Movement pattern** (weight 100): cost per ordered finger pair, e.g. ring→pinky 6,
  middle→ring 5, pinky→ring 4, middle→pinky 3, index→pinky 0.1, ring→index 0.1. Plus
  "short finger down to long finger" / "long up to short" factor 2.0 using finger lengths
  (pinky 1, ring 2, middle 3, index 2). This encodes scissor direction physically.
- **No hand switch after unbalancing key** (36), **no hand switch in trigram** (400, ×2 with
  direction change, ×0.5 if it contains the index), **irregularity** (both bigrams of a trigram
  bad), **secondary bigrams** (1st→3rd key, 0.1).
- **Layers/modifiers:** n-grams are expanded with modifier presses (hold / one-shot /
  long-press costs), so layered symbols are scored by the same bigram/trigram metrics.
- **Learnability metrics:** similar letters (a/ä), letter groups keeping the same relative
  position, shortcut keys in the left columns.
- Normalization per metric: `fixed`, `weight_found` (by found n-gram weight), `weight_all`.
- Search: SA (init_temp 150 or auto, 1 swap/iter, stop after 5000 rejected) and GA
  (pop 1000, 1000 generations, mutation 0.01).

## How to use in GECKO

- The **movement-pattern finger-pair table** and **finger-length scissor rule** are a compact,
  geometry-light alternative to fixed scissor lists; implement as B11 tables.
- Copy the YAML idea of `unbalancing_positions` into the geometry TOML (per-key 2D offset
  from home) for B13.
- Modifier n-gram expansion is the plan for phase 5 (`metrics-catalog.md` Y01); KLO's
  `ngram_mapper.split_modifiers` is a working reference.
- GPL: reimplement, do not copy code.
