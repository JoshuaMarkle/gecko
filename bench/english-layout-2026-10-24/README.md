# The best 30-key layout for English, 2026-10-24

Write-up: `docs/papers/english-layout/` (`.md` + PDF). GECKO at the commit that adds this
directory, profile `default`, ansi-30, BLS 25 M evaluations, 4 threads.

| file | what |
|---|---|
| `seeds.py` | seeds × BLS on one corpus and symbol set (`qwerty` `, . / ;`, `apos` `' , . /`, `semi` `' , . ;`) |
| `grid.sh` | `prose` × 3 sets × 20 seeds (holdout `shai`); the `apos` optimum of every other English corpus × 10 seeds (holdout `prose`) |
| `post.sh` | `gecko sweep` from the prose optimum (40 draws + drop/half/double of each weight, SA 10 M), `gecko certify --free 18 --kind full`, then `gather.py` |
| `gather.py` | scores every optimum and 17 named layouts on 7 corpora and 4 profiles, robust rank (2,000 draws), held-out timing judge J_B → `data.json`, `gather.txt` |
| `seeds-*.json`, `sweep.json`, `certify-full.json` | raw results |

Rerun: `./grid.sh 2> grid.log && ./post.sh` (about 40 min on 20 cores). J_B needs the costfit
workspace (`bench/costfit-2026-10-24/README.md`); export the corpora it reads with
`costfit corpus -c <name> > data/corpus_<name>.json`.
