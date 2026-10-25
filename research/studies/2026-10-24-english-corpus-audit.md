# English corpus fitness audit

Date: 2026-10-24. Status: audit and evaluation design complete; corpus repairs,
new source ingestion, and new scoring/optimization have **not** been performed. Since then
(commit `10991c1`): `english-cased`, `english-full(-bspc)`, `mt-10k` and `mt-450k` are retired to
`data/corpora/legacy/`, and languages are recipes over new components (`writer`, `programmer`,
`vim-programmer`; `context/representation.md#corpus`).

The existing corpora support strong English **letter-layout candidates**, but do not yet
establish a general-English complete-keyboard optimum. Keep Shai as the provisional letter
baseline, novels and Reddit as development checks, and Monkeytype lists as vocabulary stress
tests. Improve source diversity, provenance and preserved typing information before adding
large volumes of data.

Full analysis: [English corpus fitness](2026-10-24-english-corpus-audit/report.md).
Evidence: [artifact audit](2026-10-24-english-corpus-audit/corpus_audit.md),
[generalization](2026-10-24-english-corpus-audit/overfitting.md),
[external sources](2026-10-24-english-corpus-audit/sources.md),
[counting script](2026-10-24-english-corpus-audit/audit_counts.py),
[measured counts](2026-10-24-english-corpus-audit/counts.json).

## Findings

- The four novel variants share ten books. `english` and `english-cased` have identical
  letter-only mono/bi/tri/skip-1 distributions; they are not independent validation domains.
- Existing opposite-corpus score gains are 6.38–12.13% against same-set references, under the
  profile used in [Phase 2](../../context/evaluation.md). These validate transfer within the
  tested letter-layout problem, not human performance or complete keyboards.
- Word imports retain useful frequencies but approximate cross-word sequences and lack case.
  Shai/Reddit source sampling and overlap remain insufficiently documented.
- Curly double quotes are dropped by the tokenizer. The novel bodies contain 33,984 such
  characters and no ASCII double quotes; `english-full` consequently has zero ASCII quotes.
- `english-full-bspc` materially distorts unrelated frequencies (Space 17.75% → 12.37%, slash
  approximately 394-fold higher). The edit sampler draws fresh RNG values inside the
  binary-search predicate, violating its partition assumption; navigation has the same defect.
  Exact causal contribution was not isolated with a fixed-build experiment. The entire synthetic
  stream also dilutes case and replaces higher-order structure. Preserve the old artifact for
  replay, but set it aside for new full-English conclusions pending repair and validation.

## Recommended next work

Repair/version Unicode normalization and synthetic sampling/case handling first. Add a modest
raw-text suite with document IDs, case, punctuation and paragraph boundaries: MASC written
slices offer a practical initial screen; check OANC overlap and add contemporary informational
text separately. No proposed source mixture is established as the true distribution of typing.

Split before aggregation by document and source groups, deduplicate across splits, and retain
fresh final test sources. Once CPU is available, rescore saved finalists before launching new
searches. Compare equal symbol coverage, per-domain margins, source-group uncertainty, plausible
mixtures and full trigram coverage. Stop adding data when independent-source expansion no longer
changes the practical recommendation and all material domain losses remain acceptable under a
predeclared tolerance. Report a near-optimal set if uncertainty exceeds the differences.

No builds or heavy computations were run. The audit parser used only small compiled tables;
existing corpus and run artifacts were unchanged. The Phase 2 Reddit rank description was
corrected against its saved table; new measurements do not replace the historical run scores.
