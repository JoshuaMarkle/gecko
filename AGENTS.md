# GECKO — agent instructions

GECKO searches for the best possible keyboard layouts, and later generates physical
keyboards (2D → 3D). It is a data-analysis system for judging layouts, with fast
optimization on top. Rust core + CLI + server + WASM; TypeScript web UI with a 3D viewer.

## How to work

- **Work autonomously.** Development runs with little supervision. Don't ask for
  permission on implementation details. Make sound decisions, keep moving, and record
  any non-obvious choice in `context/decisions/<area>.md` (date — decision — why; index
  `context/decisions.md`).
- Ask the user only when a choice changes the project's goals or cannot be undone.
- Prefer small, verifiable steps. A change is done when it builds, tests pass, and the
  relevant `context/` file is still accurate.
- Keep docs concise. Update docs in the same change as the code.

## Git and builds

- **Work on `main`.** Never create branches or worktrees unless the user gives explicit
  typed permission. Other agents edit the same checkout at the same time: work around
  each other. Leave files and changes you did not make alone. Commit only your own paths
  (`git commit --only <paths>`).
- **Check upstream at the start of a session:** `git fetch`, `git log HEAD..origin/main`,
  and the issue's state (`gh issue view <n>`). Another agent may already have done it.
  Pull (`git pull --rebase`) before committing.
- **Build in dev mode:** `make` / `make serve` (no wasm-opt, no tsc). Use `make release` only
  when a long data crunch needs the optimized build.

## Where to look (read only what the task needs)

| Task | Read |
|---|---|
| Orientation | `context/goals.md`, `context/architecture.md` |
| Data formats (geometry, layout, corpus, runs) | `context/representation.md` |
| Metrics, scoring, profiles | `context/objective.md`, `context/metrics-catalog.md`, `context/parameters.md` |
| Languages to optimize on (recipes, components, building corpora) | `context/representation.md#corpus`, `data/corpora/index.toml` |
| Vim users: h j k l policies, the `vim` corpus | `context/vim.md` |
| Which symbols a search places (alpha / Shift pairs / symbol layer), Space, Enter, Backspace, Shift, Ctrl placement | `context/keysets.md` |
| Does the search beat known layouts (Phase 2 evaluation) | `context/evaluation.md` |
| Hand model, reach / press-angle / wrist-posture costs | `context/hand-model.md` |
| Parametric and 3D boards, collision, derived effort | `context/geometry.md` |
| Hand → optimized board → STL walkthrough | `context/build-guide.md` |
| An optimizer | `context/algorithms/README.md` + `context/algorithms/<name>.md` |
| CLI | `context/cli.md` |
| Web UI / 3D | `context/web.md`, `context/design.md` (visual design) |
| Tests, benchmarks | `context/testing.md` |
| Code style, crates, tooling | `context/conventions.md` |
| Past decisions | `context/decisions.md` |
| What to work on next (open GitHub issues) | `context/roadmap.md` |
| Why those items (research synthesis) | `research/SYNTHESIS.md` |
| Background evidence (per-source summaries, studies, experiments) | `research/README.md`, `research/sources/` |
| Unverified claims, uncovered research topics | `research/OPEN-QUESTIONS.md` |

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
cd web && npm run test:contract           # web/src/api.ts types vs real server JSON
python3 -I research/scripts/research.py lint   # research/ links, frontmatter, index (also in make test)
```

Or `make` / `make serve` (fast dev build: wasm without wasm-opt, no tsc), `make release` / `make serve-release`
(full: wasm-opt + typecheck), `make test`.

## Non-negotiables

- Hot loop: delta evaluation only, no allocation, no hash maps, no dynamic dispatch.
- The delta == full-evaluation property test must always pass.
- Runs are reproducible: seed, inputs, and hashes recorded in every run directory.
- CLI output supports `--json` with a stable schema.
- When using the CLI to explore, save interesting results as runs and summarize findings
  in the relevant `context/algorithms/*.md` Results section.
