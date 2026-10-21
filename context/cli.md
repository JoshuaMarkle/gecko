# CLI

One binary, `gecko` (`crates/gecko-cli`). Built for agents first: deterministic, scriptable, JSON output.

## Commands

```
gecko score    <layout> [-g geometry] [-p profile] [-c corpus]   # per-metric breakdown
gecko compare  [<layout>...] [--corpora a,b,c]                   # rank layouts (all references if none);
                                                                  #   --corpora: score/rank per corpus (L05)
gecko analyze  <layout> [--top 10] [--words]                      # finger loads, worst SFB/SFS/LSB/scissors; JSON adds all bigrams/skipgrams with a `kind`;
                                                                  #   --words: worst real words per metric (needs -c shai or mt-10k)
gecko optimize [--algo sa|hc|ga|bls|explore] [--time 30s | --evals N] [--seed 1] [--threads N]
               [--pin ",./;"] [--from <layout>] [--top 10] [--sa-steps N] [--ga-pop N]
gecko bench    [--algos hc,sa,ga,bls] [--seeds 3] [--time 5s]         # algorithm leaderboard
gecko explore  [--time 60s | --evals N] [--seed 1] [--threads N] [--pin ...] [--from <layout>]
               [--samples 2000] [--max-optima 100000]         # map local optima; writes an atlas run
gecko landscape [--samples 4000] [--walks 64] [--walk-len 400] [--seed 1] [--from <layout>]
                                                                  # random-score distribution, correlation
                                                                  #   length, reference percentiles, symmetry
gecko atlas    <run> [--top 10] [--cut 8] [--pool 300] [-p profile]
                                                                  # summary, ranked, families, consensus,
                                                                  #   funnels, FDC; -p re-ranks under a profile
gecko perf     [--layout qwerty]                                  # full evals/s and swap deltas/s (1 thread)
gecko corpus build <files or dirs...> [--out english] [--charset default|code|<symbols>] [--ext txt]
gecko corpus mix <corpus>[:weight]... --out <name>                     # weighted mix, same charset
gecko corpus import <cmini corpus dir> --out <name> [--charset ...]   # words.json or n-gram JSON; words.json
                                                                      #   also writes <name>.words.tsv
gecko profile normalize [-p profile] [--layouts a,b] [--out name]   # weights → spread units
gecko profile fit [--target E05] [--layouts a,b] [--out name]       # fit weights to a judge metric
gecko list     layouts|geometries|profiles|corpora|runs|metrics
gecko serve    [--port 7777] [--web web/dist]
```

Global flags: `--json`, `--root <dir with gecko.toml>`.

`<layout>` is a reference name (`data/layouts/<name>.toml`), a `.toml` path,
`run:<id>[#rank]`, or `raw:<slot-ordered symbols>` (e.g. `raw:qwertyuiopasdfghjkl;zxcvbnm,./`).
A 30-key layout on a board with thumb keys (`--geometry corne-36`) gets the geometry's thumb
fills. Tokens can be written `{space}`, `{enter}`, `{bspc}`, `{tab}`, `{esc}`, `{none}` in
`raw:` and `--pin`, e.g. `--pin '{space}{enter}{bspc}{tab}{esc}'` keeps the thumbs fixed.
Layered layouts: `--from layered/corne-code`; their rows print per layer (`[sym]`, `[num]`),
and JSON `rows` lists the base rows then each layer's. Runs store the flat layered string.

### `analyze --words`

Ranks the words of the corpus's word list (`data/corpora/<corpus>.words.tsv`) by frequency ×
cost for every lower-is-better metric with a cost table (load metrics and higher-is-better
metrics are skipped; so is any metric no word touches). A word is typed as `␣word␣` when `␣`
is on the layout, else bare; symbols not on the layout break n-grams; n-grams spanning two
words belong to neither. `--top` sets the words per metric. JSON adds a top-level field:

```
"worst_words": [ { "id": "B01", "name": "SFB", "unit": "%",
                   "words": [ { "word": "you're", "freq": 0.049, "cost": 1.0, "share": 2.2, "ngrams": ["'r"] } ] } ]
```

- `freq`: % of all word occurrences in the source (including words cut from the list).
- `cost`: per occurrence, summed over the word's n-grams: offending n-grams for `%` metrics
  (2 = two SFBs), distance for `u` metrics, raw effort for M01. Far SFS (S02) weighs gap 2/3
  by `skip_decay` / `skip_decay²`.
- `share`: % of the metric's total word cost (Σ freq × cost over the whole word list).
- `ngrams`: the word's n-grams with non-zero cost, in typing order.

Corpora without a word list (`english`, `code`, `mt-450k`, mixes) give an `error:`.

## Conventions

- `--json` on every command; stable schema with `"schema": 1`.
- Human output is compact tables. Progress goes to stderr, results to stdout.
- Exit code 0 on success, non-zero with a one-line `error:` on failure.
- `optimize` always writes `runs/<timestamp>-<algo>-<hash>/` and prints its id.
- `--evals` budgets are deterministic for a given seed and thread count; `--time` is not.
- Defaults (geometry, corpus, profile, reference layout, data/runs dirs) come from `gecko.toml`.
