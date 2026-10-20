# GECKO — Design Draft v0 (historical)

> Superseded by `context/`. Kept for history only.

---

## 1. Where we are starting

- Empty repo at `~/projects/gecko`. No code, no data, no decisions locked in.
- Machine has `python3` and `node`; no Rust/Go toolchain installed yet.
- Prior art we can learn from (worth verifying each before we lean on it):
  - **carpalx** (Perl): effort-based model + simulated annealing. The classic.
  - **Oxeylyzer** (Rust): fast analyzer + generator, modern metrics (SFB, rolls, redirects...).
  - **genkey** (Go): analyzer/generator used by the alt-layout community.
  - **dariogoetz/keyboard_layout_optimizer** (Rust): weighted-metric optimizer, ArneBab/Neo lineage.
  - Layouts to benchmark against: QWERTY, Dvorak, Colemak, Colemak-DH, Workman, Halmak,
    Canary, Graphite, Sturdy, etc.

### What the problem actually is

Assigning N symbols to N key slots to minimize a cost built from bigram (and trigram)
frequencies is a **Quadratic Assignment Problem** (cubic once trigrams matter). It is
NP-hard, there is no gradient, and the search space for 30 keys is 30! ≈ 2.6 × 10³².
That framing drives most of the design:

- The thing to make fast is **"evaluate a swap"**, not "evaluate a layout".
- The best-known tools for QAP are local-search metaheuristics (simulated annealing,
  tabu search, memetic GA + local search), so that is the algorithm roadmap.
- "Machine learning" in the strict sense enters later, mainly for **learning the
  objective** (e.g. from your own typing data) and surrogate models, not for the search itself.

---

## 2. Goals

**Primary:** find the best possible layout for a given *geometry*, *corpus*, and
*objective profile* — and be able to explain *why* it's best (per-metric breakdown).

**Secondary:**
1. **Fast.** Tens of millions of swap evaluations per second on a laptop, multi-core.
2. **Extensible objective.** Adding a metric must not slow the hot loop or require
   touching optimizer code.
3. **Reproducible.** Every run = (geometry, corpus, profile, algorithm, params, seed,
   code version) → same result. Runs are saved and comparable.
4. **Two front doors.** A scriptable CLI (for Claude to iterate quickly) and a web UI
   (for you to explore, compare, and view layouts in 3D).
5. **2D now, 3D later** without rewriting the core.
6. **Maintainable.** Small, testable modules; a `context/` directory that keeps
   design knowledge next to the code.

**Non-goals (for now):** firmware generation, real-time typing tests, cloud hosting.

---

## 3. Language choice

| Option | Speed | Web story | Maintainability | Notes |
|---|---|---|---|---|
| **Rust** | Excellent | Compiles to WASM; same scorer runs in browser | Strong types, cargo, great testing/bench tooling | What Oxeylyzer & dariogoetz use. Learning curve. |
| C++ | Excellent | Emscripten, painful | Build systems + footguns | Not worth it here. |
| Go | Good (GC, weak SIMD) | WASM is heavy | Very simple | genkey uses it. Fine, not great. |
| Zig | Excellent | Good WASM | Young ecosystem | Fun, risky. |
| Python (+Numba/NumPy) | OK→good with effort | None (needs a server) | Easiest to prototype | Great for analysis notebooks, not the core loop. |
| TypeScript only | Poor for search | Native | Easy | Only for the UI. |

**Recommendation: Rust core + TypeScript web front end.**

- Rust for everything performance-sensitive (`gecko-core`), the CLI, and a small HTTP
  server. `rayon` gives free parallel restarts/islands.
- The same `gecko-core` compiles to **WASM**, so the web UI can score a layout live as
  you drag keys around, with *identical* numbers to the CLI. No duplicated scoring logic.
- TypeScript + Vite + Three.js for the web UI.
- Python stays optional for one-off analysis (plots, notebooks) reading our JSON outputs.

---

## 4. Architecture

```
gecko/
├── crates/
│   ├── gecko-core/      # library: geometry, layout, corpus, metrics, scorer, optimizers
│   ├── gecko-cli/       # `gecko` binary: score / optimize / compare / corpus / serve
│   └── gecko-wasm/      # thin wasm-bindgen wrapper over gecko-core (later)
├── web/                 # TS + Vite + Three.js UI
├── data/
│   ├── geometries/      # physical boards (ANSI, ortho, split, Corne, …) — KLE-compatible
│   ├── corpora/         # raw text + compiled n-gram tables (binary cache)
│   ├── profiles/        # objective weights (TOML), versioned
│   └── layouts/         # known layouts for benchmarking
├── runs/                # one dir per optimization run: config + results + log (JSON)
└── context/             # design docs Claude reads (created after this draft is agreed)
```

### Data flow

```
 text corpus ──► corpus compiler ──► n-gram tables (mono/bi/skip/tri)
                                              │
 geometry ──► precompute key-pair / triple cost tables
                                              │
 profile (weights) ──► fold all metrics into ONE bigram cost matrix
                       + ONE trigram cost table
                                              ▼
                        Scorer (full eval + swap-delta eval)
                                              ▼
                 Optimizer (hill climb / SA / GA / tabu / …)
                                              ▼
                       runs/<id>/  ──►  CLI output  /  web UI
```

### Core modules (gecko-core)

- `geometry` — physical keys and the finger model.
- `layout` — assignment of symbols to keys.
- `corpus` — n-gram extraction, normalization, binary cache.
- `metrics` — each metric is a small, independently testable unit.
- `scorer` — compiles metrics × weights into flat cost tables; full and delta evaluation.
- `optimize` — algorithms behind a shared `Optimizer` trait.
- `run` — config, seeding, persistence, result format.

---

## 5. Layout representation

The key idea is to **separate three things** that most tools blur together:

1. **Geometry** (the physical board) — fixed for a run.
2. **Layout** (which symbol sits on which key) — what the optimizer changes.
3. **Corpus statistics** — fixed for a run.

### Geometry

```toml
# data/geometries/ortho-3x10.toml  (illustrative)
name = "ortho-3x10"
[[key]]
id = 0            # slot index
pos = [0.0, 0.0, 0.0]   # x, y, z in key units (z = 0 for 2D)
rot = [0.0, 0.0, 0.0]   # tilt / tent / splay, degrees (zeros for 2D)
size = [1.0, 1.0]
hand = "L"
finger = "pinky"
row = 0
home = false
effort = 3.0      # base cost of pressing this key
```

- Positions are **always 3D**; a 2D board just has z = 0 and zero rotation. That's how
  "2D now, 3D later" costs nothing today.
- Importer for **KLE JSON** (keyboard-layout-editor.com) so we can pull in any board.
- From a geometry we precompute **pair tables** once: for every (key i, key j) —
  same finger?, same hand?, distance, row jump, lateral stretch, scissor, roll
  direction. Triple tables (or sparse versions) for trigram features.

### Layout

- In memory: `pos: [u8; N]` (symbol → slot) and its inverse `sym: [u8; N]`
  (slot → symbol). Swapping two keys is O(1). Copying a layout is ~32–64 bytes.
- A `locked` bitmask for pinned keys (e.g. keep Z X C V, or keep punctuation).
- Human format: a compact string per row (`"qwfpb jluy;"…`) plus geometry name.
  Stable hash for de-duplicating results.

### Corpus

- Symbols mapped to small integer ids (alphabet ≈ 30–50).
- Dense arrays: monograms `[N]`, bigrams `[N×N]`, skipgrams `[N×N]`, trigrams
  `[N×N×N]` (50³ floats = 500 KB, fine) or top-K sparse.
- Compiled once and cached as a binary blob next to the source text.
- Corpora can be mixed with weights (e.g. 70% English prose, 30% code).

### Going 3D later

"3D" could mean two different things, and they have very different costs:

- **(a) Same optimizer, 3D boards.** Curved/tented/split boards (Dactyl-style) are just
  geometries with z and rotation; the finger model computes distances in 3D. Cheap to add.
- **(b) Optimize the physical shape itself.** Key positions become continuous variables
  (a different, continuous optimization problem — CMA-ES, Bayesian optimization, or
  gradient methods on a differentiable effort model). A separate, later project phase.

---

## 6. Objective function and weights

### Metrics (each returns a raw, interpretable number)

Start with a small, well-understood set, then grow:

| Metric | N-gram order | Meaning |
|---|---|---|
| Key effort | 1 | Σ freq × base effort of the slot |
| Finger load / balance | 1 | deviation from a target load per finger |
| Hand balance | 1 | left vs right |
| SFB | 2 | same-finger bigrams (different keys) |
| SFS | 2 (skip) | same-finger skipgrams |
| Lateral stretch (LSB) | 2 | index/pinky stretches to inner/outer column |
| Scissors / row jumps | 2 | adjacent fingers, big row difference |
| Travel distance | 2 | finger movement distance |
| Alternation | 3 | hand alternation rate |
| Inrolls / outrolls | 3 | rolls on one hand (rewarded) |
| Redirects | 3 | one-hand direction changes (penalized) |
| carpalx effort | 1–3 | carpalx's model, as a "profile" for comparison |

### How the score is built

```
score(layout) = Σ_i  w_i · normalize_i( metric_i(layout) )
```

The important trick: **every bigram-based metric is just a cost matrix over key pairs**,
so the scorer folds them all into one matrix before search starts:

```
C2[slot_a][slot_b] = Σ_i  w_i · cost_i(slot_a, slot_b)
bigram_score       = Σ_{x,y}  freq2[x][y] · C2[pos[x]][pos[y]]
```

Same for monogram (one vector) and trigram (one table). Adding the 20th metric costs
**nothing** in the hot loop. Raw per-metric values are still reported separately, so
we can always explain a score.

### Swap-delta evaluation

Swapping symbols a and b only changes terms that involve a or b. Bigram delta is
O(N) instead of O(N²), trigram delta O(N²) instead of O(N³). This is the single biggest
speed lever, and it gets a dedicated property test (delta == full re-eval).

### Profiles

- Weights live in `data/profiles/*.toml`, versioned and named (`default`, `carpalx`,
  `low-sfb`, `ergo-split`…). Every run records the exact profile it used.
- Normalization per metric (e.g. "% of bigrams") so weights are comparable.
- Later: **multi-objective mode** (Pareto front via NSGA-II) instead of fixed weights,
  so you can see trade-offs (SFB vs rolls vs effort) instead of picking weights blind.

---

## 7. CLI vs web split

Both are front ends over the same `gecko-core`. One binary does everything.

### CLI (Claude's interface)

Designed to be scriptable and machine-readable:

```
gecko corpus build data/corpora/english.txt
gecko score   --layout colemak-dh --geometry ortho-3x10 --profile default [--json]
gecko compare colemak-dh graphite runs/2026-10-20-sa-01 [--json]
gecko optimize --algo sa --profile default --seed 42 --time 60s --restarts 16
gecko analyze runs/<id>          # per-metric breakdown, worst bigrams, finger loads
gecko serve                      # starts the web UI + JSON API
```

- `--json` on everything, stable output schema.
- Deterministic given `--seed`.
- Every `optimize` writes `runs/<id>/` (config, best layouts, convergence log).

### Web (your interface)

- `gecko serve` runs a small Rust HTTP server (axum) that serves the static web app and
  a JSON API over `runs/`, `data/layouts/`, `data/geometries/`.
- Views: run browser, layout detail (metric breakdown, heatmaps), side-by-side compare,
  **interactive editor** (drag to swap, live re-score via WASM), 3D viewer.
- Optionally launch/stop optimization runs from the UI and stream progress.
- Local only (localhost). No auth, no deployment.

---

## 8. 3D visualization

- **Three.js** (plain or via react-three-fiber if we go React).
- Each key rendered from geometry `pos` + `rot` + `size` as a rounded keycap mesh with
  a legend texture. 2D boards render flat; 3D boards (tented/curved) render correctly
  with no special-casing.
- Overlays: heatmap by key frequency, color by finger, SFB arcs between keys, roll arrows.
- Orbit camera, top-down "2D" toggle, PNG export.

---

## 9. Algorithm roadmap

Each algorithm gets its own `context/algorithms/<name>.md` (idea, parameters, known
pitfalls, results on benchmarks).

| Phase | Algorithm | Why |
|---|---|---|
| 0 | Random + score known layouts | Validate the scorer: does it rank Colemak-DH above QWERTY? |
| 1 | Greedy hill climbing (swap neighborhood) + random restarts | Simplest baseline, very fast |
| 2 | **Simulated annealing** | carpalx's approach; strong for QAP; first "real" optimizer |
| 3 | Genetic algorithm (permutation encoding, OX/PMX/cycle crossover, swap mutation, elitism) | You asked for it; good to compare against SA |
| 4 | Tabu search / robust tabu, iterated local search | State of the art for QAP |
| 5 | Memetic (GA + local search), island model across cores | Usually best of both worlds |
| 6 | Re-implement / port scoring models of existing tools (carpalx, Oxeylyzer metrics) as profiles | Cross-check results against known tools |
| 7 | Multi-objective (NSGA-II) | Pareto fronts instead of hand-tuned weights |
| 8 | ML proper: learn metric weights from your typing data; surrogate models; personalized layouts | Where "machine learning" really pays off |

All optimizers share one trait (`init`, `step`, `best`, `stats`) so they're
interchangeable in the CLI, comparable in benchmarks, and composable (e.g. SA as the
local search inside the GA).

---

## 10. Testing and benchmarking

- **Metric unit tests** on tiny hand-computed cases (3–4 keys, known answers).
- **Property test:** for random layouts and random swaps, delta score == full re-score.
- **Regression snapshots:** scores of known layouts under each profile; any change must
  be intentional.
- **Optimizer tests:** on tiny instances small enough to brute-force, the optimizer
  must find the true optimum.
- **Benchmarks** (`criterion`): full evals/sec, swap-deltas/sec, time-to-target-score
  per algorithm. Tracked over time so we notice regressions.
- **Algorithm leaderboard:** same geometry/corpus/profile, fixed time budget, many
  seeds → mean/best score per algorithm.

Performance targets (30-key, bigram+trigram): full eval ≲ 2 µs, swap delta ≲ 200 ns,
≥ 10M swap evaluations/sec across cores.

---

## 11. Proposed `context/` layout (after agreement)

```
context/
├── README.md              # index: what to read for which task
├── goals.md
├── architecture.md
├── representation.md      # geometry, layout, corpus formats
├── objective.md           # scoring, folding, deltas, profiles
├── metrics/               # one file per metric
├── algorithms/            # one file per algorithm
├── cli.md
├── web.md
├── testing.md
└── decisions.md           # running log of decisions + why
```

---

## 12. MVP (first milestone)

1. Rust workspace, `gecko-core` + `gecko-cli`.
2. One geometry (ortho 3×10 or ANSI row-staggered), one English corpus, ~6 metrics
   (effort, finger balance, hand balance, SFB, SFS, LSB).
3. `gecko score` + `gecko compare` against 8–10 known layouts.
4. Hill climbing + simulated annealing with delta evaluation, parallel restarts.
5. Runs saved as JSON.
6. Then: `gecko serve` + a minimal web page showing a layout flat, then in 3D.

---

## 13. Open questions for you

1. **Language:** OK with Rust for the core? (Your familiarity matters more than my preference.)
2. **Which keys to optimize first:** just the 26 letters + a few punctuation (30 keys), or more?
   Layers, thumb keys, and symbols later?
3. **Target board(s):** standard row-staggered ANSI, ortho/columnar, split (Corne, etc.)?
4. **Corpus:** English only? Include code (and which languages)? Your own writing?
5. **"3D later"** — viewing 3D boards (5a), or also optimizing the physical shape (5b)?
6. **Personal data:** would you want to log your own typing (per-bigram timings) to
   learn weights later? That's the strongest "ML" angle.
7. **Metric philosophy:** start close to Oxeylyzer-style metrics, carpalx-style effort, or both?
8. **Web stack:** plain TS + Three.js, or React + react-three-fiber?
