# Papers

Drafts of papers that can come out of GECKO. Start with [topics.md](topics.md): the research
questions we want the software to answer, the experiment and figures for each, and what is still
missing. Each paper gets one file in `drafts/` (or the top level for the software paper).

## Focus (four papers)

| # | Paper | File | Status |
|---|---|---|---|
| 1 | Exploring the keyboard layout landscape | [drafts/01-landscape.md](drafts/01-landscape.md) | tooling done; runs pending |
| 2 | Designing a general cost function | [drafts/02-cost-function.md](drafts/02-cost-function.md) | needs sweeps, timing model, user study |
| 3 | Finding the best keyboard layout | [drafts/03-best-layout.md](drafts/03-best-layout.md) | needs exact windows (#43), budget grid |
| 2a | Is the cost function right? (experiment report) | [cost-validity/cost-validity.tex](cost-validity/cost-validity.tex) (PDF beside it; data `bench/cost-validity-2026-10-22/`) | typeset; timing validity, weight robustness, sweep; feeds paper 2 |
| 2b | Optimizing the cost function (experiment report) | [cost-optimization/cost-optimization.tex](cost-optimization/cost-optimization.tex) (PDF beside it; data `bench/costfit-2026-10-24/`) | typeset; timing calibration, bilevel weight search, the 2026-10-24 default; feeds paper 2 |
| 2c | Ranking the modern layouts (experiment report) | [layout-ranking/layout-ranking.md](layout-ranking/layout-ranking.md) (PDF beside it; data `bench/layout-ranking-2026-10-24/`) | done; 31 layouts under the evidence default, swap-polish micro experiment |
| 2d | The best English layout (experiment report) | [english-layout/english-layout.md](english-layout/english-layout.md) (PDF beside it; data `bench/english-layout-2026-10-24/`) | done; one optimum on ansi-30 / `prose` under `default`, certified over 16 letters, compared with 17 layouts |
| 2e | The best programming layout with a symbol layer (experiment report) | [programming-layout/programming-layout.md](programming-layout/programming-layout.md) (PDF beside it; data `bench/programming-layout-2026-10-25/`) | done; measured layer cost (held Shift), typed-code corpus, corne-36 against Getreuer, Miryoku, US-style |
| 2f | The best layout for a Vim programmer (experiment report) | [vim-layout/vim-layout.md](vim-layout/vim-layout.md) (PDF beside it; data `bench/vim-layout-2026-10-25/`) | done; h j k l policies, Esc placement and Vim share on corne-36 |
| 2g | What layout designers value: inverse optimization (experiment report) | [inverse-fit/inverse-fit.md](inverse-fit/inverse-fit.md) (PDF beside it; data `bench/inverse-fit-2026-10-25/`) | done; per-layout fitted weights for 17 layouts, cross-scoring, inward rolls in timing and comfort data |
| 2h | Symbol layers for programming and Vim (experiment report) | [symbol-layer/symbol-layer.md](symbol-layer/symbol-layer.md) (PDF beside it; data `bench/symbol-layer-2026-10-25/`) | done; eleven published Sym layers vs the optimum on corne-36, human rules (`brackets`, `sym_groups`), `< >` and digit placement |
| 4 | Software paper: how GECKO works | [software/software.tex](software/software.tex) (PDF: `software/software.pdf`; notes: [software-paper.md](software-paper.md)) | typeset draft, 11 pages, eight use cases; written as the finished system; planned results are `TODO(run)` + template figures |

Other files: [topics.md](topics.md) (all candidate topics; the four above are the focus),
[drafts/later/](drafts/later/) (parked: geometry co-design), [templates/](templates/) (LaTeX/HTML
style, see below).

## Writing and building PDFs

Styling is in `templates/gecko-paper.cls`; a paper is content only (`templates/skeleton.tex`).
Build: `docs/papers/templates/build_pdf.sh path/to/paper.tex`. Details: `templates/README.md`.
Draft PDFs: `python3 -I docs/papers/drafts/build_drafts.py` turns each `drafts/*.md` into a `.tex` and `.pdf`
beside it (needs pandoc and XeLaTeX); edit the `.md`, never the generated `.tex`.
Drafts are Markdown first; convert a draft to `.tex` when its results exist. Each paper lives in its
own folder (`software/`) with `<name>.tex`, `figures.py` (data from the repo, never synthetic), `assets/`.

## Rules for every draft

1. **No result without a run.** A number in a draft is either (a) copied from a run directory or
   `bench/` file and cited by run id, or (b) marked `TODO(run)` with the exact command that will
   produce it. Planned figures are described, never faked: a figure for a result that does not
   exist yet is a watermarked template (illustrative shapes, "Template." caption, `TODO(run)`
   command), never a plot of invented numbers. Expectations are written as
   **hypotheses** and kept apart from **results**.
2. **Reproduce from the repo.** Every table and figure lists its command, seed, corpus, profile,
   geometry and commit. `gecko verify run:<id>` must pass for runs cited.
3. **Negative results stay in.** Several earlier findings are modest (for example the learned
   profile gains 1.8 points of pairwise accuracy). Say so; it is what makes the strong claims
   credible.
4. **One home per fact.** Methods that exist in code are described in `context/`; the paper
   summarizes and links. Evidence for design choices lives in `research/`.
5. Figures are generated by a script in `docs/papers/figures/` from run data (not hand-drawn),
   using the web UI's colour tokens (see `templates/generate_figures.py`).

## Suggested order

1. **Software paper** (nothing blocks it; it describes what exists).
2. **Landscape** (only needs `explore`, `atlas`, `qd`; runs are cheap).
3. **Best layout** (budget grid now; exact windows after #43).
4. **Cost function** (sweeps now; timing model #31 and a user study make it strong). It is also the
   paper that sets how far the other two papers' claims can go.
