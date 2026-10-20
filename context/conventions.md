# Conventions

## Rust

- Edition 2021+, stable toolchain. `cargo fmt` (`rustfmt.toml`: width 140),
  `cargo clippy --all-targets -- -D warnings`.
- Hot paths: no allocation, no `HashMap`, no dynamic dispatch inside the inner loop.
  Use flat `Vec<f32>` tables with index helpers. Optimizers are generic over the scorer.
- Errors: `thiserror` in core, `anyhow` in the CLI.
- Crates in use: `serde`, `toml`, `serde_json`, `thiserror`, `anyhow`, `clap`, `axum`,
  `tokio`, `tower-http`, `wasm-bindgen`, `proptest`. Threads are `std::thread::scope`;
  the RNG is our own (`gecko_core::rng`) so results match across platforms.
- One algorithm per module. Metric table builders live in `metrics/` keyed by catalog ID
  (trigram classes in their own file); split a metric into its own file once it needs more
  than a few lines. Each has a doc comment linking its catalog ID or algorithm file.

## TypeScript

- Strict mode. Types for API responses generated from (or kept identical to) the
  Rust `serde` types.
- Keep components small; 3D code under `web/src/three/`.

## Data

- Hand-written config: TOML. Machine output: JSON / JSONL. Large tables: binary with a
  header (magic, version, hash).
- Everything referenced by a run is identified by content hash.

## Docs

- Concise. Update `context/` in the same change as the code it describes.
- Algorithm and metric files: what, why, parameters, pitfalls, results. No essays.
