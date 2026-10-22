# Architecture

## Stack

- **Rust** for the core, CLI, server, and WASM build.
- **TypeScript** + Vite + React + Three.js (via `@react-three/fiber`) for the web UI.
- Data files: TOML for hand-written config, JSON for machine output, binary for
  compiled n-gram tables.

## Repository layout

```
gecko/
├── AGENTS.md / CLAUDE.md   # agent instructions (CLAUDE.md imports AGENTS.md)
├── Cargo.toml · Makefile   # workspace; `make`, `make test`, `make serve`
├── crates/
│   ├── gecko-core/         # library; no I/O beyond reading/writing data files
│   ├── gecko-cli/          # `gecko` binary (incl. `gecko serve`)
│   └── gecko-wasm/         # wasm-bindgen wrapper over gecko-core
├── web/                    # TS UI (src/), API contract check (contract/)
├── data/
│   ├── geometries/         # boards (TOML, parametric `.param.toml`, KLE import)
│   ├── corpora/            # sources + compiled tables
│   ├── profiles/           # metric weights (TOML)
│   ├── layouts/            # reference layouts
│   ├── comfort/            # measured comfort tables
│   └── community/          # cmini community layout bundle
├── runs/                   # optimization outputs (gitignored except curated results)
├── bench/                  # benchmark history and curated benchmark runs
├── research/               # evidence: sources, studies, experiments, open questions
├── docs/papers/            # paper templates (HTML/LaTeX)
├── .github/workflows/      # CI (Rust tests, clippy, rustdoc; web typecheck, tests, contract)
└── context/                # this folder
```

## gecko-core modules

| Module | Responsibility |
|---|---|
| `geometry` | Keys (pos, rot, size, finger, effort), fingers, hands; TOML parse and print. Submodules: `parametric` (zones/columns/rows → ordinary geometry), `derive` (G4 effort/stretch from positions), `pose` (3D key poses, per-hand measuring frames), `flat2d` (clean 2D drawing), `collision` (G04 2D keycap overlap, SAT), `collision3d` (H8 keycap + switch hulls, grid + GJK/EPA) |
| `hand` | Hand model: skeleton, placement over home keys, finger IK, 4-DoF thumb, per-key G01/G03/G05 costs ([hand-model.md](hand-model.md)) |
| `layout` | Slot-ordered symbols; rows ↔ flat; TOML; hash |
| `magic` | Adaptive (magic / repeat) keys |
| `corpus` | Text → n-gram count tables; binary `.gkc` format; mixing |
| `profile` | Metric weights, finger targets, trigram coverage |
| `metrics` | Metric definitions + per-geometry cost tables (`metrics/mod.rs`; trigram classes in `metrics/trigram.rs`) |
| `scorer` | Folded tables and evaluation: `build` (fold metrics × weights, re-weight, re-cost a moved geometry), `eval` (hot loop: full, delta, `apply`), `load` (finger-load costs), `qap` (QAP terms for bounds) |
| `bound` | Gilmore–Lawler lower bound and optimality gap |
| `calibrate` | Spread-normalized and judge-fitted profile weights; rank agreement |
| `report` | Per-metric breakdown, finger loads, heat, worst n-grams, worst words |
| `optimize` | `Optimizer` trait, threaded runner, top-K; `hill_climb`, `anneal`, `genetic`, `memetic`, `bls`, `tabu`, `map_elites`, `nsga2` |
| `explore` | Landscape probe, local-optima archive, atlas files, distances/symmetry, atlas statistics, MAP-Elites grid |
| `cmaes` | G5 CMA-ES with IPOP restarts and box bounds (continuous geometry parameters) |
| `geomsearch` | G6 board + layout search: free parameters of a parametric board, CMA-ES outside, warm-started layout SA inside, geometry run directories |
| `interop` | Other tools: `kle` (keyboard-layout-editor.com import/export), `import` (Oxeylyzer, genkey, cmini layout files), `community` (cmini bundle, duplicates, rank agreement) |
| `run` | Run directories (config, progress, best), listing |
| `store` | The data directory (`gecko.toml`, `data/`, `runs/`) and layout specs |
| `rng` · `hash` · `error` | Seeded xoshiro256++ (identical on native and WASM); FNV-1a content hashes; error type |

Dependency direction: `metrics → geometry, corpus`; `scorer → metrics`;
`optimize → scorer, layout`; `explore → optimize` (via the `Observer` trait in `Ctx`); `run → optimize, explore`;
`geomsearch → cmaes, optimize, geometry`. No cycles. `gecko-core` has no CLI or HTTP code.

## Data flow

```
text ─► corpus ─► n-gram tables ┐
geometry (+ hand model) ─► feature tables ┼─► scorer (folded tables) ─► optimizer ─► runs/
profile (weights) ─────────────────────────┘                                     │
                                                       CLI (JSON) ◄──────────────┤
                                                       web (HTTP + WASM) ◄───────┘
```

## Concurrency

Optimizers run independent restarts/islands on `std::thread::scope` threads (no thread-pool
crate). The scorer is immutable and shared by reference (`&Scorer` in `Ctx`); each worker owns
its layout and RNG (seeded from run seed + worker index). Progress reaches the caller over an
`mpsc` channel; `optimize::Control` (atomics) cancels or pauses.

## Web/server split

The CLI is `crates/gecko-cli/src/`: `main.rs` (clap entry), `cmd/` (one module per command
group), `ctx.rs` (shared inputs and loading), `fmt.rs` (text tables). `gecko serve` (axum,
`serve.rs`; `web_assets.rs` embeds or locates `web/dist`) serves `web/dist` and a JSON API
over `data/` and `runs/`; it starts runs on a background thread and streams progress (SSE).
The browser loads `gecko-wasm` (built into `web/src/wasm/pkg`) for live re-scoring, so the
editor's numbers always match the CLI; it falls back to `POST /api/score` if WASM fails.
