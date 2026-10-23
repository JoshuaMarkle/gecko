# Contributing to GECKO

Welcome to GECKO! This guide is for human contributors working on the codebase.
(Autonomous AI agents should instead follow [AGENTS.md](AGENTS.md).)

## Prerequisites

- **Rust**: Stable toolchain (`cargo`, `rustc`, `clippy`).
- **Node.js & npm**: Node 18+ for the web UI.
- **wasm-pack**: Required for browser WASM builds (`cargo install wasm-pack`).
- **Python 3**: Standard library only (used for `research/scripts/research.py lint`).

## Project Structure

```
├── crates/
│   ├── gecko-core/   # Core library: geometry, corpora, metrics, scorer, optimizers, hand model
│   ├── gecko-cli/    # The gecko CLI binary and gecko serve HTTP API
│   └── gecko-wasm/   # WASM bindings exposing scorer and geometry to the browser
├── web/              # TypeScript / React / Three.js frontend with 3D viewer
├── data/             # Bundled boards, corpora, profiles, and reference layouts
├── context/          # Design specifications and architecture documentation
└── research/         # Research notes, literature surveys, and experimental findings
```

## Fast Development Commands

GECKO uses `make` to manage hybrid Rust/WASM/TypeScript builds:

- **Fast dev build:**
  ```bash
  make        # or `make dev`
  ```
  Builds Rust, compiles WASM in fast mode (without `wasm-opt`), and builds the web client without the separate TypeScript check pass.
- **Fast dev server:**
  ```bash
  make serve  # runs `make dev` and starts `./target/release/gecko serve` at http://127.0.0.1:7777
  ```
- **Web UI development:**
  ```bash
  cd web && npm install
  npm run dev   # starts Vite dev server with hot reload, proxying /api to gecko serve
  ```
- **Release build:**
  ```bash
  make release        # full release build with wasm-opt and tsc typecheck
  make serve-release  # runs release build and starts server
  ```

## Testing & Quality Checks

Run the full gate that CI enforces before submitting changes:

```bash
make test
```

This runs:
1. `cargo test --release`: Rust core unit and integration tests (invariants, delta == full property tests, snapshot tests).
2. `cargo clippy --all-targets -- -D warnings`: Rust lints.
3. `cd web && npm run test:parity`: WASM vs CLI parity verification across reference layouts.
4. `cd web && npm run typecheck && npm run test:contract && npm test`: Frontend TypeScript checks, API contract tests against server JSON, and Vitest suite.
5. `python3 -I research/scripts/research.py lint`: Research document links, frontmatter, and index validation.

### Focused checks while iterating

- Rust CLI unit tests: `cargo test -p gecko-cli`
- Rust core tests: `cargo test --release -p gecko-core`
- Rust clippy: `cargo clippy --all-targets -- -D warnings`
- Web unit tests: `cd web && npm test`
- Web typecheck: `cd web && npm run typecheck`

## Development Conventions

- **Performance-critical hot loops:** In `gecko-core`, the layout evaluation inner loop must remain allocation-free, avoid dynamic dispatch, and use delta evaluation. The `delta == full` property test must always pass.
- **Documentation:** When modifying behavior, keep relevant documentation in `context/` updated in the same change.
- **Snapshots:** Reference scores are snapshot-tested with `insta` (`crates/gecko-core/tests/snapshots/`). When updating metrics intentionally, review snapshot diffs with `cargo insta review`.

## Key Documentation

Start at [context/README.md](context/README.md) for an overview of the design docs:

- **Architecture:** [context/architecture.md](context/architecture.md)
- **Objective & Metrics:** [context/objective.md](context/objective.md), [context/metrics-catalog.md](context/metrics-catalog.md), [context/parameters.md](context/parameters.md)
- **Hand Model & Geometry:** [context/hand-model.md](context/hand-model.md), [context/geometry.md](context/geometry.md)
- **Algorithms:** [context/algorithms/README.md](context/algorithms/README.md)
- **CLI Reference:** [context/cli.md](context/cli.md)
- **Web Architecture:** [context/web.md](context/web.md), [context/design.md](context/design.md)
- **Testing & Benchmarks:** [context/testing.md](context/testing.md)
- **Coding Conventions:** [context/conventions.md](context/conventions.md)
- **Decisions Log:** [context/decisions.md](context/decisions.md)
