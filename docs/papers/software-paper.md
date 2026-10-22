# GECKO: A Fast, Explainable System for Analyzing and Optimizing Keyboard Layouts

*Superseded by [software/software.tex](software/software.tex) (the typeset version, 8 pages); this file keeps the figure plan and evidence map; the prose below is the older outline and lacks the use-case section.
Software paper, working draft. Describes the system as built on 2026-10-21 (main branch).
`TODO(run)` marks numbers that still need a run; every other number is copied from the repo
(source in brackets). Target: JOSS / SoftwareX / arXiv cs.SE + cs.HC.*

**Authors:** Joshua Markle, and contributors. **Code:** `github.com/JoshuaMarkle/gecko`.

---

## Abstract

Keyboard layout design is a combinatorial optimization problem (30! ≈ 2.65 × 10³² placements of
30 letters) wrapped around a judgement problem (what makes a layout comfortable?). Existing
analyzers and optimizers each fix one answer to the judgement problem, hide the per-metric
reasons behind a single score, and cannot say how close a layout is to the best possible. GECKO
is an open-source Rust system (core library, CLI, server, WebAssembly build) and a TypeScript web
UI that separates the two problems. A *scorer* turns a geometry, a corpus and a weighted profile
of ≈ 45 metrics into pre-folded cost tables, so that evaluating a key swap touches only the
n-grams that contain the swapped symbols (≈ 6.6 × 10⁵ swap evaluations per second per core on a
laptop, 30 keys, trigram costs). Eight optimizers share one `Optimizer` trait and one threaded
runner. Every run records seeds, input hashes and parameters and can be re-verified. Beyond
search, GECKO provides lower bounds with optimality gaps, local-optima atlases and
quality-diversity maps, and a hand model that makes board geometry itself a search variable.
We describe the design, the invariants that keep it correct (delta == full evaluation, native ==
WebAssembly), and measure speed, search quality and bound strength.

**Keywords:** keyboard layouts, quadratic assignment, metaheuristics, reproducible research,
ergonomics, WebAssembly.

---

## 1. Introduction

Alternative layouts (Dvorak, Colemak, Workman, Graphite, Sturdy, Semimak, Canary, and thousands
more in community collections) are designed with a family of analyzers and optimizers:
carpalx, KLO, Kühlmak, genkey, Oxeylyzer, cmini's analyzer, and others
(`research/studies/tool-comparison.md`). They agree on the core ideas — penalize same-finger
bigrams (SFB) and skipgrams, lateral stretches, scissors, bad trigrams; balance load — and
disagree on weights, definitions and the search used. Three problems recur:

1. **The score is opaque.** One number hides which trade-offs it made, and two tools rank the
   same layouts differently with no way to tell why.
2. **The optimizer is welded to the objective.** Changing a metric means editing the search.
3. **Nobody says how good "good" is.** A heuristic returns a layout; there is no certificate and
   no map of what else was nearly as good.

GECKO's contributions:

- **A scorer built for swaps.** All metrics fold into at most four tables (mono, bigram,
  skipgram, trigram); adding a metric adds no cost to search (§3).
- **A single trait for optimizers**, so SA, hill climbing, tabu, breakout local search, genetic,
  memetic, MAP-Elites and NSGA-II are compared on equal terms with `gecko bench` (§4).
- **Explanations everywhere.** Every score decomposes per metric, per finger, per n-gram, per
  word (§5).
- **Certificates and maps**: Gilmore–Lawler lower bounds with dual ascent, local-optima networks,
  quality-diversity grids (§6).
- **Geometry in the loop**: a parametric board model and a hand model whose reach, press-angle and
  wrist-posture costs are precomputed into the same tables, so a board can be optimized with the
  same scorer (§7).
- **Reproducibility by construction**: seeded RNG identical on native and WASM, content hashes,
  `gecko verify` (§8).

## 2. Problem formulation

A **geometry** is a list of *N* slots (key position, rotation, size, finger, row, column, home
flag, effort). A **layout** assigns a symbol to each slot (a permutation of *S* symbols; empty
slots, thumb tokens and layer keys are allowed). A **corpus** gives frequencies *f₁*, *f₂*
(bigrams), *f₂ᵏ* (skipgrams, gaps 1–3) and *f₃* (trigrams). A **profile** holds metric weights
and parameters.

With layout *p* (symbol → slot), the objective is

```
score(p) = Σₓ f₁[x]·C₁[p(x)]
         + Σₓᵧ f₂[x,y]·C₂[p(x),p(y)]  + skipgram terms  + Σₓᵧ𝓏 f₃[x,y,z]·C₃[p(x),p(y),p(z)]
         + small non-linear terms (finger balance, peak load, hand balance).
```

The first three lines are a quadratic assignment problem (QAP) with a sparse cubic part; the
last line are functions of a few per-finger accumulators. A swap of symbols *a, b* changes only
terms containing *a* or *b*.

## 3. The scorer

### 3.1 Metrics as cost tables

A metric (`gecko-core/src/metrics/`) is a *table builder*: given a geometry it returns a mono
`[N]`, pair `[N×N]` or triple `[N×N×N]` cost, plus a rule to report a raw value (%, per-1000).
About 45 metrics are implemented across families: same-finger (SFB, SFS, far SFS, distance),
stretch and scissors (B14 geometric stretch, B11 finger-pair movement), rolls and alternation
(a dozen trigram classes including thumbs), load and balance (M-series), comfort judges (Engram
E04/E05), hand-model costs (G01 reach, G03 wrist, G05 press angle), and symbol/layer metrics
(Y-series: layer switches, code bigrams, paired brackets, home-row-mod misfires).
(`gecko list metrics`; `context/metrics-catalog.md`.)

### 3.2 Folding

For a profile with weights *wₘ*, the linear metrics are summed once at build time:

```
C₂[s][t] = Σₘ wₘ · c₂ₘ[s][t]        (same for C₁, C₂ᵏ, C₃)
```

so search cost does not depend on the number of metrics. A property test checks *folded ==
unfolded*; another checks *delta == full* (below). Raw (unweighted) values are recomputed
separately for reports.

### 3.3 Swap-delta evaluation

For a swap of symbols *a, b*, the change is computed from the O(S) bigram partners of each and
the sparse list of non-zero trigrams that contain them. Implementation notes
(`context/objective.md`):

- Trigrams are stored per symbol in a sparse list; a `State` caches the folded cost of every
  non-zero trigram, so deltas read the old cost and `apply` refreshes only touched entries.
- The hot loop performs no allocation, no hash lookups and no dynamic dispatch.
- The cache stores exact unweighted f32 costs so that caching does not change the arithmetic.
- An index-based CSR variant was tried and rejected (≈ 303–310 k/s vs 370 k/s).
- Non-linear terms update from per-finger accumulators in O(1).

**Measured** (ansi-30, english, default profile, QWERTY start, one thread, i9-13900H, with other
processes running; `gecko perf`, 2026-10-21):

| operation | rate | per op |
|---|---|---|
| full evaluation | 155,766 /s | 6.42 µs |
| swap delta | 657,712 /s | 1.52 µs |
| scorer build (fold all tables) | 324 /s | 3.08 ms |
| move one key and rebuild (geometry search) | 1,550 /s | 0.65 ms |

The delta is ≈ 4.2× faster than a full evaluation; across 20 threads the runner reaches several
million evaluations per second (`bench/memetic-2026-10-21`: BLS 1.1 M/s, tabu 4.1 M/s on 4 threads
in the leaderboard, with background builds). `TODO(run)`: a clean 1/4/8/20-thread scaling plot
on an idle machine, (`bench/history.jsonl` is not a clean speed trend: it mixes contended runs and profiles, so it is not plotted).

### 3.4 Invariants (tests)

| Invariant | Test |
|---|---|
| delta(a,b) == full(after) − full(before), relative 1e-5, incl. 36-key boards with tokens and empty slots | `proptest` in `tests/scorer.rs` |
| Folded score == sum of individually computed metrics | `tests/scorer.rs` |
| Hand-computed metric values on 3–6 key boards | `tests/metrics.rs` |
| Rankings: modern layouts > Colemak > Dvorak > QWERTY; published SFB values within tolerance | `tests/ranking.rs` |
| Brute-force optimum on ≤ 8-key instances for every optimizer | `tests/` |
| Rebuilding a moved board == building it from scratch, bit for bit | `tests/rebuild.rs` |
| WASM scores == CLI scores for every reference layout × profile (1e-6) | `web npm run test:parity` |
| Same seed → same result | determinism tests |

## 4. Optimizers

All discrete optimizers implement

```rust
trait Optimizer {
    fn init(&mut self, scorer: &Scorer, start: Option<&Layout>, rng: &mut Rng);
    fn step(&mut self, scorer: &Scorer, rng: &mut Rng) -> StepInfo;   // a batch of work
    fn best(&self) -> &[Scored<Layout>];                              // de-duplicated top-K
}
```

The runner owns time budgets, threads, progress events (over `mpsc`) and persistence;
algorithms contain only search logic. Workers run on `std::thread::scope` threads (no pool
library); the scorer is shared immutably, and each worker has its own state and RNG seeded from
(run seed, worker index). Cancel/pause use atomics.

| Algorithm | Role |
|---|---|
| Hill climbing (steepest descent) | baseline; inner loop of the others |
| Simulated annealing | main workhorse; exponential schedule with T₀ set from sampled swap deltas |
| Breakout local search | descent + adaptive random kicks; also samples the local-optima network |
| Tabu (RoTS) | robust tabu search for QAP; cross-check |
| Genetic / memetic | crossover + local search on islands with synchronized migration |
| MAP-Elites | quality-diversity grid over behaviour axes |
| NSGA-II | multi-objective Pareto fronts |
| CMA-ES (IPOP) | continuous geometry parameters |

Neighbourhoods beyond swaps (same-finger column exchange, 3-cycles, depth-2 swaps) are available
via `--moves`; all are swap sequences, so they reuse the delta machinery and respect pins.

### 4.1 Results: algorithm leaderboard

`gecko bench --algos sa,bls,tabu,memetic --time 2s --seeds 10 --threads 4`, ansi-30 / english /
default (`bench/memetic-2026-10-21/leaderboard.json`; lower is better; background builds ran):

| Algorithm | best | mean ± sd (10 seeds) |
|---|---|---|
| BLS | 17.561 | 17.616 ± 0.081 |
| SA | 17.565 | 17.787 ± 0.145 |
| Memetic | 17.650 | 18.027 ± 0.352 |
| Tabu | 18.281 | 18.960 ± 0.256 |

A 20 s SA under the current default profile reaches 15.704 from different seeds
(`context/algorithms/simulated-annealing.md`). Larger neighbourhoods give better optima per
descent but cost 3–60× more evaluations, so swap-only stays the default
(`context/algorithms/README.md`). `TODO(run)`: budget grid with 30 seeds, performance profiles and
time-to-target ECDFs (see [topics.md T10](topics.md)).

## 5. Explaining a score

`gecko score|analyze` and the web UI report, for any layout: the weighted contribution and raw
value of every metric, finger and hand loads, a per-key heat map, the worst bigrams / skipgrams /
trigrams, and with a word corpus the worst real words per metric. Reference layouts are scored
in the same pipeline and the results are checked against published numbers (QWERTY SFB 6.63 %;
`gecko compare`). The web UI is the same core compiled to WebAssembly, so a number shown while
dragging a key equals the CLI's. If WASM fails, it falls back to `POST /api/score`.

Current ranking of reference layouts under the default profile (`gecko compare`, ansi-30,
english; lower is better):

| # | Layout | Score | SFB % | Scissors % | Rolls-in % |
|---|---|---|---|---|---|
| 1 | Graphite | 21.68 | 1.16 | 0.71 | 20.1 |
| 2 | Sturdy | 21.83 | 0.89 | 1.16 | 21.7 |
| 3 | Semimak | 24.05 | 0.96 | 0.99 | 19.7 |
| 4 | Canary | 25.85 | 0.97 | 2.02 | 22.4 |
| 5 | Dvorak | 31.43 | 2.58 | 0.12 | 26.4 |
| 6 | Colemak-DH | 31.57 | 1.54 | 0.37 | 24.3 |
| 7 | Halmak | 33.16 | 2.75 | 1.20 | 20.3 |
| 8 | Colemak | 33.67 | 1.54 | 0.48 | 24.3 |
| 9 | Workman | 37.66 | 3.00 | 0.88 | 23.0 |
| 10 | QWERTY | 69.58 | 6.63 | 5.84 | 20.6 |

The profile is expressed in "spread units" (each weight divided by the metric's standard
deviation over reference layouts), so a weight reads as "how many reference-spreads of this
metric I trade for one of another".

## 6. Beyond a single answer

### 6.1 Lower bounds and optimality gaps

The pair part of the score is a QAP; trigrams are relaxed into pair terms (each trigram
contributes the minimum over the free third key). The bound is Gilmore–Lawler (a linear
assignment per placement, solved with the Hungarian algorithm), strengthened by 20 iterations of
dual ascent. Every `gecko optimize` run reports `gap = (value − lower) / |value|`.

| free symbols | total gap | pair-part gap |
|---|---|---|
| 30 | 90.3 % | 70.3 % |
| 16 | 24.5 % | 12.1 % |
| 10 | 12.9 % | 1.7 % |

(run `20261021-171218-sa-a3306812`, `context/algorithms/bounds.md`.) The bound certifies little at
full size but is tight for sub-problems, which motivates window-wise exact search (#43). We
report this limitation openly; tighter bounds (RLT-2, SDP) are future work.

### 6.2 Local-optima atlases

`gecko explore` samples the score distribution and random-walk autocorrelation, then runs BLS
with an archive observer that stores every local optimum with its full raw metric vector and every
kick edge (a local-optima network). *Exclusion restarts* start new descents far (Hamming ≥ 8)
from the known best, to find layouts that differ from the winners. Because the archive stores raw
vectors, it can be **re-ranked under new weights in milliseconds**, which makes weight sensitivity
cheap to study (topics T1, T3). `gecko atlas` reports ranking, families, consensus slots,
funnels, fitness-distance correlation.

### 6.3 Quality diversity

`gecko qd` runs MAP-Elites over behaviour axes (roll ratio, index load, pinky load, hand balance,
or any metric). Each cell holds the best layout with that behaviour, so a user can choose "more
rolls" or "less index load" and see its price. NSGA-II provides Pareto fronts over chosen metrics.

## 7. Geometry in the loop

Geometry is a first-class input. Boards are TOML (explicit or parametric: zones, columns, rows,
stagger, splay, split, thumb arcs), importable from keyboard-layout-editor.com, with eight
bundled boards (ANSI-30, ortho-30, Corne, Sweep, Lily58, Kyria, ErgoDox, Dactyl-Manuform 5×6).
Costs that depend on geometry come from two places:

- **Derived effort and stretch** from key positions (B14 geometric stretch adapted from
  Oxeylyzer 2, G02 travel).
- **A hand model** (`gecko-core/src/hand`): a skeleton placed on the home keys, planar finger IK
  with joint limits, a 4-DoF thumb, arm and elbow geometry; it produces per-key costs for reach
  (G01), press-angle alignment (G05) and wrist posture (G03). These are mono tables, so they fold
  into the scorer with no search-time cost.

`Scorer::set_geometry` re-costs only the tables whose inputs moved, bit-identical to a full
rebuild: moving one key on ANSI-30 costs 0.64 ms vs 3.1 ms for a full build (4.8×), 0.78 ms vs
4.6 ms on Corne, 2.2 ms vs 19.5 ms on the 3D Dactyl-Manuform (9×). This is what makes bilevel
search (outer CMA-ES over board parameters, inner layout search) feasible. A 2D collision test
(oriented-box SAT) keeps keys from overlapping. 3D bilevel search and collision (H10, H8) are in
progress; see §10.

## 8. Reproducibility and engineering

- **Seeded RNG**: xoshiro256++ implemented in-tree, identical on native and WASM.
- **Run directories** (`runs/<time>-<algo>-<hash>/`): `config.json` (params, input names and
  content hashes, full profile, start layout, pins, version) and `best.json` (top-K with full
  breakdown, evaluations, speed, optimality gap). `gecko verify run:<id>` re-runs an
  evaluation-budget run and checks hashes, evaluation count and best score.
- **Content hashing**: FNV-1a of geometry, corpus, profile tables and layouts; used to
  de-duplicate results and detect stale atlases.
- **Stable JSON**: every CLI command supports `--json` with a versioned schema; a contract test
  compares the web client's TypeScript types against real server output.
- **CI**: cargo test + clippy `-D warnings` + rustdoc; web typecheck, Vitest, WASM parity,
  contract test. Benchmarks append to `bench/history.jsonl` with the commit hash.
- **Size**: `TODO(run)` generate lines-of-code, test counts and coverage with `tokei`/`cargo
  llvm-cov` for the camera-ready.

## 9. Related work

Layout analyzers and optimizers: carpalx (SA over effort model), KLO, Kühlmak, genkey,
Oxeylyzer, cmini's analyzer, Keyboard Layout Editor, Ergogen, Cosmos (physical-board generation).
Combinatorial optimization: QAP heuristics (robust tabu, breakout local search), Gilmore–Lawler
bounds, quality diversity (MAP-Elites). Ergonomics: finger independence (Häger-Ross & Schieber),
Fitts-type movement models, large-scale typing timing (Dhakal et al., 136M keystrokes), Engram
comfort study. Per-tool comparison: `research/studies/tool-comparison.md`. Literature coverage
2023–2026 is thin (`research/studies/literature-survey-2023-2026.md`); the camera-ready must cite
what exists and state what no one has measured.

## 10. Limitations and future work

- Costs are **models**, not measurements. We have no user study yet; scores are predicted
  comfort, not observed comfort or speed (see [topics.md](topics.md), "Gaps").
- Bounds are weak on the full problem (90 % gap with 30 free symbols).
- The learned-profile experiment gained only 1.8 pairwise-accuracy points (53.8 % vs 52.0 %) and
  likes are confounded by popularity (`research/experiments/2026-10-21-profile-ranking.md`).
- Single-language corpora dominate; multilingual and code corpora exist but are less validated.
- Geometry search beyond flat boards (3D bilevel, free-form keys, STL export) is roadmap, not
  shipped (`context/roadmap.md`, tracks G–J).
- Timing and personal calibration (E01, E02) are planned (track K).

## 11. Availability

MIT (no LICENSE file yet: TODO). `cargo build --release`; `gecko serve` for the web UI; WASM build via
`npm run wasm`. Data (corpora, boards, profiles, reference layouts) ships in `data/`.
Quick start:

```
gecko compare                       # rank the reference layouts
gecko optimize --time 30s           # writes runs/<id>/ with a bound and a breakdown
gecko explore --time 5m && gecko atlas <run>
gecko qd --axes roll_ratio,index_load
gecko verify run:<id>
```

---

## Figure plan

Typeset figures are `software/assets/*.pdf`, made by `software/figures.py` from repo data.

| # | Figure (name in `software.tex`) | Source | Status |
|---|---|---|---|
| 1 | Architecture and data flow (`architecture`) | `context/architecture.md` | done |
| 2 | Swap-delta rate vs metrics enabled (`metricsweep`) | `bench/software-paper-2026-10-21/metrics_scaling.py` | done |
| 3 | Search throughput vs threads (`threads`) | `bench/software-paper-2026-10-21/thread_scaling.sh` | done (8-thread points noisy; rerun idle) |
| 4 | Algorithm leaderboard, 10 seeds (`leaderboard`) | `bench/memetic-2026-10-21` | done; budget grid with 30 seeds `TODO(run)` |
| 5 | Optimality gap vs free symbols (`bounds`) | `context/algorithms/bounds.md` | done |
| 6 | One-key rebuild vs full build (`rebuild`) | `context/objective.md` | done |
| 7 | Reference layouts, score vs SFB (`references`) | `gecko compare` | done |
| 8 | Optimized vs Graphite, per metric (`tradeoff`) | `bench/software-paper-2026-10-21/runs/*sa-8b3885fd` | done |
| 9 | MAP-Elites map (`qdmap`) | `bench/software-paper-2026-10-21/runs/*qd-af3fe56a` | done |
| - | Local-optima network, fitness-distance plot | `gecko explore` + `atlas` | `TODO(run)` |
| - | Screenshot of the web UI with 3D board | web UI | capture |

Name figures so they do not collide with `templates/assets/` (the build finds that directory first).

## Section-to-evidence map (for reviewers)

| Claim | Evidence in repo |
|---|---|
| Delta == full | `crates/gecko-core/tests/scorer.rs` (proptest) |
| Cost depends on channels, not metric count | `bench/software-paper-2026-10-21/metrics_scaling.json`; `context/objective.md` §Folding |
| Thread scaling 8.7× at 20 threads | `bench/software-paper-2026-10-21/thread_scaling.json` |
| Native == WASM | `web/` parity test |
| Runs reproducible | `gecko verify`, `tests/verify.rs` |
| Bound results | `context/algorithms/bounds.md` §Results |
| Algorithm ranking | `bench/memetic-2026-10-21/` |
| Rebuild speed | `context/objective.md` §Geometry rebuild |
| Design use case (17.561 vs Graphite 21.680; holdout keeps 99.6 %, ρ 0.16) | `bench/software-paper-2026-10-21/runs/*sa-8b3885fd` |
| QD price of rolls (0.4 / 2.6 / 5.5 points) | `bench/software-paper-2026-10-21/runs/*qd-af3fe56a/qd.json` |
| Random baseline, correlation length | `bench/software-paper-2026-10-21/landscape.json` |
| Magic Sturdy 19.995 vs Sturdy 21.831 | `bench/software-paper-2026-10-21/magic-sturdy-compare.json` |
| Code size, test counts (22.7k Rust lines, 197 + 69 tests) | `find crates -name '*.rs'`, `cargo test --release`, `npx vitest run`, 2026-10-21 |
