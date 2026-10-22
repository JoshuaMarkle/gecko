# CLI

One binary, `gecko` (`crates/gecko-cli`). Built for agents first: deterministic, scriptable, JSON output.
Code: `main.rs` (clap entry) dispatches to `cmd/<group>.rs` (score, optimize, bench, explore,
magic, corpus, geometry, layouts, profile, list); `ctx.rs` holds the shared inputs, loading
and output helpers, `fmt.rs` the text tables, `serve.rs` the HTTP server.

## Commands

```
gecko score    <layout> [-g geometry] [-p profile] [-c corpus]   # per-metric breakdown
gecko compare  [<layout>...] [--corpora a,b,c]                   # rank layouts (all references if none);
                                                                  #   --corpora: score/rank per corpus (L05)
gecko analyze  <layout> [--top 10] [--words]                      # finger loads, worst SFB/SFS/LSB/scissors; JSON adds all bigrams/skipgrams with a `kind`;
                                                                  #   --words: worst real words per metric (needs -c shai or mt-10k)
gecko optimize [--algo sa|hc|ga|memetic|bls|tabu|explore|qd|nsga2|rwsa] [--time 30s | --evals N] [--seed 1] [--threads N]
               [--pin ",./;"] [--from <layout>] [--top 10] [--sa-steps N] [--ga-pop N]
               [--memetic-pop 24] [--local-search hc|tabu] [--local-steps 12] [--islands N]
               [--migration-interval 5] [--migrants 2]
               [--objectives B01,T03,M01] [--nsga-generations N] [--front-size N]
               [--moves swap|columns,cycle3,depth2|all] [--depth2-width 32]
               [--holdout shai]                                   # also score results on a corpus the search never sees (L06)
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
gecko perf     [--layout qwerty] [--record]                       # full evals/s, swap deltas/s, scorer builds/s, 1-key rebuilds/s (1 thread); --record appends to bench/history.jsonl
gecko corpus build <files or dirs...> [--out english] [--charset default|code|nav|<symbols>] [--ext txt] [--fold]
gecko corpus mix <corpus>[:weight]... --out <name>                     # weighted mix, same charset
gecko corpus import <cmini corpus dir> --out <name> [--charset ...] [--fold]   # words.json or n-gram JSON; --fold: é→e; also writes <name>.words.tsv
gecko geometry import-kle <file.json> --out <name> [--force]       # KLE board → data/geometries/<name>.toml
                                                                  #   (fingers/rows/home/effort guessed; edit after)
gecko layouts import <file> [--from oxeylyzer|genkey|cmini|auto] [--out <name>] [--force]
               [--name N] [-g geometry]          # → layout TOML (stdout, or data/layouts/<out>.toml)
gecko layouts export <layout> --to oxeylyzer|genkey|cmini [-g geometry]   # 30 alpha keys → stdout
gecko layouts svg <layout> [--color plain|fingers|heat] [--base-only] [--no-title] [--out f.svg] [-g -p -c]
                                                                  # the web Export tab's image: every layer,
                                                                  #   title + score; PNG: rsvg-convert f.svg
gecko layouts import-cmini <cmini checkout>                           # → data/community/cmini.json
gecko layouts likes [--top 10] [--dups] [-g -p -c]                     # GECKO rank vs cmini likes (Spearman)
gecko corpus nav <corpus> [--share 0.03] [--seed 1] --out <name>        # add modelled arrow keys
gecko profile normalize [-p profile] [--layouts a,b] [--out name]   # weights → spread units
gecko profile fit [--target E05] [--layouts a,b] [--out name]       # fit weights to a judge metric
gecko profile fit --target likes [--seed 1] [--out learned] --json  # pairwise fit, 20% layout holdout
gecko list     layouts|geometries|profiles|corpora|runs|metrics
gecko serve    [--port 7777] [--web web/dist]                          # serves embedded web UI (with embed-web) or filesystem dist
```

Global flags: `--json`, `--root <dir with gecko.toml>`.

`--geometry` accepts board names, explicit `.toml` paths and parametric `.param.toml`
paths (`corne-36.param` also selects that named model). See [geometry.md](geometry.md).

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

### Multi-objective optimization (NSGA-II / RWSA)

When `--algo nsga2` or `--algo rwsa` is selected, `best.json` contains the non-dominated Pareto front over the chosen `--objectives` (default `B01,T03,M01`):
- `objectives`: list of evaluated criteria with their unit, optimization direction (`min`/`max`), and cost description.
- `front`: list of non-dominated layouts with standard layout fields (`rank`, `score`, `chars`, `rows`, `hash`, `breakdown`) plus:
  - `raw`: vector of metric values in native units (e.g. percentages or effort).
  - `costs`: vector of normalized minimization costs used during dominance sorting.


## Conventions

- `--json` on every command; stable schema with `"schema": 1`.
- Human output is compact tables. Progress goes to stderr, results to stdout.
- Exit code 0 on success, non-zero with a one-line `error:` on failure.
- `optimize` always writes `runs/<timestamp>-<algo>-<hash>/` and prints its id. It then prints
  a lower bound and the best layout's optimality gap (`bound` in `best.json`, `algorithms/bounds.md`).
- `optimize --holdout <corpus>` (L06) scores every ranked layout on a second corpus that the
  search never sees, with the same board and profile, and prints how much of the gain carries
  over: `holdout shai: start 70.155 → best 18.593 (search 69.584 → 17.809); keeps 99.6% of the
  gain, rank ρ 0.66 over the top 10`. Kept gain = (start − best) on the holdout ÷ the same on
  the search corpus. Near 100% means the result is not fitted to the search corpus. Rank ρ is
  the Spearman correlation of the top-K scores on the two corpora. Each ranked layout's
  `holdout` score is in `best.json`, with the summary under `holdout`. `config.json` records the
  corpus (name + hash) and `verify` checks it. Holding out the search corpus itself is an error.
- `--evals` budgets are deterministic for a given seed and thread count; `--time` is not.
- `verify` loads geometry and corpus by their recorded names (the profile is stored in
  `config.json`). If a hash differs it fails without re-running. Otherwise it re-runs with the
  recorded params into a temp dir, leaving the run untouched, and compares evals and every
  ranked layout: chars exact, score to 1e-9 relative. Exit code 1 when it does not
  reproduce; time-budget runs are an error.
- Defaults (geometry, corpus, profile, reference layout, data/runs dirs) come from `gecko.toml`.

`profile fit --target likes` uses unique positively liked cmini layouts, not `--layouts`.
Its schema-1 JSON has `fit` (weights/scales, seed, input hashes, training/held-out layout
names, pair counts, logistic loss, training/test accuracy and default/Engram baselines) and
`profile`. Prediction ties count half, like ties are excluded; absent test pairs report null.
`--out learned` writes an opt-in profile; defaults are unchanged. See
`research/experiments/2026-10-21-profile-ranking.md` for fitting rules, measured accuracy and limitations.
