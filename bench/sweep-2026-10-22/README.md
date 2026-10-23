# Weight sweep of the 2026-10-22 default profile

`gecko sweep` on ansi-30 / english / default (the retuned profile), start QWERTY, SA 20M
evaluations per weight vector, one seed, 50 log-normal draws at σ = 0.5 (seed 1):

```
gecko sweep --evals 20000000 --draws 50 --seed 1 --out bench/sweep-2026-10-22/default.json
```

`default.txt` is the printed summary; `default.json` holds every weight vector, its optimum,
family, regret, keys moved and judges, plus input hashes. 111 searches, about 8 minutes on all 20
threads.

- Base optimum `lwdgbzquo. rntsfpheai jxmcvky/;,` (score 12.154, E01 6.64, SFB 0.89 %, home row
  63.4 %), the same layout the profile was tuned with.
- Its family holds 55 of 111 weight vectors; the best members of the next three families are
  0.14–0.44 points behind.
- 50 draws: 47 distinct optima, median 21 keys moved, regret median 1.10 (p90 2.01). 10 draws
  find layouts with lower E01 than the base optimum, by up to 5 % of E01.
- Single weights: dropping B01 costs most (regret 1.82); T03, T04, S01, B14, M01 and B09 follow.
  Halving any one weight costs at most 0.40 (T03). M02, M04 and T07 do not move the optimum.

The near-tie note of `gecko optimize` uses 0.4 from these numbers.
