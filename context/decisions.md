# Decisions

Non-obvious choices and why they were made, one line each: date — decision — why.
Newest last in each file. Add an entry to the file for its area when you make a choice
that someone would otherwise question or undo.

Keep entries to one short line: the choice and the reason, not the spec. How something
works belongs in its topic doc (or a code comment); numbers and results belong in the
algorithm file's Results section. If the reason fits as a comment on the code it governs,
put it there instead of here.

| File | Area |
|---|---|
| [decisions/project.md](decisions/project.md) | Stack, process, CI, testing approach, papers and repository organization. |
| [decisions/scoring.md](decisions/scoring.md) | Metrics, profiles, the scorer, corpora, layouts, layers and adaptive keys. |
| [decisions/search.md](decisions/search.md) | Optimizers, runs and their reproducibility, exploration, quality-diversity and bounds. |
| [decisions/geometry.md](decisions/geometry.md) | Boards (2D, parametric, 3D), KLE import, collision, derived costs and the hand model. |
| [decisions/web.md](decisions/web.md) | The web app: views, interaction, local data and visual design. |
| [decisions/promo.md](decisions/promo.md) | The promo video (`promo/`): stack, rendering, data. |

When a decision reverses an earlier one, say so in the new entry ("reverses …") and edit the
old entry's text to note it was superseded, rather than deleting it.
