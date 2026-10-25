# Generalization and overfitting of English keyboard corpora

## What does the existing evidence establish?

### Takeaway
The corpora are sufficient to show that GECKO finds layouts which beat reference layouts across the tested distributions under a fixed score. They do not establish an optimal keyboard for general English, a complete keyboard workload, or robust ranking of near-tied winners across unseen genres. Shai is the best-supported provisional training corpus for the 30-key letter-layout problem, not a final universal-English standard.

### Cited Findings
- Phase 2 evaluated 30 seeds on each of two boards, English and Shai, with matching symbol sets. The worst seed beat same-set reference layouts by approximately 5–16%; the optimized layout also beat the reference on the opposite corpus. These are substantial transfer margins under the chosen score, not merely a training-set win. [Evaluation, lines 19–38 and 85–103](../../../context/evaluation.md#L19)

- From the rounded holdout table, the improvement versus best same-set reference is ANSI English→Shai: 6.38%; ANSI Shai→English: 12.13%; Corne English→Shai: 9.69%; Corne Shai→English: 9.87%. These percentages describe model score reduction, not speed or comfort improvement. [Holdout table](../../../context/evaluation.md#L85)
- On ANSI, English-trained winner scores 13.273 on Shai versus reference 14.178; Shai-trained winner scores 12.818 on English versus reference 14.587. On Corne the corresponding values are 10.546 versus 11.678 and 10.588 versus 11.748. [Evaluation holdout table](../../../context/evaluation.md#L85)
- Existing saved five-corpus comparison has six candidate layouts, including the two winners. Shai winner is first or second in every cell, with mean ranks 1.4/1.6; English winner is fifth of six on ANSI MT-450k and fourth on Corne. [Actual score table](../../../bench/phase2-2026-10-23/corpora.txt#L1)
- Important correction: the prose says English winner is first on Reddit, but on ANSI Shai is first (13.60897), English second (13.97576). English is first on Corne Reddit (10.721 vs 11.086). Thus the actual evidence slightly strengthens Shai's provisional standing. [ANSI JSON](../../../bench/phase2-2026-10-23/corpora-ansi-30.json); [table lines 3–4, 11–12](../../../bench/phase2-2026-10-23/corpora.txt#L3); [incorrect prose](../../../context/evaluation.md#L99)
- Five corpus names are not five independent English domains. English and English-cased are the same ten novels; MT-450k weights words once rather than by natural use. [Representation](../../../context/representation.md#L198)
- The Phase 2 experiment pins Corne thumbs and optimizes the same 30 symbols as the references. It does not evaluate the freedom to place letters, Space, modifiers and symbols across a complete keyboard. [Run script](../../../bench/phase2-2026-10-23/run.py#L5)
- No whole-board optimality certificate exists. Seed convergence is search evidence, whereas the exact certificates fix the other keys and free only 18 letters. [Bounds](../../../context/evaluation.md#L105)
- Previous weight sensitivity generated 73 distinct optima from 150 moderate weight draws; the results describe the old 2026-10-21 profile and must not be presented as a newly measured result for the current profile. [Cost validity](../../experiments/2026-10-22-cost-function-validity.md#L34)

### Inferences
- Keep the positive conclusion: transfer is real evidence, and redundancy in the five-corpus table does not invalidate it. However, an average rank over this list double-weights the novels and gives a vocabulary stress distribution equal importance to actual prose. It is not a justified estimate of the average English user's workload.
- A poor rank on MT-450k primarily demonstrates sensitivity to a different target distribution. Calling it proof of harmful ordinary-English overfitting is too strong: rare words really are rare in ordinary writing. Use MT-450k as a stress test, not a coequal main objective.
- The existing cross-corpus result justifies starting from Shai for letter positions, while using Reddit and novels as diagnostics. It does not establish their raw sources are disjoint, nor that Shai's imported word counts adequately represent sentence/space/modifier behavior.
- Convergence across optimizer seeds measures optimizer stability; it says nothing directly about sampling uncertainty or missing domains. More seeds cannot substitute for new text sources.

### Gaps
- No held-out source/document/domain confidence analysis was identified in these artifacts; aggregate tables cannot reveal author/site concentration or duplicates.
- The six-layout broad-corpus table cannot establish best-in-community ranking on every corpus; its stronger whole-community comparison is restricted to the cells and symbol sets in Phase 2.
- General English has no defined unique genre weighting here. A literal universal optimum is undefined until workload, geometry, symbol coverage and scoring preferences are fixed.

## How should overfitting and fair scoring be handled?

### Takeaway
Use source-separated, genre-stratified development and test data, with genuine full text for complete keyboard evaluation. Report cross-domain score margins and uncertainty, not just rank or a percentage of the gain from QWERTY.

### Cited Findings
- The scorer extracts only the layout's characters and then normalizes each mono/bi/skip/tri table separately. Missing symbols therefore disappear before normalization. [Scorer build](../../../crates/gecko-core/src/scorer/build.rs#L35)
- Corpus mixing separately normalizes every table before applying corpus weights, requires identical charsets, and preserves Shift-to-mono ratios. It is not raw-text concatenation proportional to source size. [Mix implementation](../../../crates/gecko-core/src/corpus.rs#L788)
- Default trigram coverage is 0.99. Each scorer drops the least frequent trigrams and renormalizes the retained table. Final full-coverage sensitivity is therefore a distinct test from merely using the search's normal fidelity. [Profile default](../../../crates/gecko-core/src/profile.rs#L185); [build lines 88–89](../../../crates/gecko-core/src/scorer/build.rs#L88); [truncate](../../../crates/gecko-core/src/scorer/build.rs#L678)
- The holdout tool measures `(holdout start - holdout best)/(training start - training best)` and top-K rank correlation. The CLI prose says near 100% means no fitting to the search corpus; this is stronger than the statistic establishes. [CLI](../../../context/cli.md#L227); [implementation](../../../crates/gecko-core/src/run.rs#L307)
- Finite-sample model-selection criteria can themselves be overfit, creating performance-evaluation selection bias. [Cawley and Talbot, JMLR 2010](https://www.jmlr.org/beta/papers/v11/cawley10a.html)
- Official cross-validation guidance separates model selection from final testing, and recommends group-aware splitting when observations share a group dependence. [scikit-learn cross-validation documentation](https://scikit-learn.org/stable/modules/cross_validation.html)

### Inferences
Proposed protocol (recommendations, not already-completed measurements):
1. Define the primary estimand as expected modelled cost for contemporary English writing across explicitly stated domains. Keep code/Vim as future separate workloads. For now distinguish letter-only ranking from full-keyboard ranking.
2. Freeze common symbols, board, fingering, constraints, profile, tokenizer and evaluation coverage. Report excluded character/ngram mass. A winner that cannot type the same text is not a fair winner.
3. Split before aggregation by complete document and, where possible, author/site/thread/book. Deduplicate across splits and avoid concatenating unrelated documents. Random characters or random ngrams from the same documents mostly test finite-sample noise, not generalization to unseen sources.
4. Keep development sources for choosing corpus mixtures and candidates. Reserve fresh sources for the final test; repeatedly examined English/Shai/Reddit are useful validation benchmarks but cannot indefinitely remain untouched final tests. This is a methodological warning, not a claim proven leakage already occurred.
5. Stratify domains and expose mixture weights. Source byte size should not silently dictate the objective. An equal-domain mixture is a transparent initial baseline, not evidence that people type equal shares of each genre. Test plausible mixture perturbations; avoid treating several versions of the same novels as different domains.
6. Compare each candidate to the same fixed strong reference on each held-out domain, reporting raw score difference and raw metrics. Report rank as a supplement. Regret relative to the best *tested* candidate on a domain is useful but must not be labelled regret to the true optimum. Percentage regret can mislead for signed, near-zero scoring functions; use fixed scales/raw margins.
7. Bootstrap whole source groups to estimate paired score-difference uncertainty, preserving domains. Use sample-size curves and leave-one-domain-out evaluations. These rescore frozen layouts; they need not repeat optimization for every resample. They estimate uncertainty conditional on chosen candidates, not full selection-process uncertainty.
8. Check full trigram coverage and preprocessing alternatives on a small frozen finalist pool. Keep synthetic edits separately labelled and vary their assumed rate rather than claiming observational generalization.
9. Prefer a candidate with consistently small losses over one with a narrow aggregate win and a large loss in a major target domain. Choose the allowable loss tolerance before the final test; the project has no empirically validated universal numerical threshold.

### Gaps
- A corpus mixture's effect after layout-character filtering can differ from its advertised input weights because each table is conditioned on covered characters. Full coverage and per-domain evaluation are needed before interpreting a mixture as a literal workload average.
- Even near-100% retained gain from a poor starting layout can coexist with important overfitting among excellent candidates. The large QWERTY-to-modern improvement can mask small but consequential modern-to-modern differences.
- Corpus robustness establishes robustness under the score, not that the score measures comfort or speed correctly; those are distinct uncertainty sources.

## What should be done now, later, and when is it enough?

### Takeaway
The corpus set is enough to continue developing and finding strong English letter layouts. It is not yet enough to stop corpus work or claim a general English complete-keyboard optimum. Improve breadth, provenance and preserved typing information before seeking vastly more tokens.

### Cited Findings
- The primary full-character and edit-aware corpora remain transformations of the same ten Gutenberg novels. [Corpus inventory](../../../context/representation.md#L198)
- Word imports estimate cross-word skipgrams using independent consecutive words, and old case-free corpora do not contain observed Shift usage. [Import semantics](../../../context/representation.md#L185); [case caveat](../../../context/representation.md#L214)
- Existing CLI can already mix corpora, score candidates across corpora, and record held-out scores, so staged validation can reuse much of the current infrastructure. [CLI](../../../context/cli.md#L12)

### Inferences
**Now, low CPU:** preserve existing corpora for historical replay; record source manifests/checksums and semantic limitations; fix the narrow Reddit documentation error; design source-group splits and a small independent English test suite. Reuse saved finalists and score tables. Treat Shai as provisional letter-layout training, novels/Reddit as validation, MT lists as stress tests. Do not casually mix case-free Shai into a Shift-aware workload and interpret zero counts as observed zero capital use.

**Next data step:** acquire modest, reproducible samples across multiple contemporary English writing domains, preserving sentence order, capitalization, punctuation, paragraph boundaries and document identity. Conversational writing, expository/professional writing, and narrative text offer different coverage. Use each domain's train/development/test partitions consistently. Specific source choices are delegated to the source researcher.

**When CPU is free:** score existing finalists by held-out source/domain; analyze per-group paired differences and learning curves; then optimize a small set of defensible mixtures, compare to Shai alone, and evaluate frozen finalists on the fresh test. Only add substantially more data where marginal data changes a decision or uncertainty remains material.

**Enough-data stopping rule:** no magic token count. The set is adequate for the stated target when more independent sources and larger source-group samples stop materially changing held-out candidate differences, candidate choice remains acceptable under plausible domain mixtures, full-text character/action coverage is complete, and preprocessing/synthetic-count sensitivity is smaller than the claimed advantage. If several layouts remain indistinguishable, report a robust near-optimal set. All thresholds are engineering choices to predeclare, not empirical facts currently established.

**Narrow documentation correction proposed:** in `context/evaluation.md:99`, replace “The english winner is 1st on english, english-cased and reddit” with “The english winner is 1st on english and english-cased on both boards; on reddit it is 2nd on ansi-30 and 1st on corne-36.” Consider replacing “winner over-fits that difference” at line 102 with “winner transfers less well to this word-list distribution” to avoid implying MT-450k is the ground truth for everyday English.

### Gaps
No new optimizations, corpus downloads or builds were run for these notes. The judgement uses the current checked-in implementation and already-produced results, which predate the small scorer tie-breaking correction documented in Phase 2. The reported margins are historical results, not newly reproduced scores.
