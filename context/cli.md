# CLI

One binary, `gecko` (`crates/gecko-cli`). Built for agents first: deterministic, scriptable, JSON output.

## Commands

```
gecko score    <layout> [-g geometry] [-p profile] [-c corpus]   # per-metric breakdown
gecko compare  [<layout>...]                                      # rank layouts (all references if none)
gecko analyze  <layout> [--top 10]                                # finger loads, worst SFB/SFS/LSB/scissors
gecko optimize [--algo sa|hc|ga] [--time 30s | --evals N] [--seed 1] [--threads N]
               [--pin ",./;"] [--from <layout>] [--top 10] [--sa-steps N] [--ga-pop N]
gecko bench    [--algos hc,sa,ga] [--seeds 3] [--time 5s]         # algorithm leaderboard
gecko perf     [--layout qwerty]                                  # full evals/s and swap deltas/s (1 thread)
gecko corpus build <files or dirs...> [--out english] [--charset ...]
gecko list     layouts|geometries|profiles|corpora|runs|metrics
gecko serve    [--port 7777] [--web web/dist]
```

Global flags: `--json`, `--root <dir with gecko.toml>`.

`<layout>` is a reference name (`data/layouts/<name>.toml`), a `.toml` path,
`run:<id>[#rank]`, or `raw:<slot-ordered symbols>` (e.g. `raw:qwertyuiopasdfghjkl;zxcvbnm,./`).

## Conventions

- `--json` on every command; stable schema with `"schema": 1`.
- Human output is compact tables. Progress goes to stderr, results to stdout.
- Exit code 0 on success, non-zero with a one-line `error:` on failure.
- `optimize` always writes `runs/<timestamp>-<algo>-<hash>/` and prints its id.
- `--evals` budgets are deterministic for a given seed and thread count; `--time` is not.
- Defaults (geometry, corpus, profile, reference layout, data/runs dirs) come from `gecko.toml`.
