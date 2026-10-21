# CLI

One binary, `gecko` (`crates/gecko-cli`). Built for agents first: deterministic, scriptable, JSON output.

## Commands

```
gecko score    <layout> [-g geometry] [-p profile] [-c corpus]   # per-metric breakdown
gecko compare  [<layout>...] [--corpora a,b,c]                   # rank layouts (all references if none);
                                                                  #   --corpora: score/rank per corpus (L05)
gecko analyze  <layout> [--top 10] [--words]                      # finger loads, worst SFB/SFS/LSB/scissors; JSON adds all bigrams/skipgrams with a `kind`;
                                                                  #   --words: worst real words per metric (needs -c shai or mt-10k)
gecko optimize [--algo sa|hc|ga|bls|tabu|explore|qd] [--time 30s | --evals N] [--seed 1] [--threads N]
               [--pin ",./;"] [--from <layout>] [--top 10] [--sa-steps N] [--ga-pop N]
               [--moves swap|columns,cycle3,depth2|all] [--depth2-width 32]
gecko bench    [--algos hc,sa,ga,bls,tabu] [--seeds 3] [--time 5s]         # algorithm leaderboard
               [--moves "swap/cycle3/depth2"] [--depth2-width N]      # one row per algo × variant
gecko explore  [--time 60s | --evals N] [--seed 1] [--threads N] [--pin ...] [--from <layout>]
               [--samples 2000] [--max-optima 100000]         # map local optima; writes an atlas run
gecko qd       [--time 60s | --evals N] [--seed 1] [--threads N] [--pin ...] [--from <layout>]
               [--axes roll_ratio,index_load] [--bins 16] [--seeds 64] [--descent 0]
               [--show <run>]                                     # MAP-Elites grid over behaviour axes
                                                                  #   (name or name:lo:hi); writes a qd run
gecko landscape [--samples 4000] [--walks 64] [--walk-len 400] [--seed 1] [--from <layout>]
                                                                  # random-score distribution, correlation
                                                                  #   length, reference percentiles, symmetry
gecko atlas    <run> [--top 10] [--cut 8] [--pool 300] [-p profile]
                                                                  # summary, ranked, families, consensus,
                                                                  #   funnels, FDC; -p re-ranks under a profile
gecko magic    <layout> [--key 1] [--on <symbol>] [--keep] [--max-rules 32] [--min-gain 0.01] [--out name|path.toml]
                                                                  # greedy rule search for an adaptive key
gecko verify   run:<id>                                           # re-run an --evals run from config.json;
                                                                  #   check input hashes, evals and best.json
gecko perf     [--layout qwerty] [--record]                       # full evals/s and swap deltas/s (1 thread); --record appends to bench/history.jsonl
gecko corpus build <files or dirs...> [--out english] [--charset default|code|nav|<symbols>] [--ext txt] [--fold]
gecko corpus mix <corpus>[:weight]... --out <name>                     # weighted mix, same charset
gecko corpus import <cmini corpus dir> --out <name> [--charset ...] [--fold]   # words.json or n-gram JSON; --fold: é→e; also writes <name>.words.tsv
gecko geometry import-kle <file.json> --out <name> [--force]       # KLE board → data/geometries/<name>.toml
                                                                  #   (fingers/rows/home/effort guessed; edit after)
gecko layouts import <file> [--from oxeylyzer|genkey|cmini|auto] [--out <name>] [--force]
               [--name N] [-g geometry]          # → layout TOML (stdout, or data/layouts/<out>.toml)
gecko layouts export <layout> --to oxeylyzer|genkey|cmini [-g geometry]   # 30 alpha keys → stdout
gecko layouts import-cmini <cmini checkout>                           # → data/community/cmini.json
gecko layouts likes [--top 10] [--dups] [-g -p -c]                     # GECKO rank vs cmini likes (Spearman)
gecko corpus nav <corpus> [--share 0.03] [--seed 1] --out <name>        # add modelled arrow keys
gecko profile normalize [-p profile] [--layouts a,b] [--out name]   # weights → spread units
gecko profile fit [--target E05] [--layouts a,b] [--out name]       # fit weights to a judge metric
gecko list     layouts|geometries|profiles|corpora|runs|metrics
gecko serve    [--port 7777] [--web web/dist]
```

Global flags: `--json`, `--root <dir with gecko.toml>`.

`<layout>` is a reference name (`data/layouts/<name>.toml`), a `.toml` path,
`run:<id>[#rank]`, `cmini:<name>` (imported community layout), or `raw:<slot-ordered symbols>` (e.g. `raw:qwertyuiopasdfghjkl;zxcvbnm,./`).
A 30-key layout on a board with thumb keys (`--geometry corne-36`) gets the geometry's thumb
fills. Tokens can be written `{space}`, `{enter}`, `{bspc}`, `{tab}`, `{esc}`, `{left}`, `{down}`,
`{up}`, `{right}`, `{none}` in
`raw:` and `--pin`, e.g. `--pin '{space}{enter}{bspc}{tab}{esc}'` keeps the thumbs fixed.
Layered layouts: `--from layered/corne-code`; their rows print per layer (`[sym]`, `[num]`;
non-hold layers add their mode, `[num, toggle]`),
and JSON `rows` lists the base rows then each layer's. Runs store the flat layered string.
Adaptive keys (`[[magic]]`, e.g. `magic/magic-sturdy`): `score`/`compare` rewrite the corpus for
their rules; human output prints `★ ao cy …` under the rows, `score --json` adds `magic`.
`optimize --from` a magic layout moves the key like any symbol with the rules fixed; the run's
`config.json` keeps `magic`, so `run:<id>` layouts carry their rules. `gecko magic` searches
rules on a fixed layout; `--on z` first turns that symbol's key into a new `★` (the symbol is
dropped). Alternate `magic --out` and `optimize --from` to co-optimize.

`layouts import` reads an Oxeylyzer `.dof` (JSON) or Oxeylyzer 1 grid (`.kb`), a genkey file
(name, 3 key rows, 3 finger rows) or a cmini JSON; `auto` guesses from the content. E.g.
`gecko layouts import --from oxeylyzer sturdy.dof --out sturdy-oxey`. Keys map by position onto
the 30 alpha keys; the geometry defaults to `ansi-30` (`ortho-30` for ortho/colstag sources).
Dropped keys and ignored fingering (angle mod) are reported as `warning:` lines on stderr and
in JSON `warnings`; a letter outside the 30 keys is an error.

### `analyze --words`

Ranks the words of the corpus's word list (`data/corpora/<corpus>.words.tsv`) by frequency ×
cost for every lower-is-better metric with a cost table (load metrics, paired-bracket placement and higher-is-better
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
- `verify` loads geometry and corpus by their recorded names (the profile is stored in
  `config.json`). If a hash differs it fails without re-running. Otherwise it re-runs with the
  recorded params into a temp dir, leaving the run untouched, and compares evals and every
  ranked layout: chars exact, score to 1e-9 relative. Exit code 1 when it does not
  reproduce; time-budget runs are an error.
- Defaults (geometry, corpus, profile, reference layout, data/runs dirs) come from `gecko.toml`.
