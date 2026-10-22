---
group: analyzers
kind: tool
read: 2026-10-20
status: read
feeds: [B14, B03, T01-T07, search]
take: Geometric stretch formula for any board; trigram weights; depth-N greedy
---

# Oxeylyzer 1 and 2 (O-X-E-Y)

Rust analyzer + generator, the de facto reference of the AKL community.
v1: https://github.com/O-X-E-Y/oxeylyzer (Apache-2.0). v2 (early, web at https://o2.oxey.dev):
https://github.com/O-X-E-Y/oxeylyzer-2. Layout format `.dof` via the `libdof` crate.

## Key ideas

- **Score = weighted counts** of SFB, SFS, stretches, and trigram classes, with SFB/SFS
  **distance weighted per finger** (v1 `finger_weights`: dexterity, higher = better; v2
  `fingers`: cost multiplier, higher = worse).
- **Trigram classes** (v1 weights show what the community values):
  inroll +2.5, outroll +2.4, onehand +0.9, alternate +0.4, alt-SFS +0.1,
  redirect −3.4, redirect-SFS −4.2, bad redirect (no index) −4.9, bad redirect-SFS −5.5.
- **Max finger use** soft caps (pinky 9%, ring 14%, middle/index 20%, thumb 22%).
- **Geometric stretch metric (v2)** replaces hand-coded LSB/scissor lists. For two keys on
  the same hand, different fingers:
  - each key is a rectangle (x, y, w, h) shrunk by an edge offset; each finger has a
    length offset added to y (pinky −0.15, ring +0.35, middle +0.25, index −0.30, thumb −1.8 u);
  - `dx, dy` = gap between the rectangles; `dx` is negative when fingers cross over;
  - `stretch = hypot(dx, dy) + x_overlap − 1.35·|finger index difference|`; counted if > 0;
  - weight = (bigram + skipgram·(sfb_w/sfs_w)) × stretch × `stretches` weight.
  One formula covers LSBs, scissors and finger crossings on **any geometry**.
- **SFB cost** = freq × distance × finger multiplier (v2 defaults: pinky 77, ring 32,
  middle 24, index 21, thumb 46; weights sfb −7, sfs −1, stretch −3).
- **Search v1:** random start → steepest-descent over all swaps with a per-swap cache, up to
  200 accepted swaps; many restarts in parallel (rayon). Trigrams truncated to the top 1000
  during generation (`trigram_precision`). Pins: pinning a vowel block speeds generation ~2.5×.
- **Search v2:** greedy, "depth-N" greedy (best combination of 2, 3 or 4 swaps when single
  swaps are exhausted), and SA whose acceptance is normalized by the worst score seen:
  `p = exp(((new − cur)/|worst|)/T)`, geometric cooling.
- Corpus configs: `letters_to_lowercase` (models shift), `to_multiple` (expand a char into key
  sequences, e.g. dead keys), `punct_unshifted`.

## How to use in GECKO

- Port the **v2 stretch formula** as metric B04/B05 generalization ("B14 geometric stretch"):
  it needs only key rectangles and finger ids, so it works unchanged on 2D/3D geometries
  (project to the hand plane). Precompute per pair at build time; it folds into C2.
- Use the v1 trigram weights as a sanity reference for GECKO's T01–T07 weights.
- Depth-2 swaps (best pair of swaps) are a cheap escape move for HC/tabu.
- Read the `.dof` format for interop (import/export of community layouts).

## Code / data

- `oxeylyzer-2/core/src/cached_layout.rs` (stretch, dx_dy), `analyze.rs`, `simulated_annealing.rs`,
  `depth_optimization.rs`; v1 `oxeylyzer-core/src/generate.rs`, `config.toml`.
- Corpus: `static/language_data/shai.json` (v1), `data/english.json` (v2). Layouts: many `.dof`.
