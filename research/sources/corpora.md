# Corpora and n-gram data used by the community

Extends `../data-sources.md` with what the analyzers actually ship.

| Corpus | Where | Notes |
|---|---|---|
| shai (iweb-based English) | Oxeylyzer `static/language_data/shai.json`; cmini `corpora/shai`; genkey default `shai-iweb` | De facto community default |
| Monkeytype word lists 200 / 1k / 5k / 10k / 450k | cmini `corpora/english-*` | Short common words; 450k stresses rare bigrams |
| monkeyracer, mt-quotes, tr-quotes, keymash, reddit, movie, wordle | cmini `corpora/*` (mono/bi/tri/words JSON) | Typing-test and chat-like text |
| akl | cmini `corpora/akl` | Community-assembled mix |
| Wikipedia, Google Books (since 1950) | Kühlmak `corpus/`, scripts `process_wiki.sh`, `dl_gbooks.sh` | Google Books 1-grams ≈ 3 GB (German example) |
| Google Books n-gram letter pairs/triples (normalized) | Engram `optimize_layouts/input/frequency/*.csv` | Used for Engram-en |
| Leipzig corpora (many languages) | https://wortschatz.uni-leipzig.de | Used for Engram-es |
| French, Dutch, other languages | cmini `corpora/` | |

## Notes

- In GECKO: `shai`, `mt-10k`, `mt-450k`, `reddit`, `french` (`--fold`) are imported from cmini with `gecko corpus import`
  (see `context/representation.md`). cmini's `words.json` is exactly the source of its
  mono/bi/trigram tables (totals match), and its n-grams never span words.

- Results depend on corpus (Gallium 10k vs 450k). GECKO's bundled corpus is Gutenberg novels;
  add shai or a Monkeytype list so SFB numbers line up with what users see in cmini.
- Skipgram definitions differ: genkey counts skipgrams up to gap 10 with `1/2^g` weights and
  only across valid chars; GECKO counts `x _ z` with any middle char. Document which one each
  reported number uses.
- Licenses vary (cmini repo GPL, Oxeylyzer Apache-2.0). N-gram counts are generally reusable
  with attribution; keep raw text out of git.
