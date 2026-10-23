# Roadmap

Future work only. Task scope lives in the [open GitHub issues](https://github.com/JoshuaMarkle/gecko/issues?q=is%3Aissue%20state%3Aopen).
Take the next priority below, then the first unblocked backlog item. Remove completed
items instead of keeping checked boxes. Update issue scope before changing its roadmap
summary; follow `AGENTS.md` for the working process.

## Next priorities

Evidence and packaging before more features: the scorer must be shown correct (V1) and the
optimizers shown to beat known layouts (V2) before results are shared or published.

1. **[#140](https://github.com/JoshuaMarkle/gecko/issues/140) · Phase 1 validation · M.**
   SFB and other numbers vs cmini on the same corpus; ranking vs community consensus.
2. **[#141](https://github.com/JoshuaMarkle/gecko/issues/141) · Phase 2 evaluation · M.**
   30 seeds per board/corpus vs Graphite, Sturdy, Semimak, Canary; holdout and bounds.
   Depends on #140.
3. **[#142](https://github.com/JoshuaMarkle/gecko/issues/142) · Release readiness · S.**
   README for outsiders, `CITATION.cff`, contributor guide. The owner tags releases.

## Remaining backlog

Sizes are relative effort (S/M/L). Dependencies below list only unfinished work;
“ready” means no open prerequisite, not that implementation design is settled.

| Issue | Remaining work | Size | Dependency |
|---|---|---|---|
| [#45](https://github.com/JoshuaMarkle/gecko/issues/45) | Serve benchmark results and show performance profiles / time-to-target ECDFs in the web UI | M | Ready |
| [#97](https://github.com/JoshuaMarkle/gecko/issues/97) | Search allowed finger assignments per key; compare against fixed angle mod | M | Ready |
| [#53](https://github.com/JoshuaMarkle/gecko/issues/53) | Record per-worker convergence, temperature and acceptance telemetry; show process views | M | Ready |
| [#55](https://github.com/JoshuaMarkle/gecko/issues/55) | Adaptive top-K n-gram scoring with full-objective confirmation; benchmark speed and quality | M | Ready |
| [#100](https://github.com/JoshuaMarkle/gecko/issues/100) | Local-optima / trajectory networks, symbol migration and scored paths between layouts | L | #53 for telemetry-dependent views |
| [#68](https://github.com/JoshuaMarkle/gecko/issues/68) | Finish hover-to-pose interaction and per-key reach overlay, reusing the hand rig and parity-tested TS IK | M | Ready |
| [#103](https://github.com/JoshuaMarkle/gecko/issues/103) | Optimize mean or worst-case cost over hand sizes; compare with single-hand optima | M | Ready |
| [#70](https://github.com/JoshuaMarkle/gecko/issues/70) | Free per-key 6-DoF search with neighbour smoothness and collision repair | L | Ready |
| [#71](https://github.com/JoshuaMarkle/gecko/issues/71) | G06 thumb-IK metric; search thumb key count and poses | M | Ready |
| [#74](https://github.com/JoshuaMarkle/gecko/issues/74) | Optional differentiable reach surrogate; compare refinement time-to-quality with CMA-ES | L | Ready |
| [#75](https://github.com/JoshuaMarkle/gecko/issues/75) | Generate switch plate and case STL from optimized key poses | L | Ready |
| [#76](https://github.com/JoshuaMarkle/gecko/issues/76) | Finish export formats and printable case preview | S | #75 |
| [#77](https://github.com/JoshuaMarkle/gecko/issues/77) | Reproducible hand → optimized board → STL walkthrough | M | #75 |

## Path to a printable board

[#75](https://github.com/JoshuaMarkle/gecko/issues/75) plate and case generation →
[#76](https://github.com/JoshuaMarkle/gecko/issues/76) printable preview and
[#77](https://github.com/JoshuaMarkle/gecko/issues/77) reproducible walkthrough.
Free-form search (#70) and thumb-cluster search (#71) extend the design space;
they are not prerequisites for exporting a parametric board.
