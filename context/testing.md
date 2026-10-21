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

## Web

TypeScript type check + a few Vitest tests for data shaping. A smoke test that the
WASM scorer matches the CLI on reference layouts.

## Implemented (`crates/gecko-core/tests/scorer.rs`, `tests/corpora.rs` + unit tests)

delta == full (proptest), folding == breakdown, brute-force optimum on a 6-key board for
HC/SA/GA, determinism with an eval budget, pins respected, ranking sanity + QWERTY SFB,
corpus counting (incl. skip-2/3, word imports) + binary round-trip + `GKCORP1` compatibility,
imported corpora reproduce QWERTY SFB computed independently from their source tables,
layout TOML round-trip, word lists (fold/merge/TSV round-trip), worst words sum to B01 on a
word corpus and Semimak shows `you're` on shai. Explore (`tests/explore.rs` + unit tests in
`explore/`): raw-vector re-scoring == full score (proptest); on a mirror-symmetric 6-key board the archive equals the
brute-forced set of canonical local optima; ansi atlas consistency (pins, constrained
records, every unconstrained record locally optimal); distances, PCA, funnels, Chao1.
BLS and explore are in the brute-force and determinism tests.
Web: `web/src/format.test.ts`, `web/src/explore.test.ts`.
WASM == CLI is checked by hand for now (same `gecko-core` code; scores match to 3 decimals).

Still to add: metric unit tests with hand-computed answers per metric, `insta` snapshots,
criterion benches + `bench/history.jsonl`.

## Rule

`cargo test` and `cargo clippy -- -D warnings` pass before any change is considered done.

## Overnight harness

`scripts/overnight.sh` (`make overnight`, `make overnight-dry`) works through open issues in `context/roadmap.md` order, skipping any with open GitHub blockers, an open PR, or an existing `fix/issue-<n>` branch. Each issue gets a headless `claude -p` run in the worktree `../gecko-overnight` (your checkout is never touched). The run resumes its session after usage limits. Then comes the gate: release build, `cargo test`, clippy, wasm, web typecheck and tests, with one repair round. If the gate passes, the harness commits, pushes `fix/issue-<n>` and opens a PR. Failed work is saved as a patch. Logs and `summary.tsv` go in `~/.local/state/gecko-overnight/<stamp>/`; `touch ~/.local/state/gecko-overnight/STOP` stops it after the current issue. Agents see only what is on `origin/main`.
