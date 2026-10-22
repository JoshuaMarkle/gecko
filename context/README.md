# context/

The design reference for GECKO. Read only what your task needs.

| Task | Read |
|---|---|
| Anything, first time | [goals.md](goals.md), [architecture.md](architecture.md) |
| Geometry, layout, corpus formats | [representation.md](representation.md) |
| Scoring, metrics, weights, profiles | [objective.md](objective.md), [metrics-catalog.md](metrics-catalog.md), [parameters.md](parameters.md) |
| An optimizer | [algorithms/README.md](algorithms/README.md) + that algorithm's file |
| CLI commands or output | [cli.md](cli.md) |
| Web UI or 3D viewer | [web.md](web.md), [design.md](design.md) (visual design) |
| Tests, benchmarks | [testing.md](testing.md) |
| Code style, layout, tooling | [conventions.md](conventions.md) |
| Why something is the way it is | [decisions.md](decisions.md) |

Rules for this folder: keep files short and current. When code and docs disagree,
fix whichever is wrong in the same change. Record new decisions in `decisions.md`.
