# Decisions

Newest last. Format: date — decision — why. Agents add entries for any non-obvious choice.

- 2026-10-20 — Rust core, CLI, server, WASM. — Speed in the hot loop; one scorer shared by CLI and browser.
- 2026-10-20 — TypeScript (not plain JS) for the web UI. — Types help agents work reliably with little supervision; the user is fine with TS for that reason.
- 2026-10-20 — React + react-three-fiber for the UI. — Widely known by agents; good Three.js integration.
- 2026-10-20 — Geometry / layout / corpus kept separate; positions always 3D. — 2D → 3D without rewriting the core.
- 2026-10-20 — Linear metrics folded into one table per n-gram order; delta evaluation for swaps. — Metric count doesn't slow search.
- 2026-10-20 — Start with the 30 main keys on a standard row-staggered board. — Requested; symbol layers come later.
- 2026-10-20 — Geometry phases: 2D (x, y, rotation) → z axis + hand model → free-form 3D curved with no key intersections. — Requested roadmap.
- 2026-10-20 — Development is autonomous. Agents make reasonable decisions without asking and record them here. — Requested by the user.
- 2026-10-20 — Web visual design adapted from the user's JAGUAR design doc into `context/design.md` (monochrome, one accent, semantic colors for state only, dense tool shell). — Requested; keeps both projects' tools consistent.
- 2026-10-20 — Profile weights apply to metric values in natural units (%, avg effort); direction comes from the metric so weights are always ≥ 0. Spread-normalization deferred. — Simple and readable; default weights tuned so the ranking test passes.
- 2026-10-20 — Frequencies normalized over the symbols on the layout. — Layouts with different punctuation stay comparable; matches common analyzers.
- 2026-10-20 — Trigrams stored sparsely per symbol with `trigram_coverage` = 0.99 (renormalized) in the profile. — ~2× faster deltas; scoring, deltas and reports use the same table so delta == full holds.
- 2026-10-20 — Own xoshiro256++ RNG instead of `rand`. — Identical streams on native and WASM, no dependency churn.
- 2026-10-20 — Metric tables grouped in `metrics/mod.rs` (+ `trigram.rs`) instead of one file per metric. — Most metrics are 1–3 lines; one file each would be noise. Split when a metric grows.
- 2026-10-20 — Optimizers boxed as `dyn Optimizer` per worker; dispatch is per step (≥ thousands of evals), never per evaluation. — Keeps the runner simple without touching the hot loop.
- 2026-10-20 — SA `steps = 0` means auto: about 4 restarts per worker within the budget. — Fixed 1M-step schedules never finished on short runs with many threads.
- 2026-10-20 — Bundled corpus: 10 public-domain Project Gutenberg novels (6.1M chars); compiled `data/corpora/english.gkc` is committed, raw text is not. — Works offline; reproduces published QWERTY SFB (6.63% vs ≈ 6.6%). Add chat/code corpora later.
- 2026-10-20 — SSE terminal events are `done` / `failed` (not `error`). — `error` collides with EventSource's built-in connection error event.
- 2026-10-20 — Web toolbar modeled on the Unity editor: icon view tabs (Home, Results, Layout analysis) left, Play / Pause / End centred, progress right; wordmark removed; Algorithm moved into Inputs › Search. — Requested by the user.
- 2026-10-20 — Transport active state uses `--accent` (Unity's blue play-mode tint) instead of ink. — The live run is "the one thing that needs attention"; keeps the one-accent rule.
- 2026-10-20 — Pause is real: `optimize::Control` (cancel + pause) replaces the bare cancel flag; workers sleep while paused and paused time is excluded from the time budget and `elapsed_ms`. — A UI-only pause would let the budget expire while paused. The deadline check only takes the pause lock once wall time passes the budget, so the hot loop is unchanged.
