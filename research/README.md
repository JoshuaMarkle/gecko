# research/

Evidence behind GECKO's design. **Research records what is known and where it came from, and is
dated. `context/` records what was decided and is kept current.** If a page says what the code
does, it belongs in `context/`.

Start with [SYNTHESIS.md](SYNTHESIS.md): what the sources say to build next, with the status of
each recommendation. Then open only the page you need.

| Page | Read when |
|---|---|
| [SYNTHESIS.md](SYNTHESIS.md) | Deciding what to build next; why a metric or method is worth it |
| [OPEN-QUESTIONS.md](OPEN-QUESTIONS.md) | Finding unverified claims and uncovered topics; planning research |
| [sources/](sources/README.md) | Implementing an idea: one summary per source, with an index by group and status |
| studies/ | A design exploration that spans sources (below) |
| experiments/ | A dated result produced by GECKO itself, linked to its run directory: [profile ranking](experiments/2026-10-21-profile-ranking.md), [cost-function validity](experiments/2026-10-22-cost-function-validity.md), [skipgram timing](experiments/2026-10-22-skipgram-timing.md), [cost-function optimization](experiments/2026-10-24-cost-function-optimization.md), [English layout](experiments/2026-10-24-english-layout.md), [programming layout](experiments/2026-10-25-programming-layout.md), [Vim layout](experiments/2026-10-25-vim-layout.md) |
| [archive/](archive/design-draft-v0.md) | History only: the first design draft, superseded by `context/` |

Studies: [search-space-visualization](studies/search-space-visualization.md) (exploring the
landscape, escaping local optima, the Explore mode) and
[tool-comparison](studies/tool-comparison.md) (other tools side by side),
[literature-survey-2023-2026](studies/literature-survey-2023-2026.md) (recent papers; mostly a gap),
and [English corpus audit](studies/2026-10-24-english-corpus-audit.md) (adequacy, preprocessing,
and source-separated evaluation), and the [research plan](studies/2026-10-24-research-plan.md)
(what to research next, toward the best standard keyboard).

Moved to `context/` because they specify the code: [metrics-catalog](../context/metrics-catalog.md)
(every metric and its ID), [parameters](../context/parameters.md) (what describes, steers and
judges a layout).

## Conventions

- **Sources:** one file per source, from [`_template-source.md`](_template-source.md). Frontmatter
  (`group`, `kind`, `read`, `status`, `feeds`, `take`) drives the index; run
  `python3 -I research/scripts/research.py index` after changing it.
- **Studies:** a cross-source design question. Date it, say what it recommends, and keep a status
  line saying what was built.
- **Experiments:** file name `YYYY-MM-DD-name.md`, with a header table (question, answer, command,
  data, artifacts). Heavy output goes in `bench/`, not here.
- **Uncertainty:** mark any number not read at the primary source **(verify)** and add a row to
  [OPEN-QUESTIONS.md](OPEN-QUESTIONS.md). Remove both when closed.
- **One home per fact.** Link instead of copying. Prefer a table or a number over prose.
- **Check:** `python3 -I research/scripts/research.py lint` (runs in `make test`) fails on missing
  frontmatter, broken links, unlinked pages, stale `research/` paths cited elsewhere in the repo,
  and a stale index.

## Confidence

Research was done 2026-10-20 (hand-model parameters checked 2026-10-21). Repositories were cloned
and read directly. Several primary sites (carpalx at bcgsc.ca, the Aalto 136M keystrokes page (its data was read from a Kaggle mirror instead), MPI
keyboard-optimization pages, semilin.github.io) were unreachable, so those details come from
mirrors, archives or abstracts and are marked `secondary` in the index.
