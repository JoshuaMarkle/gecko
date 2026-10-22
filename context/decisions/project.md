# Decisions: Project, tooling, tests and docs

Stack, process, CI, testing approach, papers and repository organization. Newest last. Format: date — decision — why.

- 2026-10-20 — Rust core, CLI, server and WASM. — Hot-loop speed; one scorer shared by CLI and browser.
- 2026-10-20 — TypeScript for the web UI. — Types help agents work reliably with little supervision.
- 2026-10-20 — React + react-three-fiber. — Widely known by agents; good Three.js integration.
- 2026-10-20 — Own xoshiro256++ RNG instead of `rand`. — Identical streams on native and WASM; no dependency churn.
- 2026-10-20 — Backlog lives in GitHub issues (milestones = tracks, blocked-by links); `roadmap.md` mirrors it in order, without dates. — Agents can pick unblocked work unattended; dates mean little at AI pace.
- 2026-10-21 — The overnight harness works in a separate worktree, one PR per issue off main, skipping issues with open blockers. — Leaves the user's checkout alone; no stacked PRs.
- 2026-10-21 — CI runs parallel rust and web jobs; wasm-pack comes from `taiki-e/install-action`. — Mirrors `make test`; the rustwasm installer is unmaintained.
- 2026-10-21 — Metric unit tests build boards and corpora in code and check each value through both `breakdown` and `Scorer::full`. — Hand arithmetic stays readable and tests don't move when `data/` changes.
- 2026-10-21 — Reference snapshots record the score plus weighted metrics only, at 4 decimals. — The diff shows what moved; 4 decimals absorb f32 summation noise.
- 2026-10-21 — Benchmark history is `gecko perf --record` JSONL, not criterion output; criterion is for before/after comparisons. — Criterion's JSON lives in `target/` and changes between versions.
- 2026-10-21 — Issue and PR templates mirror the existing issue body and the gate commands. — Agents see one format.
- 2026-10-21 — WASM/CLI parity runs the shipped web-target WASM under Node against native `compare --json` fixtures. — Tests the real binary without a second build target.
- 2026-10-21 — `embed-web` feature embeds `web/dist`; `/api` stays isolated so API 404s are JSON. — Single-binary releases; API clients never get `index.html`.
- 2026-10-21 — The Claude Code action triggers only on the repo owner's `@claude`. — Outside users can't spend the API key.
- 2026-10-21 — Release/test profiles are incremental with 256 codegen units; `--profile max` has full optimization. — A one-line core change rebuilds in ~1.3 s instead of ~16 s.
- 2026-10-21 — The paper template is one journal design (dependency-free HTML + a XeLaTeX PDF build) in `docs/papers/templates`. — The user chose the journal direction; local TeX lacks a XeLaTeX format, so the build makes a temporary one.
- 2026-10-21 — `research/` holds dated evidence only; specs (`metrics-catalog.md`, `parameters.md`) live in `context/`; the source index is generated and linted by `make test`. — The folder mixed evidence, specs and tooling and couldn't flag stale claims.
- 2026-10-21 — API types stay hand-written in `web/src/api.ts`, checked by `npm run test:contract`, not generated with ts-rs. — They are deliberate UI subsets; the check catches the drift that actually happens.
- 2026-10-21 — The TS hand model stays a port of Rust, held equal by a Rust-written fixture, not WASM calls. — The tab animates per frame and needs joints the scorer never computes.
- 2026-10-21 — `bench/` run copies are kept as recorded. — Time-budget runs can't be regenerated; they back the leaderboards.
- 2026-10-21 — `CLAUDE.md` is just `@AGENTS.md`; `context/README.md` is the file index; decisions and web docs split by area. — Duplicated routing tables had drifted apart.
- 2026-10-21 — Decision entries trimmed to one line; specs live in topic docs and code. — The log had grown into a second spec that agents had to read.
