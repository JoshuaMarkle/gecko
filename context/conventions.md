# Conventions

## Rust

- Edition 2021+, stable toolchain. `cargo fmt` (`rustfmt.toml`: width 140),
  `cargo clippy --all-targets -- -D warnings`.
- Hot paths: no allocation, no `HashMap`, no dynamic dispatch inside the inner loop.
  Use flat `Vec<f32>` tables with index helpers. Optimizers take the concrete `&Scorer`; a
  boxed `dyn Optimizer` is dispatched once per step (thousands of evaluations), never per evaluation.
- Errors: `thiserror` in core, `anyhow` in the CLI.
- Crates in use: core `serde`, `serde_json`, `toml`, `thiserror`; CLI `clap`, `anyhow`, `axum`,
  `tokio`, `tokio-stream`, `futures-util`, `tower-http`, `mime_guess`, `percent-encoding`;
  WASM `wasm-bindgen`; tests `proptest`, `insta`, `criterion`, `tower`. Threads are `std::thread::scope`;
  the RNG is our own (`gecko_core::rng`) so results match across platforms.
- One algorithm per module. Metric table builders live in `metrics/` keyed by catalog ID
  (trigram classes in their own file); split a metric into its own file once it needs more
  than a few lines. Each has a doc comment linking its catalog ID or algorithm file.
- Group a module into a directory (`geometry/`, `scorer/`, `interop/`) once it has distinct
  parts; keep its public paths stable with `pub use` in `mod.rs`.
- CLI: one module per command group in `gecko-cli/src/cmd/`, each a clap `Subcommand`/`Args`
  with a `run(self, &Ctx)`; shared loading lives in `ctx.rs`, text tables in `fmt.rs`.

## TypeScript

- Strict mode. API types in `web/src/api.ts` are hand-written subsets of the Rust `serde`
  types; `npm run test:contract` checks them against real responses.
- Code ported from Rust (the hand model) is checked by a parity fixture written by Rust.
- Keep components small: one file per tab in `components/views/`, its parts in a folder named
  after it (`components/hand/`, `components/explore/`); 3D code under `web/src/three/`.
- Pure logic goes in `web/src/*.ts` with a `*.test.ts`; components hold UI state only.
- Persisted UI preferences use `usePref` (`inputs.ts`); wrap storage access in try/catch.

## Data

- Hand-written config: TOML. Machine output: JSON / JSONL. Large tables: binary with a
  header (magic, version, hash).
- Everything referenced by a run is identified by content hash.

## Docs

- Concise. Update `context/` in the same change as the code it describes.
- Algorithm and metric files: what, why, parameters, pitfalls, results. No essays.
