# Current English corpus artifacts and preprocessing

## What data is actually present, and is it independent?

### Takeaway
The project has three useful natural-frequency English source families (ten novels, Shai, Reddit), plus two deliberately unnatural word-list stress tests. The several `english-*` variants are not independent corpora. Their size is sufficient to investigate common character patterns, but the artifacts do not establish representativeness of modern general English typing or held-out generalization by document/source.

### Cited Findings
- Audited the committed GKC files using a standalone stdlib binary parser, without running GECKO or rebuilding. Reproduction: `nice -n 19 python3 -I research/studies/2026-10-24-english-corpus-audit/audit_counts.py` (<1 s wall time here). Parser checks the entire file length and exports SHA-256, charset, each table's counts/nonzero cells, symbol frequencies, and letter-only total-variation distances. [Script](audit_counts.py), [results](counts.json), [binary format implementation](../../../crates/gecko-core/src/corpus.rs#L508).

| Artifact | Monogram total | Stored format | Space % | Shift starts per 100 tokens | Interpretation |
|---|---:|---|---:|---:|---|
| english | 7,485,869 | GKCORP2 | 18.230 | 0 | Frozen older-tokenizer novels |
| english-cased | 7,510,472 | GKCORP3 | 18.171 | 2.466 | Same novels, current key-folding tokenizer |
| english-full | 7,528,900 | GKCORP3 | 17.749 | 2.171 | Same novels, separate ASCII symbols, paragraph Enter |
| english-full-bspc | 999,999,999,938 | GKCORP3 | 12.373 | 1.324 | Synthetic normalized mixture; **not a trillion observations** |
| shai | 520,397,321 | GKCORP2 | 17.392 | 0 | Imported weighted words; documented iWeb-based |
| reddit | 55,633,851 | GKCORP2 | 18.190 | 0 | Imported weighted words; documented Reddit |
| mt-10k | 81,669 | GKCORP2 | 12.183 | 0 | Each listed word once |
| mt-450k | 4,775,076 | GKCORP2 | 9.455 | 0 | Each listed word once |

All numeric entries measured from [GKC results](counts.json); source descriptions from [representation](../../../context/representation.md#L194). Counts are normalized accepted tokens, not raw byte counts; imported word boundaries add one space per word. Skip counts for word imports are multiplied by 1,000; they must **not** be interpreted as independent observations. Mixture table masses are scaled by 1e12. [Word import](../../../crates/gecko-core/src/corpus.rs#L398), [mix](../../../crates/gecko-core/src/corpus.rs#L790).

- Local `data/corpora/src/` contains ten Gutenberg raw texts. Their header titles are Alice's Adventures in Wonderland; Jane Eyre; Pride and Prejudice; The Adventures of Sherlock Holmes; Moby Dick; Dracula; Ulysses; The Adventures of Tom Sawyer; Frankenstein; A Tale of Two Cities. After the same start/end marker trim, 7,612,271 Unicode characters remain; Ulysses + Moby Dick constitute 35.98% of that raw character volume. This is raw-body volume, not a computed share of GECKO normalized counts. [Inventory](raw_inventory.json), raw source headers at `data/corpora/src/pg*.txt:11`.
- CLI build adds each file's counts without book balancing. Thus large books receive more weight. Separate files are counted separately, avoiding artificial n-grams between books. [Build](../../../crates/gecko-cli/src/cmd/corpus.rs#L113).
- Source text is ignored by Git; only GKC and two word-sidecars are committed in this directory. Shai sidecar records 90,509,418 total word occurrences and retains at most 20,000 distinct folded words. It is not sufficient to reconstruct all source provenance or recover original word order. [Ignore](../../../.gitignore#L9), [Shai words](../../../data/corpora/shai.words.tsv#L1), [word-sidecar implementation](../../../crates/gecko-core/src/corpus.rs#L153).
- Actual letter-only `english` and `english-cased` mono, bi, tri, and skip-1 distributions are **exactly identical**, not merely similar. The case variant therefore adds no independent evidence about letter layout. [Measured distributions](counts.json).

### Inferences
- Ten literary works are a narrow domain regardless of millions of characters. Larger counts reduce count noise but do not correct author/style/era bias. Book dominance compounds this limitation.
- For optimizing the alphabetic core, Shai plus Reddit plus novels provides a meaningful preliminary suite. For a final full English keyboard, case, punctuation, document boundaries, and editing need more faithful data.
- Treat word-list results as stress tests of rare combinations, not equal votes defining English usage. Their low space fractions reflect longer uniformly weighted words, not evidence that real English needs less Space.

### Gaps
- No pinned source manifest with acquisition dates, source revision/hash, collection policy, deduplication policy, document IDs, and train/validation/test split was found alongside committed English corpus artifacts. Exact Shai/Reddit source sampling and overlap need upstream audit by the other researcher.
- No independent raw modern prose/chat corpus is available in the local source directory inspected. The imported histograms cannot supply document-level resampling or leakage checks.

## What information does preprocessing preserve or lose?

### Takeaway
The importer is quite useful for conventional letter n-grams, but word histograms cannot recover syntax-dependent cross-word sequences. There is also a concrete Unicode quotation omission in the raw-text tokenizer, relevant to the full English keyboard goal.

### Cited Findings
- In-word n-grams and each word's leading/trailing space n-grams are counted exactly relative to the supplied word histogram; `x space z` trigrams and cross-word skipgrams assume independently drawn consecutive words. Spans crossing three or more word segments are ignored. Thus imported Shai/Reddit are not faithful sequential text streams. [Boundary reconstruction](../../../crates/gecko-core/src/corpus.rs#L398), [independence approximation](../../../crates/gecko-core/src/corpus.rs#L468).
- Actual imported artifacts have skip-2 and skip-3 counts and skip tables at approximately 1,000x mono scale, consistent with word-import rather than n-gram-only fallback. The fallback leaves skip-2/3 empty. [Measured tables](counts.json), [fallback](../../../crates/gecko-core/src/corpus.rs#L433), [CLI import](../../../crates/gecko-cli/src/cmd/corpus.rs#L142).
- Tokenization collapses whitespace; lowercases ASCII capitals but records Shift-run starts; maps curly **single** quotes to apostrophe and en/em dashes to hyphen. ASCII shifted punctuation folds to US key bases only if the actual symbol is absent from charset. Unrecognized symbols break adjacent n-grams, while skipgrams may span them. [Tokenizer](../../../crates/gecko-core/src/corpus.rs#L283), [counting](../../../crates/gecko-core/src/corpus.rs#L350).
- **Curly double quotes `“”` are not folded.** They are not in the default or code charset and so become OTHER. The ten source bodies contain **33,984 curly double quotes and zero ASCII double quotes**. Correspondingly, the measured `english-full` ASCII `"` count is **zero**. This loses quotation key use and its adjacent sequences in these literary texts. [Tokenizer exact match arms](../../../crates/gecko-core/src/corpus.rs#L320), [raw inventory](raw_inventory.json), [GKC counts](counts.json).
- `english-full` has underscore 0.120536% and asterisk 0.012273%; source formatting cleanup only strips Gutenberg start/end headers and joins paragraphs. No semantic removal of italics/markup/front matter inside the body occurs in that pipeline. These symbols may partly represent editorial plaintext markup rather than intended English typing; quantify before treating them as natural usage. [Preprocessing](../../../crates/gecko-core/src/corpus.rs#L213), [measured symbols](counts.json).
- Shai and Reddit are not punctuation-free: apostrophe frequencies are 0.215539% and 0.614962% respectively. The imported artifacts do lack Shift tables; do **not** infer they lack contractions or all punctuation. [Measured symbols](counts.json).
- Documentation's line “only english-cased has case counts” is stale: full and full-bspc are GKCORP3 too. Case-run information is weaker than raw case sequence; keyed Shift-pair transformations use estimates where capitals meet shifted punctuation. [Stale documentation](../../../context/representation.md#L214), [keyed implementation](../../../crates/gecko-core/src/corpus.rs#L673), [measured format](counts.json).
- Scorer normalizes each table after selecting the layout's symbols. Missing punctuation therefore drops out rather than incurring a coverage penalty. Changing modeled symbols changes denominators and makes raw total scores incomparable. [Scorer](../../../crates/gecko-core/src/scorer/build.rs#L33).
- Mix is a weighted sum of each independently normalized table, requiring identical charsets. A 1:1 mix of Shai and novels does not let Shai's much larger count dominate; each gets equal table mass. Case is normalized by mono mass. If a component has a zero table, it contributes nothing to that order; weights can consequently mean different effective mixtures per metric. [Mix](../../../crates/gecko-core/src/corpus.rs#L790).

### Inferences
- Corpus improvement should first preserve natural frequencies and real sequences, normalize Unicode consciously, and record document metadata. Adding more aggregate words cannot repair missing sentence order or capitalization.
- Native full-symbol text should underpin modifier/thumb/layer recommendations; Shai remains useful for alphabetic positions and compatibility with community analysis.
- A shared complete task charset and explicit coverage report are prerequisites for claiming one whole keyboard is better than another.

### Gaps
- This audit did not rebuild corrected quotation corpora or measure how much any optimized layout would move. The defect's direction is clear; layout impact is unmeasured.
- Generic text captures produced characters, not actual editing or command sequences. An English text corpus alone cannot validate later Vim command use.

## How large are the differences, and is synthetic editing safe to use?

### Takeaway
Natural-frequency corpora agree much more on single letters than on letter sequences; the word-list distributions differ considerably more. More seriously, the present synthetic Backspace corpus has substantial unexpected distortions and a confirmed sampling-code defect, so its recommendations should not be used as evidence about English until corrected and rebuilt under a new version.

### Cited Findings
- Total variation distance, after retaining only a–z and independently normalizing each table:

| Pair | Mono TV | Bigram TV | Trigram TV | Skip-1 TV |
|---|---:|---:|---:|---:|
| english vs shai | 3.61% | 9.45% | 19.30% | 10.58% |
| english vs reddit | 4.29% | 9.29% | 20.65% | 11.35% |
| shai vs reddit | 2.99% | 8.06% | 15.55% | 8.35% |
| shai vs mt-10k | 8.48% | 17.12% | 24.24% | 16.33% |
| shai vs mt-450k | 9.68% | 20.46% | 34.42% | 19.66% |

Measured by [script](audit_counts.py) and [results](counts.json). TV = half the sum of absolute probability differences; it is redistributed probability mass, **not score regret, error bars, or fraction of an optimal layout that changes**. This conditions on letters; skip-1 still includes endpoints separated by spaces/punctuation. No raw-source split or bootstrapping performed.

- `with_edits` generates 2M synthetic tokens from a bigram Markov sampler, creates wrong-key runs then equal-length Backspace runs, and mixes the entire stream with original tables. It does not edit the actual source text. The generated stream has all Shift flags false; the original Shift counts therefore get diluted by the mixture weight. [Generator](../../../crates/gecko-core/src/corpus.rs#L734).
- **Code defect confirmed by inspection:** the cumulative-distribution sampling call draws `rng.f64()` **inside** the `partition_point` predicate, at each binary-search comparison. A correct inverse-CDF selection must draw a single threshold before searching. Here the predicate is not a consistent partition of the cumulative array, so it does not implement the intended categorical draw. `Rng::f64()` advances state via `next_u64()` on every call. Same defect exists in `with_nav`. [Edit draw](../../../crates/gecko-core/src/corpus.rs#L756), [nav draw](../../../crates/gecko-core/src/corpus.rs#L625), [RNG mutation](../../../crates/gecko-core/src/rng.rs#L49).
- **Observed artifact consequences, independently measured:** `english-full`→`english-full-bspc` Space share 17.748795%→12.372635%; slash 0.000930%→0.366126% (about 394x); all digit share 0.052635%→1.034989%; Shift starts 2.170755%→1.323861%. Backspace is exactly 5.6%. These changes cannot be described as simply adding 5.6% Backspace. [Measurements](counts.json).

### Inferences
- The wrong sampler is a strong likely cause of the severe synthetic distribution distortion, but no controlled corrected-build experiment was performed, so the exact portion attributable to that bug is unmeasured. Even after the bug is fixed, mixing a Markov stream changes genuine higher-order patterns and case; those modeling choices still need validation.
- **Priority recommendation:** preserve old hashes for run reproduction, quarantine full-bspc from new English claims, fix/test sampling and case treatment, regenerate under a new corpus version, and recheck symbol distributions before any expensive search.
- Do not let this full-keyboard synthetic issue obscure the independent usefulness of `english`, Shai, and Reddit for preliminary alphabet-only research. The distortion does not modify those files.
- An adequate next corpus suite should retain novels as one domain, Shai as an aggregate benchmark, Reddit as informal-domain evidence, word lists as stress cases, then add independently sourced contemporary sequential text with stable source splits. Treat corpus selection/tuning as training: already-consulted corpora are development evidence, not untouched final tests.

### Gaps
- No timing runs, optimizations, builds, or existing-file edits were performed; all work was read-only except new audit notes/results. The other agent's untracked `bench/costfit-2026-10-24/` was untouched.
- New corpus collection and domain mixture weights need a declared target (general English composition), and stopping criteria should be based on stable held-out layout regret rather than a fixed character-count target.
