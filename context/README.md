# context/

The design reference for GECKO. Which file to read for a task: the table in
[../AGENTS.md](../AGENTS.md). Every file here:

| File | Contents |
|---|---|
| [goals.md](goals.md) | What GECKO is for, scope, success criteria |
| [architecture.md](architecture.md) | Repository layout, crates and modules, data flow |
| [representation.md](representation.md) | Geometry, layout, corpus and run formats |
| [geometry.md](geometry.md) | Parametric and 3D boards, collision, derived effort |
| [hand-model.md](hand-model.md) | Hand skeleton, IK, reach / press-angle / wrist-posture costs |
| [objective.md](objective.md) | Scoring, normalization, profiles, calibration |
| [metrics-catalog.md](metrics-catalog.md) | Every metric by ID |
| [parameters.md](parameters.md) | Every tunable parameter and its default |
| [algorithms/](algorithms/README.md) | One file per optimizer, with results |
| [cli.md](cli.md) | Commands, output, conventions |
| [web.md](web.md) | Web stack, code layout, rendering, API |
| [web-screens.md](web-screens.md) | What each web tab shows and does |
| [design.md](design.md) | Visual design system |
| [build-guide.md](build-guide.md) | Walkthrough: hand measurements → optimized board → STL |
| [testing.md](testing.md) | Tests, benchmarks, CI |
| [conventions.md](conventions.md) | Code style, crates, tooling |
| [decisions.md](decisions.md) | Past decisions by area, in [decisions/](decisions/) |
| [roadmap.md](roadmap.md) | Timeline and GitHub issues |

Rules for this folder: keep files short and current. When code and docs disagree,
fix whichever is wrong in the same change. Record new decisions in `decisions/<area>.md`.
