---
group: community
kind: data
read: 2026-10-20
status: read
feeds: [L05, L06, corpus]
take: Which corpora and timing data exist; which to add for comparable numbers
---

# Corpora, n-gram data and timing data

What the analyzers ship, what else exists, and the rules for using it. GECKO's bundled
corpus is Gutenberg novels; the community default is `shai`.

## Text corpora

| Corpus | Where | Notes |
|---|---|---|
| shai (iweb-based English) | Oxeylyzer `static/language_data/shai.json`; cmini `corpora/shai`; genkey default `shai-iweb` | De facto community default |
| Monkeytype word lists 200 / 1k / 5k / 10k / 450k | cmini `corpora/english-*` | Short common words; 450k stresses rare bigrams |
| monkeyracer, mt-quotes, tr-quotes, keymash, reddit, movie, wordle | cmini `corpora/*` (mono/bi/tri/words JSON) | Typing-test and chat-like text |
| akl | cmini `corpora/akl` | Community-assembled mix |
| Wikipedia, Google Books (since 1950) | Kühlmak `corpus/`, `process_wiki.sh`, `dl_gbooks.sh` | Google Books 1-grams ≈ 3 GB (German example) |
| Google Books letter pairs/triples (normalized) | Engram `optimize_layouts/input/frequency/*.csv` | Used for Engram-en |
| Leipzig corpora (many languages) | https://wortschatz.uni-leipzig.de | Used for Engram-es |
| Project Gutenberg | GECKO bundled | Older language; robustness checks |
| OpenSubtitles / chat logs | not yet used | Closer to everyday typing |
| GitHub code per language | not yet used | Needed for symbol layers (Y02) and a code profile |
| User's own text | planned (#96) | Personal profile; keep local |
| French, Dutch, other languages | cmini `corpora/` | |

## Timing data

| Source | Use | Notes |
|---|---|---|
| Aalto 136M keystrokes ([summary](dhakal-136m-keystrokes.md)) | Fit an inter-key-interval model (E01, #31) | 168k typists, mostly QWERTY; free for research. Aalto host down; Kaggle mirror `noelsmathew/keystrokes` verified (bigram means within 3.5 % of the paper) |
| How We Type motion capture ([summary](feit-how-we-type.md)) | Validate hand-model reach postures | 30 participants; Zenodo |
| User keystroke logs | Personal timing model (E02, #56) | Needs a logger; keep local |

Timing data comes from QWERTY typists, so it reflects QWERTY habits. Fit **geometric** costs
(finger pair, distance, direction), never per-letter costs.

## Rules

- Results depend on corpus (Gallium: fine on 10k, worse on 450k). Report several corpora and
  their spread (L05).
- In GECKO, `shai`, `mt-10k`, `mt-450k`, `reddit` and `french` (`--fold`) are imported from
  cmini with `gecko corpus import` (`context/representation.md`). cmini's `words.json` is the
  exact source of its mono/bi/trigram tables, and its n-grams never span words.
- Skipgram definitions differ: genkey counts gaps up to 10 with `1/2^g` weights, only across
  valid characters; GECKO counts `x _ z` with any middle character. State which one a number uses.
- Licenses vary (cmini GPL, Oxeylyzer Apache-2.0). Counts are generally reusable with
  attribution; keep raw text out of git, commit compiled tables or a fetch script.
- Reference layouts live in `data/layouts/*.toml` with source links: QWERTY, Dvorak, Colemak,
  Colemak-DH, Workman, Halmak, Canary, Graphite, Sturdy, Semimak, plus cmini's ≈ 4100
  community layouts ([cmini](cmini.md)).
