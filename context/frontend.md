# Frontend redesign: open points

The redesign from the user's sketches (2026-10-21) is built; its views are described in
[web.md](web.md). What is still placeholder or missing behind those views:

- **Hand in optimization:** needs a hand model in the scorer (#92). Until then the toggle
  only previews.
- **Hand data:** size distribution and similarity use a synthetic population; they need
  reference hand measurements (see `research/`).
- **Estimators:** board goodness, time-to-optimum and chance-of-optimal are heuristics
  (`board.ts`, `estimate.ts`); the Gilmore–Lawler bound (#37) is shown when a run saved one.
  Stop confidence is saved but not applied to runs.
- **Custom boards:** key edits are kept in the browser and scored in WASM; running them needs
  a saved geometry format and a write/upload API (`representation.md`). Board constraints
  (Custom on) are saved but nothing builds a board from them yet.
- **Languages:** local languages (IndexedDB sources, WASM summary) cannot be used for runs;
  needs a corpus upload route wrapping `gecko corpus build`. Bundled languages are read-only.
