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
`{date, commit, layout, geometry, corpus, profile, full_per_sec, delta_per_sec}` (`commit` gets
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
through both `breakdown` and the folded `Scorer::full`.
delta == full (proptest), folding == breakdown, brute-force optimum on a 6-key board for
HC/SA/GA/memetic, determinism with an eval budget, pins respected, ranking sanity + QWERTY SFB,
corpus counting (incl. skip-2/3, word imports) + binary round-trip + `GKCORP1` compatibility,
imported corpora reproduce QWERTY SFB computed independently from their source tables,
layout TOML round-trip, word lists (fold/merge/TSV round-trip), worst words sum to B01 and Semimak shows `you're` on shai. Reference snapshots (`tests/snapshots.rs`): score + weighted
metrics of the 10 `data/layouts/*.toml` × default/sfb/engram/code on `english`, 4 decimals. KLE (`tests/kle.rs` + unit tests in `kle.rs`): bundled boards round-trip
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
Web: `web/src/format.test.ts`, `web/src/explore.test.ts`.
WASM == CLI: `web/scripts/wasm-parity.mjs` tests all 10 references × all 4 profiles
using the same web-target package loaded by the browser.

Bundled splits (`tests/boards.rs`): every Corne/Sweep/Lily58/Kyria/ErgoDox/Dactyl key
matches pinned QMK-derived KLE inputs; counts, stagger, alpha fingers, fills, reference fits,
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

## CI

`.github/workflows/ci.yml` runs the `make test` checks on every push and PR, in two jobs:
`rust` (`cargo test --release`, `cargo clippy --all-targets -- -D warnings`) and `web`
(`npm ci`, `npm run test:parity`, `npm run typecheck`, `npm test`). Cargo and npm caches are kept
between runs. The WASM package is built before the type check because `web/src/wasm/pkg`
is generated and not committed. The parity command builds it first; failures upload the
generated CLI fixture as a CI artifact for inspection. `make test` also runs parity.


## Snapshots (`insta`)

A changed score or metric fails `cargo test` (snapshots in `crates/gecko-core/tests/snapshots/`).
If the change is intended: `cargo install cargo-insta` once, then `cargo insta test --release`
and `cargo insta review` (accept/reject each diff), and commit the updated `.snap` files with
the change. Without cargo-insta: `INSTA_UPDATE=always cargo test --release --test snapshots`
overwrites them, then review with `git diff`. With `CI` set, insta never writes files, so
unreviewed changes just fail.

## Rule

`cargo test` and `cargo clippy -- -D warnings` pass before any change is considered done.

## Overnight harness

`scripts/overnight.sh` (`make overnight`, `make overnight-dry`) works through open issues in `context/roadmap.md` order, skipping any with open GitHub blockers, an open PR, or an existing `fix/issue-<n>` branch. Each issue gets a headless `claude -p` run in the worktree `../gecko-overnight` (your checkout is never touched). The run resumes its session after usage limits. Then comes the gate: release build, `cargo test`, clippy, wasm, web typecheck and tests, with one repair round. If the gate passes, the harness commits, pushes `fix/issue-<n>` and opens a PR. Failed work is saved as a patch. Logs and `summary.tsv` go in `~/.local/state/gecko-overnight/<stamp>/`; `touch ~/.local/state/gecko-overnight/STOP` stops it after the current issue. Agents see only what is on `origin/main`.
