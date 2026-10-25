# Roadmap

Future work only. Task scope lives in the [open GitHub issues](https://github.com/JoshuaMarkle/gecko/issues?q=is%3Aissue%20state%3Aopen).
Take the first unblocked backlog item. Remove completed items instead of keeping checked
boxes. Update issue scope before changing its roadmap summary; follow `AGENTS.md` for the
working process. Done and written up: Phase 1 validation (`objective.md` §Validation, #140),
Phase 2 evaluation ([`evaluation.md`](evaluation.md), #141).

## Remaining backlog

Sizes are relative effort (S/M/L). Dependencies below list only unfinished work;
“ready” means no open prerequisite, not that implementation design is settled.

| Issue | Remaining work | Size | Dependency |
|---|---|---|---|
| [#74](https://github.com/JoshuaMarkle/gecko/issues/74) | Optional differentiable reach surrogate; compare refinement time-to-quality with CMA-ES | L | Ready |

## Path to a printable board

`gecko build stl` and the Export tab's case preview produce a printable parametric board, and
the Export tab writes an ergogen config. [`context/build-guide.md`](build-guide.md) walks from
hand measurements to an optimized board and its STL ([#77](https://github.com/JoshuaMarkle/gecko/issues/77)).
Free-form search (`--free keys`, #70) refines a parametric best per key; it is not a prerequisite. Thumb clusters
follow the thumb's arc with G06 and `--thumb-keys` (#71).
