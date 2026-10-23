<h1 align="center">🦎 GECKO 🦎</h1>
<h3 align="center">General Engine for Custom Keyboard Optimization</h3>

**GECKO** searches for the best keyboard layout for any physical keyboard, and later for the
best keyboard itself (2D → 3D). It is first a data-analysis system for judging layouts
thoroughly and fast, with optimization on top.

- Score any layout on any board: 48 metrics (same-finger bigrams, rolls, stretches, reach
  and wrist posture from a hand model, …) weighted by editable profiles.
- Optimize with several algorithms (simulated annealing, tabu, memetic, MAP-Elites, NSGA-II, …)
  using fast swap-delta evaluation; every run is reproducible.
- Explore the landscape of good layouts, not just one optimum.
- Boards from TOML, keyboard-layout-editor.com, or parameters (stagger, splay, split, 3D keywells).
- Web UI with a 3D viewer; the browser scores with the same Rust code through WASM.

## Quick start

Needs Rust (stable), and for the web UI Node and `wasm-pack`.

```
cargo build --release
./target/release/gecko compare                 # rank the reference layouts
./target/release/gecko optimize --time 30s     # writes runs/<id>/
make && ./target/release/gecko serve           # web UI at http://127.0.0.1:7777
make test                                      # everything CI runs
```

Every command takes `--json`. See [context/cli.md](context/cli.md).

## Layout

| Path | |
|---|---|
| `crates/gecko-core` | Library: geometry, corpora, metrics, scorer, optimizers, hand model |
| `crates/gecko-cli` | The `gecko` binary and `gecko serve` |
| `crates/gecko-wasm` | The scorer for the browser |
| `web/` | TypeScript / React / Three.js UI |
| `data/` | Boards, corpora, profiles, reference layouts |
| `context/` | Design docs: start at [context/README.md](context/README.md) |
| `research/` | Evidence behind the metrics and parameters |

Contributors and agents: read [AGENTS.md](AGENTS.md).

## License

Code: [MIT](LICENSE). Some bundled data (the cmini-derived corpora and layout list) is GPL-3.0;
see [NOTICE.md](NOTICE.md).
