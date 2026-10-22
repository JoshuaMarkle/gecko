# Parameters

Three groups: what **describes** a keyboard, what **steers** optimization, and what
**judges** a layout (see `metrics-catalog.md` for the last).

## A. Describing a keyboard

### Per key (geometry)

| Param | Type | Notes |
|---|---|---|
| `id` | u8 | Slot index, stable within a geometry |
| `pos` | [f32; 3] | x, y, z in key units (1u = 19.05 mm). z = 0 for flat boards |
| `rot` | [f32; 3] | Rotation about x, y, z in degrees (tilt, tent, splay). 2D uses only z |
| `size` | [f32; 2] | Width, height in units (1u, 1.25u, 1.5u, 2u…) |
| `hand` | L / R | |
| `finger` | LP…RP, LT, RT | Default finger; may be overridden by a fingering profile |
| `row`, `col` | i8 | Logical grid for reporting and some metrics |
| `home` | bool | Home position for its finger |
| `homing` | bool | Has a homing bump |
| `effort` | f32 | Base cost (from table or hand model) |
| `role` | enum | alpha / punct / mod / thumb / layer, controls what can be placed |

### Per board

| Param | Notes |
|---|---|
| `stagger` | Row stagger (ANSI/ISO/JIS) or column stagger, or none (ortho) |
| `split`, `split_angle` | Split boards, angle between halves |
| `angle_mod` | Alternate fingering for bottom-left (ANSI angle mod) |
| `fingering` | Map from key to finger; several can exist per geometry |
| `layers` | Count and how they are reached (hold, one-shot, toggle) |

### Finger model (2D)

| Param | Notes |
|---|---|
| `strength[finger]` | Relative capacity (index > middle > ring > pinky) |
| `speed[finger]` | Relative speed for distance-weighted SFB |
| `target_load[finger]` | Desired share for M02 |
| `lateral_multiplier` | Extra cost on horizontal movement |
| `row_cost[row]` | Off-home penalty per row (often asymmetric: top easier than bottom for some fingers) |

### Hand model (3D) — see `hand-model-and-geometry.md`

Segment lengths, joint ranges of motion, neutral posture, wrist position and posture,
thumb CMC degrees of freedom, scale from hand length and breadth.

### Layout

| Param | Notes |
|---|---|
| `geometry` | Name/hash of geometry it targets |
| `assign[slot]` | Symbol per slot per layer |
| `locked` | Slots that cannot move |
| `name`, `source` | Known layout or run id |

## B. Steering optimization

### Corpus

| Param | Notes |
|---|---|
| `sources` + `mix_weights` | e.g. 70% English prose, 20% chat, 10% code |
| `charset` | Which symbols are optimized; everything else is dropped or mapped |
| `case_fold`, `shift_model` | Fold uppercase, or count Shift as a key press |
| `max_skip` | Skipgram depth (1–3) |
| `trigram_top_k` | Keep top-K trigrams during search (speed), full set for final report |
| `min_count` | Drop rare n-grams |
| `holdout` | Corpus for generalization checks (L06) |

### Objective

| Param | Notes |
|---|---|
| `profile` | Named weights file |
| `weights[metric]` | Per metric |
| `targets[metric]`, `target_factor` | Soft targets |
| `normalization` | % / per-1000 / z-score against reference layouts |
| `aggregation` | weighted sum / lexicographic / Pareto |

### Constraints

| Param | Notes |
|---|---|
| `pins` | Fixed symbol → slot |
| `allowed[slot]` | Symbol classes allowed per slot |
| `reference` + `max_moved` | Stay close to a known layout |
| `shortcut_keys` | Keep Z X C V on the left bottom row |

### Search

| Param | Applies to | Notes |
|---|---|---|
| `seed` | all | Reproducibility |
| `time_budget` / `max_evals` | all | Stop condition |
| `restarts`, `threads` | all | Independent runs in parallel |
| `neighborhood` | local search | swap, 3-cycle, column swap, row swap, layer swap |
| `T0`, `cooling`, `steps`, `reheat` | SA | T0 scaled to typical swap delta |
| `pop_size`, `crossover`, `mutation_rate`, `selection`, `elitism` | GA | |
| `tenure`, `aspiration` | Tabu | |
| `islands`, `migration_interval` | Island/memetic | |
| `sigma0`, `popsize` | CMA-ES (geometry) | Continuous phases only |

## C. Judging

See `metrics-catalog.md`. Every run report includes: total score, each metric's raw
value, rank against reference layouts, and the worst offending n-grams per metric.
