# English corpus provenance and practical improvements

Research date: 2026-10-24. Read-only upstream checks; no corpus downloads, builds, or optimization. Recommendations below are prospective judgments, not experimentally established optimal mixtures.

## What do the upstream sources establish about the current corpora?

### Takeaway
Shai is a useful community reference with natural frequency weighting, but GECKO's aggregate is not the entire iWeb corpus and the exact transformation chain remains insufficiently documented. Adding more similarly derived aggregates would not automatically add independent evidence.

### Cited Findings
- GECKO documents `shai` as imported cmini `words.json`, reports roughly 520M characters, and documents that word-import reconstruction estimates cross-word transitions; this is distinct from the raw source size. The same document identifies the Gutenberg family as variants of ten novels. — [GECKO representation](../../context/representation.md)
- The primary iWeb publisher describes 14 billion words from 22 million web pages and about 95,000 websites. Its corpus catalog dates iWeb to 2017 and distinguishes web text from balanced COCA. — [iWeb description](https://www.english-corpora.org/iweb/help/iweb_name.asp), [publisher catalog](https://www.english-corpora.org/corpora.asp?b=y)
- The developer of Keycraft identifies its own default corpus as Shai's cleaned iWeb, about 90M words, linking the Colemak-hosted `iweb-corpus-samples-cleaned.txt.xz`. This is primary evidence about Keycraft's source, not proof that cmini's exact artifact is byte-identical or identically filtered. — [Keycraft README](https://github.com/rbscholtus/keycraft), [linked source](https://colemak.com/pub/corpus/iweb-corpus-samples-cleaned.txt.xz)
- The cmini source directory contains monograms, bigrams, trigrams, and words JSON, with `words.json` about 38MB. API inspection found no corpus-specific README, and its file history listed an addition under commit `1370aba5e763229fc4c1bfbfce211c44489d8079` with message “Add caching layer.” Its root README only describes a layout-statistics bot. These do not document the original source sample, filtering, dates, or genres. — [directory](https://github.com/Apsu/cmini/tree/master/corpora/shai), [history endpoint inspected](https://api.github.com/repos/Apsu/cmini/commits?path=corpora/shai/words.json&per_page=5), [root README](https://raw.githubusercontent.com/Apsu/cmini/master/README.md)
- The iWeb publisher distinguishes its freely searchable interface from purchasable downloadable full data. Do not treat the entire raw corpus as an unrestricted free dataset. — [publisher catalog](https://www.english-corpora.org/corpora.asp?b=y), [full-text data](https://www.corpusdata.org/corpora.asp)

### Inferences
- Shai remains a reasonable provisional main letter-frequency training source; its scale is unlikely to be the first bottleneck. Coverage, provenance, missing sequence information, and external validation are more immediate concerns.
- A 520M-character aggregate is not 520M words, independent documents, or a representative survey of English typing. Distinguish each unit in reports.
- The independent test requirement cannot be satisfied merely by using an alternative analyzer's shai/iWeb-derived tables.
- Ten novels and several rebuilds of those novels constitute one source family. Large character counts do not remove author, era, genre, and formatting bias.

### Gaps
- Exact cmini shai source file, extraction version, capitalization/punctuation filtering, sample composition, and deduplication were not established from primary documentation. The Colemak corpus page/directory was inaccessible in the web tool. No archive was downloaded to work around this.
- cmini Reddit sampling dates, communities, authors, bot filtering, and source text provenance remain unverified. “Reddit” alone is not a sufficient domain description.
- No measured distribution of what the intended user population actually types is available. Published web/novel text is a proxy for intended output, not an observed stream of editing, corrections, or shortcuts.

## Which modest external additions are most useful?

### Takeaway
Start with small, documented raw-text genre slices rather than a giant crawl. MASC is the most convenient initial independent screening suite; selected OANC texts can expand it, while a dated Wikipedia or Leipzig sample adds a distinct informational-English check.

### Cited Findings
- MASC offers approximately 500,000 words of written and spoken American English under CC BY 3.0 US, with downloadable data. Its 19 genres include email, letters, blogs, newspapers, fiction, technical writing, Twitter, and speech transcripts. Some genre slices are only about 25–30K words. — [MASC overview](https://anc.org/data/masc/), [genre table](https://anc.org/data/masc/corpus/), [download page](https://anc.org/data/masc/downloads/data-download/)
- MASC is drawn primarily from OANC, so treating all OANC as training and MASC as independent testing would risk overlap. — [MASC corpus description](https://anc.org/data/masc/corpus/)
- OANC is about 15M words, with unrestricted use and redistribution per its publisher. Its source table includes letters, travel guides, journal articles, government text, telephone and face-to-face transcripts. The published composition is dominated by several large components rather than equal genres. Its download page warns that the full annotated distribution expands to about 4.8GB; only selected text files are needed for GECKO. — [OANC overview](https://anc.org/data/oanc/), [contents](https://anc.org/data/oanc/contents/), [downloads](https://anc.org/data/oanc/download/)
- OANC's texts are since 1990, with, for example, Slate articles from 1996–2000. These improve on Victorian fiction but do not establish contemporary 2026 correspondence. — [MASC/OANC description](https://anc.org/data/masc/corpus/), [OANC contents](https://anc.org/data/oanc/contents/)
- Leipzig offers corpora classified by language, genre, and acquisition year, with sizes including 10K, 30K, and 100K sentences. Its data format includes sentence and source identifiers. However, original documents are split into shuffled sentences and deleted, so document order is unavailable. — [documentation](https://wortschatz.uni-leipzig.de/en/documentation), [format specification](https://www.wortschatz.uni-leipzig.de/documents/Format_Download_File-eng.pdf), [FAQ](https://www.wortschatz.uni-leipzig.de/en/documentation/faq)
- Wikimedia describes its text licensing and attribution/reuse requirements under CC BY-SA 4.0, with exceptions where indicated. A small version-pinned English Wikipedia article sample can retain article IDs and provenance. — [Wikimedia terms](https://foundation.wikimedia.org/wiki/Terms_of_Use?useformat=mobile)
- An existing keyboard-specific alternative, Granite English, mixes 40% Leipzig and 60% Reddit TLDR17. This is useful evidence that mixed-source character tables exist, but it would not automatically provide independent validation against Reddit or iWeb-adjacent web data. Its stated mixture is the author's design decision, not an empirically representative English-typing distribution. — [Granite English repository](https://github.com/fohrloop/granite-english-ngrams)

### Inferences
- Recommended order: (1) MASC selected email/letters/blog/informational slices as a cheap screening evaluation; (2) source-separated OANC slices if screening reveals material variation; (3) a version-pinned contemporary informational source, preferably raw article text with paragraph boundaries. Keep Gutenberg as a historical-fiction stress case, shai as a legacy baseline, and Monkeytype lists as vocabulary stress tests.
- Do not combine MASC's equal-sized categories blindly: spam, court transcripts, scripts, and biomedical writing need not receive the same weight as everyday messages and prose. “Balanced by genre” is not “representative of keyboard use.”
- Speech transcripts can probe conversational vocabulary, but transcription marks, disfluencies, and editor-inserted punctuation are not ordinary typed messages. Use them as a secondary stress test, not the centerpiece of an English typing corpus.
- Leipzig is useful for within-sentence letter, punctuation, and space sequences; reset state between independently shuffled sentences. Never turn each stored sentence into an observed Enter key or pretend the file has natural paragraph order.
- Wikipedia gives informational prose and varied subject vocabulary, but topic lists, references, URLs, markup, and templates require separate treatment. It is a supplement, not a general-English solution.
- Prefer raw sequences for new data so that case, spaces, punctuation, paragraphs, and cross-word context remain measurable. Keep every transformation versioned and record observed vs inferred keystrokes.

### Gaps
- These are verified candidate sources, not newly ingested or scored corpora. No finding here establishes their resulting layout rankings.
- Leipzig's terms page returned an anti-bot challenge, so exact reuse permissions for a selected release must be checked before adoption. Its catalog page access also failed in the web tool. Do not label it uniformly CC BY or otherwise freely redistributable without checking the release.
- MASC/OANC is American and not particularly recent. A British/international and genuinely contemporary slice remains desirable if the goal covers broad English rather than specifically American English.

## How should the new sources be used to avoid overfitting?

### Takeaway
Define the target workload and preserve source-level holdouts. Confidence should come from low performance loss across genuinely distinct domains, rather than a single best score on an ever-growing pooled corpus.

### Cited Findings
- GECKO's Phase 2 evaluation reports broad gains for shai-trained layouts but weaker transfer from the novel-trained winner to the mt-450k word list; it explicitly limits its conclusion to the chosen profile and corpora. — [Phase 2 evaluation](../../context/evaluation.md)
- The current representation normalizes over layout-covered symbols and notes that scores with different symbol coverage are not directly comparable. — [representation](../../context/representation.md)

### Inferences
- Treat “general English” provisionally as ordinary written communication plus prose/informational writing, with explicit American/British sensitivity. Code and Vim command streams should later be distinct workloads, not silently mixed into this study.
- Select a small training mixture by declared policy, not raw dataset byte size. Equal high-level strata can be a neutral starting experiment, but no source justifies claiming equal strata or any proposed percentage is the true optimal mixture.
- Preserve case and punctuation in all new raw-text sources. Evaluate the letter core and full-keyboard workload separately with identical target coverage; otherwise lower scores can reward omissions.
- Split by document and, where possible, author/site/thread/book before aggregation. Keep duplicated or quoted passages in the same split. Reserve an untouched source family for final evaluation; once inspected repeatedly to choose weights or a winner it is validation data, not a pristine test.
- For existing aggregate-only corpora, retain them as domain checks. Randomly dividing n-gram counts does not create a meaningful out-of-domain holdout and cannot reconstruct document-level uncertainty.
- For a short list of saved layouts, report relative degradation/regret per source family, plus key-position stability. Compare with the strongest fixed reference on the same input and symbol set. Use reasonable mixture perturbations to test whether a small score edge requires a narrowly chosen weighting.
- Establish sufficiency empirically later: increasing independent documents or adding another heldout source should not materially change the recommended tier of layouts or reveal unacceptable domain regressions. Choose the practical tolerance before selecting a final winner; there is no universal number of characters that proves adequacy.
- Bootstrap whole documents or source clusters, not individual keystrokes. A 500K-word balanced sample can reveal gross domain weaknesses but does not on its own certify tiny ranking margins, especially in its smaller genre slices.

### Gaps
- No new scoring, resampling, learning curve, or optimization was run in this research. The current saturated machine should first be used only for provenance and design work, with later evaluation bounded to a small candidate set before launching new searches.
- Corpus robustness only validates robustness under GECKO's score. It does not resolve whether its cost model predicts human comfort or speed; that is a separate uncertainty.
