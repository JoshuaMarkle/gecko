# GECKO — agent instructions

GECKO searches for the best possible keyboard layouts, and later generates physical
keyboards (2D → 3D). It is a data-analysis system for judging layouts, with fast
optimization on top. Rust core + CLI + server + WASM; TypeScript web UI with a 3D viewer.

## How to work

- **Work autonomously.** Development runs with little supervision. Don't ask for
  permission on implementation details. Make sound decisions, keep moving, and record
  any non-obvious choice in `context/decisions.md` (date — decision — why).
- Ask the user only when a choice changes the project's goals or cannot be undone.
- Prefer small, verifiable steps. A change is done when it builds, tests pass, and the
  relevant `context/` file is still accurate.
- Keep docs concise. Update docs in the same change as the code.

## Where to look (read only what the task needs)

| Task | Read |
|---|---|
| Orientation | `context/goals.md`, `context/architecture.md` |
| Data formats (geometry, layout, corpus, runs) | `context/representation.md` |
| Metrics, scoring, profiles | `context/objective.md`, `research/metrics-catalog.md` |
| Hand model, reach / press-angle / wrist-posture costs | `context/hand-model.md` |
| An optimizer | `context/algorithms/README.md` + `context/algorithms/<name>.md` |
| CLI | `context/cli.md` |
| Web UI / 3D | `context/web.md`, `context/design.md` (visual design), `context/frontend.md` (planned redesign) |
| Tests, benchmarks | `context/testing.md` |
| Code style, crates, tooling | `context/conventions.md` |
| Past decisions | `context/decisions.md` |
| What to work on next (timeline, GitHub issues #1–103) | `context/roadmap.md` |
| Why those items (research synthesis) | `research/SYNTHESIS.md` |
| Background (other tools, parameters, hand model, data, per-source summaries) | `research/README.md`, `research/sources/` |

## Commands

```
cargo build --release                       # ./target/release/gecko
cargo test --release                        # core invariants (delta == full, optimizers, ranking)
cargo clippy --all-targets -- -D warnings
./target/release/gecko perf                 # scorer speed (full / delta per second)
./target/release/gecko compare              # rank the reference layouts
./target/release/gecko optimize --time 30s  # writes runs/<id>/
./target/release/gecko bench --time 5s      # algorithm leaderboard
cd web && npm install && npm run wasm && npm run build   # web UI (needs wasm-pack)
./target/release/gecko serve                # http://127.0.0.1:7777
cd web && npm run dev                       # UI dev server, proxies /api to gecko serve
cd web && npm run typecheck && npm test
```

Or `make` (build everything), `make test`, `make serve`.

## Non-negotiables

- Hot loop: delta evaluation only, no allocation, no hash maps, no dynamic dispatch.
- The delta == full-evaluation property test must always pass.
- Runs are reproducible: seed, inputs, and hashes recorded in every run directory.
- CLI output supports `--json` with a stable schema.
- When using the CLI to explore, save interesting results as runs and summarize findings
  in the relevant `context/algorithms/*.md` Results section.
