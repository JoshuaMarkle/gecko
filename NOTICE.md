# Third-party data

GECKO's source code is MIT licensed ([LICENSE](LICENSE)). Some bundled data files come from
other projects and keep their own licenses. Only aggregated counts are committed, never raw text.

| Files | Derived from | License |
|---|---|---|
| `data/corpora/{shai,mt-10k,mt-450k,reddit,french}.gkc`, `data/corpora/{shai,mt-10k}.words.tsv` | [cmini](https://github.com/Apsu/cmini) corpora (shai: iweb-based English; mt-*: [Monkeytype](https://github.com/monkeytypegame/monkeytype) word lists; reddit comments; french) | GPL-3.0 |
| `data/community/cmini.json` | [cmini](https://github.com/Apsu/cmini) layout collection | GPL-3.0 |
| `data/comfort/engram-*.csv` | [keyboard_layout_scorers](https://github.com/binarybottle/keyboard_layout_scorers) (Arno Klein) | MIT |
| `data/corpora/{english,english-cased}.gkc` | Ten Project Gutenberg novels | Public domain (US) |
| `data/corpora/{code,code-nav}.gkc` | Rust crates (MIT/Apache-2.0), CPython stdlib (PSF), hls.js and @use-gesture (Apache-2.0 / MIT) | Counts only; source licenses as listed |

If you redistribute the GPL-3.0 files, do so under GPL-3.0. GECKO runs without them: build
your own corpus with `gecko corpus build` (see [context/representation.md](context/representation.md)).
