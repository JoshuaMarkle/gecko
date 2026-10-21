# CLI

One binary, `gecko` (`crates/gecko-cli`). Built for agents first: deterministic, scriptable, JSON output.

## Commands

```
gecko score    <layout> [-g geometry] [-p profile] [-c corpus]   # per-metric breakdown
gecko compare  [<layout>...] [--corpora a,b,c]                   # rank layouts (all references if none);
                                                                  #   --corpora: score/rank per corpus (L05)
gecko analyze  <layout> [--top 10]                                # finger loads, worst SFB/SFS/LSB/scissors; JSON adds all bigrams/skipgrams with a `kind`
gecko optimize [--algo sa|hc|ga|bls|tabu|explore] [--time 30s | --evals N] [--seed 1] [--threads N]
               [--pin ",./;"] [--from <layout>] [--top 10] [--sa-steps N] [--ga-pop N]
               [--moves swap|columns,cycle3,depth2|all] [--depth2-width 32]
gecko bench    [--algos hc,sa,ga,bls,tabu] [--seeds 3] [--time 5s]         # algorithm leaderboard
               [--moves "swap/cycle3/depth2"] [--depth2-width N]      # one row per algo × variant
gecko explore  [--time 60s | --evals N] [--seed 1] [--threads N] [--pin ...] [--from <layout>]
               [--samples 2000] [--max-optima 100000]         # map local optima; writes an atlas run
gecko landscape [--samples 4000] [--walks 64] [--walk-len 400] [--seed 1] [--from <layout>]
                                                                  # random-score distribution, correlation
                                                                  #   length, reference percentiles, symmetry
gecko atlas    <run> [--top 10] [--cut 8] [--pool 300] [-p profile]
                                                                  # summary, ranked, families, consensus,
                                                                  #   funnels, FDC; -p re-ranks under a profile
gecko verify   run:<id>                                           # re-run an --evals run from config.json;
                                                                  #   check input hashes, evals and best.json
gecko perf     [--layout qwerty] [--record]                       # full evals/s and swap deltas/s (1 thread); --record appends to bench/history.jsonl
gecko corpus build <files or dirs...> [--out english] [--charset default|code|<symbols>] [--ext txt]
gecko corpus mix <corpus>[:weight]... --out <name>                     # weighted mix, same charset
gecko corpus import <cmini corpus dir> --out <name> [--charset ...]   # words.json or n-gram JSON
gecko geometry import-kle <file.json> --out <name> [--force]       # KLE board → data/geometries/<name>.toml
                                                                  #   (fingers/rows/home/effort guessed; edit after)
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
