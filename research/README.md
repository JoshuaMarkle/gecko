# research/

Background research behind GECKO's design. **Start with [SYNTHESIS.md](SYNTHESIS.md)**: what
the sources say to build next, with a prioritized backlog. Then open only the file you need.

| File | Read when |
|---|---|
| [SYNTHESIS.md](SYNTHESIS.md) | Deciding what to build next; why a metric or method is worth it |
| [search-space-visualization.md](search-space-visualization.md) | Visualizing runs and the fitness landscape, exploring the space, escaping local optima, the Explore mode |
| [sources/](sources/README.md) | Implementing a specific idea: one summary per source with code/data links |
| [existing-optimizers.md](existing-optimizers.md) | Quick comparison table of other tools |
| [metrics-catalog.md](metrics-catalog.md) | Adding or changing a metric. The master list of ways to judge a layout |
| [parameters.md](parameters.md) | Designing config/schema: what describes a keyboard, what steers optimization |
| [hand-model-and-geometry.md](hand-model-and-geometry.md) | Working on 2D/3D physical geometry or the hand model |
| [data-sources.md](data-sources.md) | Building corpora or timing models (also `sources/corpora.md`) |
| [design-draft-v0.md](design-draft-v0.md) | History only. The agreed design now lives in `context/` |

## Confidence notes

Research was done 2026-10-20. Repositories were cloned and read directly. Some primary sites
(carpalx at bcgsc.ca, the Aalto 136M keystrokes page, MPI keyboard-optimization pages,
semilin.github.io) were unreachable from the research environment, so a few details come from
mirrors, archives or abstracts and are marked **(verify)**. Check the primary source before
hard-coding any number marked that way.
