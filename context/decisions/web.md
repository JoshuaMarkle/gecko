# Decisions: Web UI

The web app: views, interaction, local data and visual design. Newest last. Format: date — decision — why.

- 2026-10-20 — Visual design adapted from the user's JAGUAR design doc (`design.md`). — Keeps both projects' tools consistent.
- 2026-10-20 — SSE terminal events are `done` / `failed`, not `error`. — `error` collides with EventSource's built-in connection error.
- 2026-10-20 — The transport's active state uses `--accent`. — The live run is the one thing that needs attention; keeps the one-accent rule.
- 2026-10-20 — Atlas statistics run in WASM, not on the server. — Re-weighting and brushing 10⁴–10⁵ optima must be instant.
- 2026-10-20 — The PCA map is fit on ≤ 3000 strided rows, independent of weights. — Points stay put while weights change.
- 2026-10-20 — Card grids use `grid-template-columns: minmax(0, 1fr)`. — An `auto` track locked to the canvas's initial width and overflowed.
- 2026-10-20 — The finger palette is vivid. — Requested; hues stay lighter than the accent so selection stays visible.
- 2026-10-21 — Input state is one hook (`inputs.ts`) owned by App and passed as props, not a context. — One owner, no extra dependency.
- 2026-10-21 — The layout editor is a 2D click-to-swap overlay on Search, with a Pin mode. — Requested in the frontend redesign.
- 2026-10-21 — The working model (`gecko.model`) is the layout adopted from a run, a saved layout or an Explore pick. — Spec: "the layout selected after an optimization".
- 2026-10-21 — Custom boards live in browser storage and score in WASM only; Play refuses them. — No geometry upload route yet; bundled boards are never modified.
- 2026-10-21 — Local languages live in IndexedDB and are summarized in WASM, for stats only. — Real language stats without server write routes.
- 2026-10-21 — Corpus sources are served read-only from `data/corpora/src/<name>/`. — The texts already lived there.
- 2026-10-21 — The Hand tab's population is synthetic (scale sd 5.5%, shape noise 4%) and labelled so. — Placeholder until reference hand data lands.
- 2026-10-21 — Board goodness (comfort 40, spread 25, thumbs 20, compactness 15), chance of optimal and time to optimum are heuristics. — Cheap, honest estimators until proper ones exist.
- 2026-10-21 — Stop confidence and board constraints are saved but have no effect yet. — The spec asked for the views first.
- 2026-10-21 — Hands are one right-hand skeleton mirrored for the left, placed by fitting each finger's natural curl to its home key. — Symmetric by construction.
- 2026-10-21 — The typing preview paces each finger naturally and reports the speed reached; strict rhythm stays as `planTyping(strict)`. — Forcing the rhythm made fingers jump; user preference after seeing both.
- 2026-10-21 — Explore uses the shared sidebar width (380px, 340px ≤ 1360px). — Consistent left controls across tabs.
- 2026-10-21 — "Use hand in optimization" adds G01/G05 only where the profile has no weight, and drops them again unless edited. — Changes runs without rewriting a user's explicit weights.
- 2026-10-21 — Export files are built in the browser, except analyzer formats, which go through WASM `export_layout`. — Custom boards export too, and analyzer formats match `gecko layouts export`.
- 2026-10-21 — Export image is one SVG for preview and download; KLE export uses the flattened 2D drawing. — The preview is exactly the file; raw 3D positions overlap in KLE.
- 2026-10-21 — L10 (#87) keeps `Board2D.Thumb` as the mini keyboard instead of a new `MiniKeyboard`, and drops the "Optimize result sidebar" step. — Thumb was already the shared component; the Optimize tab became Home, whose main view already shows the result keyboard in 3D.
- 2026-10-22 — Board + layout runs (G9 #67 and the run controls) go through `POST /api/runs` with `geom` and the same live-run plumbing; candidate boards are built in the browser with WASM `param_board`. — Pause/stop/SSE come for free, and previews, the live board and replays need no new server routes.
- 2026-10-22 — A finished geometry run's best board is a board like any other (`run:<id>`, also for `-g`), and finishing a run adopts it with its layout. — Matches layout runs adopting their best layout; the board can be scored, layout-optimized or shape-searched again.
- 2026-10-22 — Profiles picked from the list keep their own G01/G03/G05 weights (`loadProfile`); only edits from views drop weights "Use hand" added. — The bundled `hand` profile had its hand-model weights stripped on load, so boards were searched without them.
- 2026-10-22 — The 3D view raises boards whose keys dip below the case top. — Tented halves pivot about their inner edge, which sank the outer columns into the case.
