# Testing and benchmarks

## Must-have tests

| Test | What it guards |
|---|---|
| Metric unit tests | Each metric on a tiny geometry (3–6 keys) with hand-computed answers |
| Delta == full (property test, `proptest`) | Random layouts + random swaps: delta matches re-evaluation within 1e-5 relative (ansi-30; also corne-36/sweep-34 with tokens and empty slots) |
| Folding == unfolded | Folded score equals sum of individually computed metrics |
| Reference snapshot | Scores and metric values of reference layouts per profile (`insta`). Changes must be intentional |
| Ranking sanity | Default profile ranks modern layouts > Colemak > Dvorak > QWERTY |
| Published numbers | SFB % for QWERTY/Colemak/Graphite within tolerance of published values on the matching corpus |
| Optimizer correctness | On instances small enough to brute-force (≤ 8 keys), each optimizer finds the optimum |
| Determinism | Same seed → identical result |
| Format round-trip | Layout/geometry parse → print → parse |
| Collision (geometry phases) | Known overlapping/non-overlapping key pairs |

## Benchmarks (`criterion`)

- Full evaluation per layout, swap-delta per call, swaps/sec on all cores.
- Algorithm leaderboard (`gecko bench`): fixed geometry/corpus/profile and time budget,
  10+ seeds, report mean/best/stdev of final score and time-to-target.
- Keep a `bench/history.jsonl` so performance regressions are visible.

Implemented: `cargo bench -p gecko-core` (`crates/gecko-core/benches/scorer.rs`) runs
`full_eval`, `swap_delta/single` and `swaps_all_cores/<n>_threads` (random swap deltas with
improvements applied, one `State` per thread) on the `gecko.toml` defaults with QWERTY.
`gecko perf --record` appends one JSON line to `bench/history.jsonl`:
`{date, commit, layout, geometry, corpus, profile, full_per_sec, delta_per_sec, build_per_sec, rebuild_per_sec}` (`commit` gets
`-dirty` when the tree has uncommitted changes). Record after any scorer change; compare
entries from the same machine only.

## Web

TypeScript type check + Vitest tests for data shaping. `cd web && npm run test:parity`
builds the browser WASM package and native CLI, generates fresh `gecko compare --json`
expectations, and runs Node tests against the actual WASM binary. Every reference layout
is checked under every bundled profile on ansi-30 / english: total score and each metric's
raw value and weighted contribution must agree within 1e-6 relative (exactly zero when
the CLI value is zero). Rust, Node, and `wasm-pack` are required; no running server is needed.
The fixture is regenerated on every run at `web/.cache/wasm-cli.json` (ignored by Git).
The Rust example `wasm_parity_fixture` exports fully resolved inputs via `Store`, including
comfort tables and profile defaults, then invokes the real CLI to get expected results.

## Implemented (`crates/gecko-core/tests/scorer.rs`, `tests/corpora.rs`, `tests/metrics.rs` + unit tests)

Metric unit tests (`tests/metrics.rs`): every implemented metric on a 6-key board (plus a
2-key crossed-finger board for B14 and a 6-key layered board for Y01/Y02 and thumb loads)
with explicit n-gram counts; each expected value is commented with its arithmetic and checked
through both `breakdown` and the folded `Scorer::full` (E01's model and B10's home-relative row
jump included).
delta == full (proptest), folding == breakdown, brute-force optimum on a 6-key board for
HC/SA/GA/memetic, determinism with an eval budget, pins respected, ranking sanity + QWERTY SFB,
corpus counting (incl. skip-2/3, word imports) + binary round-trip + `GKCORP1` compatibility,
imported corpora reproduce QWERTY SFB computed independently from their source tables,
layout TOML round-trip, word lists (fold/merge/TSV round-trip), worst words sum to B01 and Semimak shows `you're` on shai. Reference snapshots (`tests/snapshots.rs`): score + weighted
metrics of the 10 `data/layouts/*.toml` × default/sfb/engram/code on `english`, 4 decimals. KLE (`tests/kle.rs` + unit tests in `interop/kle.rs`): bundled boards round-trip
export → import (physical keys exact, fingers/rows/columns re-guessed), geometry TOML
round-trip, raw-data parsing, rotation origins, ANSI 60% traditional fingering. Explore (`tests/explore.rs` + unit tests in `explore/`): raw-vector
re-scoring == full score (proptest); on a mirror-symmetric 6-key board the archive equals the
brute-forced set of canonical local optima; ansi atlas consistency (pins, constrained
records, every unconstrained record locally optimal); distances, PCA, funnels, Chao1.
BLS, explore, tabu and memetic are in the brute-force and determinism tests. Memetic also
checks ring snapshot transfer, duplicate rejection, collapse restarts, both HC/tabu refinement,
fixed/pinned layouts, cancellation and identical results across worker counts with fixed
islands; `tests/verify.rs` replays a migrating memetic run. Tabu's incremental Δ
matrix: `delta_shift` == recomputed `delta_linear` for every disjoint pair after a swap
(proptest, including Shift and brackets), and delta == linear + load.
Exact sub-problems (`tests/certify.rs`): `certify` equals exhaustive enumeration for both kinds
(`full`, `pair`) on the 6-key board (all, 4 and 2 free) and on ansi-30 with 5–7 free symbols
from random layouts (where it must also report an improvement); non-free symbols never move;
a re-certified optimum is "already optimal" and no swap of free symbols improves it; results
are deterministic; a node limit gives a best-so-far and a lower bound that bracket the optimum.
`--ignored` adds the bounds.md instance (english, 10 free, 3.6 M placements, threaded brute force).
External validation (`tests/validation.rs`): every non-thumb finger triple in cmini's
`table.json` (pinned in `tests/fixtures/cmini-table.json`) gets the matching trigram class;
ansi-30 distances `rg` 1.6 u, `my` 2.66 u, `vt` 2.02 u match the Layouts Doc.
Run reproducibility (`tests/verify.rs`): an `--evals` run passes `run::verify`; a tampered
`best.json`, a changed input hash or a time-budget run fails. 2D drawing (`tests/flat2d.rs`):
flat boards unchanged, a rigidly tilted board recovered, a curved tented split board with no
overlaps and row/column order kept, layer keys copy their physical key.
Adaptive keys (`magic.rs` unit tests + `scorer.rs`): the count rewrite equals the rewritten
text exactly for mono/bigrams and beats no rewrite for trigrams/skipgrams; delta == full with
a magic key (proptest); `[[magic]]` TOML round-trip; Magic Sturdy's rules halve its SFBs on
`english` and the rule search matches or beats them.
Web: `web/src/format.test.ts`, `web/src/explore.test.ts`, `web/src/export.test.ts` (layout TOML, KLE, stats, standalone SVG), `web/src/runViews.test.ts` (metric columns, qd grid projection: best elite per cell, one-axis rows).
WASM == CLI: `web/scripts/wasm-parity.mjs` tests all 10 references × all 4 profiles
using the same web-target package loaded by the browser.

3D boards (`tests/three_d.rs`): `rot` Euler round trip and axes; the Dactyl's poses equal the
parametric keywell fixture (an independent formulation of the same math); 18.4 mm between
neighbouring keycap tops in every column, tent direction, mirror symmetry; tenting Corne
changes no metric; Dactyl delta == full. Web: tilted keys are pressed along their normal.

Collision (`tests/collision.rs`): touching vs 0.1 u overlap (depth exact), margins, corner
contact, a 45° key whose bounding box overlaps but whose box does not, the grid broad phase
equal to brute force on random boards (sizes, rotations, margins), every bundled flat board
clear, `validate` rejecting an overlap that `check_keys` allows (2D, and a tilted key in 3D),
layer copies skipped, and the 3D grid equal to brute force (depths too) on random tilted, lifted
boards. `collision3d` unit tests: GJK + EPA depth equals the separating axis depth on 4000 random
frustum pairs, moving by the MTV clears every pair; symmetric, coincident, stacked and
face-touching hulls. Repair (`Geometry::repair`): a flat pair moves a quarter key each and
stays flat, a clear board is untouched, margins hold, random overlapping 3D boards end
collision-free with rotations unchanged, layer copies follow their key.

CMA-ES (`tests/cmaes.rs` + `eigh` unit test): sphere 10-D to 1e-10 in < 8k evaluations,
Rosenbrock 10-D to 1e-8 at (1, …, 1), a bounded problem whose optimum lies outside the box ends
on the corner and the objective never sees an out-of-box point, an interior bounded optimum,
same seed → identical history, IPOP solves 5-D Rastrigin where one run does not (λ doubling),
batch evaluator gets whole generations within the budget, an empty generation halts the search,
restarts from the best point.

Geometry search (`tests/geomsearch.rs`): `--free` groups expand to the expected parameters and
bounds contain the start; bad names and bounds fail; every bundled board and the Dactyl well
fixture are clear under the 3D collision test; on random flat boards the 3D test finds the same
pairs as the 2D one; tilted keys meeting at their tops, a convex column's switch bodies and a
stacked key collide while a 15° keywell does not. Bilevel and alternate searches with `--evals`
reproduce exactly, end collision-free, inside the bounds and no worse than the start board; the
run directory's `geometry.toml` loads, equals `geometry.param.toml`, and scores the reported
layout (scored with the run's own symbol order: the trigram coverage cut breaks frequency ties by
symbol number). Progress
events carry the reported board (values, layout) ending at the chosen board; paused time does
not use the time budget; a run's board loads as `run:<id>` (keys and parameters) and is listed
with `kind = "geometry"`.

Holdout (`tests/verify.rs`): a run with `holdout: shai` records it in config.json; every ranked
layout's holdout score equals a fresh shai scorer's; the report's start/best/retained match;
rank ρ in [−1, 1]; the run verifies with a `holdout` hash check; holding out the search corpus
is refused.

Derived costs (`tests/derive.rs`): every bundled board with its `effort`/`stretch` lines removed
derives the same stretch flags (one documented exception, ANSI `N`) and efforts with r > 0.8
against the hand-entered ones, home keys at their home effort, cheapest thumb key 1.0, explicit
efforts kept; parametric efforts follow a moved column; column stagger beats ANSI row stagger
for five non-QWERTY layouts.

Geometry rebuild (`tests/rebuild.rs`): chained random key moves (pose, 3D z/tilt/tent, effort)
on ansi-30, corne-36 with Shift, the 3D Dactyl and a layered corne: after each
`Scorer::set_geometry`, every unfolded table and the full score of random layouts equal a fresh
`Scorer::new` bit for bit (also after `with_weights`); changed-slot counts (1 on a flat board,
the hand on a 3D one); delta == full after `refresh`; a different key set is rejected.
Bench group `tables` (`build`, `rebuild_one_key`); `gecko perf` prints both.

Adaptive fidelity (`tests/fidelity.rs`): ladder sizes; a coarse scorer keeps delta == full;
SA and BLS runs with `fidelity` report exact scores, end on exact local optima and reproduce
for a seed. Fingering (`tests/fingering.rs`): choosing every `ansi-30-alt` option gives
`ansi-30-angle`'s fingers, options survive `with_fingering` and the TOML round trip, bad
`alt`s are rejected and missing efforts derived; a finger reassignment re-costed by
`set_geometry` equals `Scorer::new` (all metrics weighted; this caught the inverted
pose-only check); the fingering search never worsens and is deterministic.

Bundled splits (`tests/boards.rs`): every flat split's key (Corne/Sweep/Lily58/Kyria/ErgoDox)
matches pinned QMK-derived KLE inputs (the 3D Dactyl keeps their slots only); counts, stagger, alpha fingers, fills, reference fits,
TOML round-trips, and all-symbol score deltas (including ErgoDox above 64 slots) are checked.

Integration coverage: corpus extension and adaptive-key rewriting retain Shift counts; tabu
mixed deltas include Shift and bracket placement. Trigram-cache updates and rollback swaps are
covered by the delta/full properties and extended-neighbourhood regressions. Full evaluation
recomputes trigram terms independently of the cache. Reference snapshots include S04, row
effort offsets and Y03.

Parametric geometry (`tests/parametric.rs`): ortho-30 and corrected QMK corne-36 generation
matches every explicit key/metadata field and content hash, and all reference scores/raw
metrics/contributions under every bundled profile exactly. Analytic tests cover home-pivot
splay, split reflection/angle/spread, circular thumb arcs and zone pose; invalid parameters
and duplicate logical slots fail. Store tests cover parameter-only discovery, direct paths,
explicit-file precedence, de-duplication and comfort-table resolution.

Manufacturability (`tests/manufacture.rs` + unit tests in `geometry/manufacture.rs`): each
limit (pitch, key angle, height, plate turned to fit and split into pieces, column counts) on
small boards, zero limits are off; every bundled board passes the defaults; a profile `[build]`
parses and rejects unknown fields; a geometry search under a 5 × 5 u plate finds no feasible
board, and the default limits leave a Corne search unchanged.

Rank-sum selection (`tests/select.rs` + `ranks` unit test): three HC runs (one seed twice)
merge duplicate layouts; per-metric ranks are permutations of 1..n (ties averaged, `max`
metrics reversed), sorted by their sum; re-scored layouts match the runs' own scores.

Profile ranking (`tests/ranking.rs`, `calibrate/rank.rs`): preference orientation, stable
logistic loss and nonnegative weights; duplicate/mirror representatives stay in one group;
changing held-out labels leaves fitted weights, scales and training loss exactly unchanged.

Relative finger effort: scorer regression covers inverse-cost normalization, absent thumbs,
active thumbs, independent hands, scale invariance, directional reach costs, invalid zero
costs, and all pair swaps against full evaluation. Web tests cover generated targets,
mirrored edits, unlocking/relinking hands, and positive typing-cost limits.

Hand model (`tests/hand.rs`): IK recovers the natural curl and reports exact shortfalls; home
keys cheapest per finger on six boards; mirrored boards give mirrored costs; reach ordering by
hand size; a moved key becomes unreachable; ergonomic boards beat flat ones on G01/G05 with
QWERTY; profile `[hand]` round-trips, changes the table hash and scales B14. Thumb: cluster keys
near rest are cheap, a far key is out of reach, tilted keywell thumb keys press better. G03:
ANSI ≈ 12° ulnar with palms flat, splits ≈ 0°, a tent lowers pronation, Dactyl < Corne < ANSI,
wider elbows deviate more, sideways misses add cost, bad `[hand.body]` rejected. G01/G03/G05 are
in `all_weights`, so the delta == full tests cover them; the tent test covers G01/G05's frame
invariance and that tenting lowers G03. Web: `hand.test.ts` covers `withHand` / `handSpec` /
`bodyOf`, the TS thumb (rest, reach, segment lengths) and `toLocal`. The Tabu `delta_shift`
check allows 3e-5 relative: f32 pair tables valued 0–100 can leave a few ULP after cancelling
(seen on Y02 when the per-metric test weights shift).

Weight robustness (`tests/robust.rs`): unit costs times `weight / scale` reproduce the scorer's
score (default, hand, familiar; three layouts); QWERTY and its mirror are 0 keys apart
(mirror-invariant) and one swap 2; families join a swap and a mirror and keep a scramble apart;
rank confidence with σ = 0 keeps every rank, QWERTY stays last at σ = 0.5; a small sweep runs
1 + 3·weights + draws searches with base regret 0 and E01 judges on every optimum. `select`
re-scores runs with a fresh scorer, so it allows 1e-4 relative (trigram tie-break by symbol
order, see above).

## Cross-language checks

The web reimplements or consumes some Rust code; these keep the two sides equal.

- **WASM == CLI** (`npm run test:parity`): see Web above.
- **Hand model** (`web/src/hand.parity.test.ts`): `gecko_core::hand` writes
  `web/src/fixtures/hand-parity.json` (skeleton, natural curl, finger IK, placement, thumb IK for
  three hand sizes); `cargo test` fails if the committed fixture is stale and `npm test` fails if
  `hand.ts` disagrees (1e-6 relative). After an intended hand-model change:
  `GECKO_BLESS=1 cargo test --release -p gecko-core hand_parity`, then port it to `hand.ts`.
- **API contract** (`npm run test:contract`): `serve::tests::api_samples` calls every JSON
  endpoint on a temporary data dir and writes the responses to
  `web/contract/api-samples.generated.ts` (gitignored), each checked with
  `satisfies Response<"name">` against the return types of `web/src/api.ts`. A field renamed or
  retyped in Rust fails the type check. Optional fields the server omits are not caught.
  It includes a short board + layout run (`geom`), its detail and free parameters, and checks the
  run's board is listed as `run:<id>` and scores.

## CI

`.github/workflows/ci.yml` runs the `make test` checks on every push and PR, in two jobs:
`rust` (`cargo test --release`, `cargo clippy --all-targets -- -D warnings`,
`cargo doc --no-deps -p gecko-core -p gecko-wasm` with warnings denied) and `web` (`npm ci`,
`npm run test:parity`, `npm run typecheck`, `npm run test:contract`, `npm test`). Cargo and npm caches are kept
between runs. The WASM package is built before the type check because `web/src/wasm/pkg`
is generated and not committed. The parity command builds it first; failures upload the
generated CLI fixture as a CI artifact for inspection. `make test` also runs parity, the contract check and `research.py lint`.

`.github/workflows/claude.yml` runs `anthropics/claude-code-action` when the repo owner writes
`@claude` in an issue, issue/PR comment or review. It needs the secret `ANTHROPIC_API_KEY` (or
`CLAUDE_CODE_OAUTH_TOKEN`) and the Claude GitHub app; the agent follows `AGENTS.md`.

## Snapshots (`insta`)

A changed score or metric fails `cargo test` (snapshots in `crates/gecko-core/tests/snapshots/`).
If the change is intended: `cargo install cargo-insta` once, then `cargo insta test --release`
and `cargo insta review` (accept/reject each diff), and commit the updated `.snap` files with
the change. Without cargo-insta: `INSTA_UPDATE=always cargo test --release --test snapshots`
overwrites them, then review with `git diff`. With `CI` set, insta never writes files, so
unreviewed changes just fail.

## Rule

`cargo test` and `cargo clippy -- -D warnings` pass before any change is considered done.
