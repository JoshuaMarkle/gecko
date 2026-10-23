# Phase 2 evaluation (#141), 2026-10-23

Write-up: `context/evaluation.md`. GECKO `4e8ace4`, built from a clean `git archive` of that
commit (the shared work tree had unrelated uncommitted changes), default profile.

```
gecko --json bench --algos sa,bls,tabu,memetic --seeds 5 --time 5s --threads 4 -g <board>   # algorithm choice
for b in ansi-30 corne-36; do for c in english shai; do
  python3 -I run.py <gecko> $b $c > seeds-$b-$c.json                                          # 1M,5M,25M, qwerty set
  python3 -I run.py <gecko> $b $c 25000000 "raw:abcdefghijklmnopqrstuvwxyz',./" > seeds-$b-$c-apos.json
  python3 -I run.py <gecko> $b $c 25000000 "raw:abcdefghijklmnopqrstuvwxyz',.;" > seeds-$b-$c-semi.json
done; done
python3 -I refs.py <gecko> > refs.json          # named references + best cmini per symbol set
python3 -I summarize.py > summary.json          # Markdown table on stderr
python3 -I breakdown.py <gecko> > breakdown.json
gecko --json certify run:<winner> --free 18 --kind full --time 300s > certify-<winner>-full.json
gecko --json certify run:<winner> --free 22 --kind pair --time 300s > certify-<winner>-pair.json
gecko compare <winners> cmini:galite-reference cmini:gallium-stdpct graphite canary -g <board> \
  --corpora english,english-cased,shai,mt-450k,reddit > corpora.txt   (--json: corpora-<board>.json)
```

`runs/` holds the 12 winning 25 M runs (`winners.txt`) and their `gecko verify` output; replay one
by copying it into the repository's `runs/` and running `gecko verify run:<id>`.
