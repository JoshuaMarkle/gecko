# Architecture

## Stack

- **Rust** for the core, CLI, server, and WASM build.
- **TypeScript** + Vite + React + Three.js (via `@react-three/fiber`) for the web UI.
- Data files: TOML for hand-written config, JSON for machine output, binary for
  compiled n-gram tables.

## Repository layout

```
gecko/
├── AGENTS.md / CLAUDE.md   # agent instructions (CLAUDE.md links to AGENTS.md)
├── Cargo.toml              # workspace
├── crates/
│   ├── gecko-core/         # library; no I/O beyond reading/writing data files
│   ├── gecko-cli/          # `gecko` binary (incl. `gecko serve`)
│   └── gecko-wasm/         # wasm-bindgen wrapper over gecko-core
├── web/                    # TS UI
├── data/
│   ├── geometries/         # boards (TOML, KLE import)
│   ├── corpora/            # sources + compiled tables
│   ├── profiles/           # metric weights (TOML)
│   └── layouts/            # reference layouts
├── runs/                   # optimization outputs (gitignored except curated results)
├── research/               # background research
└── context/                # this folder
```

## gecko-core modules

| Module | Responsibility |
|---|---|
| `geometry` | Keys (pos, rot, size, finger, effort), fingers, hands; TOML parse and print |
| `kle` | KLE JSON import (guessed fingers/rows/home/effort) and export |
| `layout` | Slot-ordered symbols; rows ↔ flat; TOML; hash |
| `corpus` | Text → n-gram count tables; binary `.gkc` format; mixing |
| `profile` | Metric weights, finger targets, trigram coverage |
| `metrics` | Metric definitions + per-geometry cost tables (`metrics/mod.rs`; trigram classes in `metrics/trigram.rs`) |
| `scorer` | Folds metrics × weights into flat tables; full + delta evaluation; `State` |
| `report` | Per-metric breakdown, finger loads, heat, worst n-grams, worst words |
| `optimize` | `Optimizer` trait, threaded runner, top-K; `hill_climb`, `anneal`, `genetic`, `memetic`, `bls`, `tabu`, `map_elites` |
| `explore` | Landscape probe, local-optima archive, atlas files, distances/symmetry, atlas statistics, MAP-Elites grid (`explore/`) |
| `run` | Run directories (config, progress, best), listing |
| `store` | The data directory (`gecko.toml`, `data/`, `runs/`) and layout specs |
| `rng` | Seeded xoshiro256++ (identical on native and WASM) |
| `hand` (phase 7+) | Hand model, IK, reach costs → feeds `geometry` tables |

Dependency direction: `metrics → geometry, corpus`; `scorer → metrics`;
`optimize → scorer, layout`; `explore → optimize` (via the `Observer` trait in `Ctx`); `run → optimize, explore`. No cycles. `gecko-core` has no CLI or HTTP code.

## Data flow

```
text ─► corpus ─► n-gram tables ┐
geometry (+ hand model) ─► feature tables ┼─► scorer (folded tables) ─► optimizer ─► runs/
profile (weights) ─────────────────────────┘                                     │
                                                       CLI (JSON) ◄──────────────┤
                                                       web (HTTP + WASM) ◄───────┘
```

## Concurrency

Optimizers run independent restarts/islands with `rayon`. The scorer's tables are
immutable and shared (`Arc`); each worker owns its layout and RNG
(seeded from run seed + worker index).

## Web/server split

`gecko serve` (axum, `crates/gecko-cli/src/serve.rs`) serves `web/dist` and a JSON API
over `data/` and `runs/`; it starts runs on a background thread and streams progress (SSE).
The browser loads `gecko-wasm` (built into `web/src/wasm/pkg`) for live re-scoring, so the
editor's numbers always match the CLI; it falls back to `POST /api/score` if WASM fails.
