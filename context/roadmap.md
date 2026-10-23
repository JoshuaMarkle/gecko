# Roadmap

Future work only. Task scope lives in the [open GitHub issues](https://github.com/JoshuaMarkle/gecko/issues?q=is%3Aissue%20state%3Aopen).
Take the next priority below, then the first unblocked backlog item. Remove completed
items instead of keeping checked boxes. Update issue scope before changing its roadmap
summary; follow `AGENTS.md` for the working process.

## Next priorities

Evidence and packaging before more features: the scorer must be shown correct (V1) and the
optimizers shown to beat known layouts (V2) before results are shared or published.

1. **[#141](https://github.com/JoshuaMarkle/gecko/issues/141) · Phase 2 evaluation · M.**
   30 seeds per board/corpus vs Graphite, Sturdy, Semimak, Canary; holdout and bounds.
   Phase 1 validation (#140) is in `context/objective.md` §Validation.
2. **[#142](https://github.com/JoshuaMarkle/gecko/issues/142) · Release readiness · S.**
   README for outsiders, `CITATION.cff`, contributor guide. The owner tags releases.

## Remaining backlog

Sizes are relative effort (S/M/L). Dependencies below list only unfinished work;
“ready” means no open prerequisite, not that implementation design is settled.

| Issue | Remaining work | Size | Dependency |
|---|---|---|---|
| [#103](https://github.com/JoshuaMarkle/gecko/issues/103) | Optimize mean or worst-case cost over hand sizes; compare with single-hand optima | M | Ready |
| [#74](https://github.com/JoshuaMarkle/gecko/issues/74) | Optional differentiable reach surrogate; compare refinement time-to-quality with CMA-ES | L | Ready |

## Path to a printable board

`gecko build stl` and the Export tab's case preview produce a printable parametric board, and
the Export tab writes an ergogen config. [`context/build-guide.md`](build-guide.md) walks from
hand measurements to an optimized board and its STL ([#77](https://github.com/JoshuaMarkle/gecko/issues/77)).
Free-form search (`--free keys`, #70) refines a parametric best per key; it is not a prerequisite. Thumb clusters
follow the thumb's arc with G06 and `--thumb-keys` (#71).
