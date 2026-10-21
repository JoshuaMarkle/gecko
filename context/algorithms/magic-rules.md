# Magic-key rule search

Model: `context/representation.md` (adaptive keys, corpus rewrite). Code: `magic::search_rules`,
CLI `gecko magic`.

**Problem:** for a layout with an adaptive key on it, choose rules `previous → output` (at most
one per previous symbol) that minimize the profile score. Positions are fixed; the layout
optimizer moves keys with the rules fixed, so co-optimization alternates the two.

**Algorithm (lazy greedy + prune):**
1. Candidates: every `w → x` with both symbols on the layout and `w x` in the corpus
   (`x = w` only if the key does not already repeat), ≈ 750 on ansi-30.
2. Score each candidate exactly once: rewrite the corpus, build the scorer, full evaluation
   (≈ 2.3 ms each, ~1.8 s for a search).
3. Repeatedly take the candidate with the largest stored gain; if its gain is stale, re-score
   it against the current rules and put it back; if fresh and > `min_gain` (0.01), accept it
   and drop the other candidates for the same `w`. Stop at `max_rules` (32) or no gain.
4. Prune: drop any rule whose removal lowers the score.

Candidate lists of each symbol's most frequent followers (first try, top 8) missed the
valuable rules: `eu`, `ao`, `ue` are rare bigrams but costly SFBs. Gains are not exactly
submodular (a rule can change what another one is worth), so lazy greedy is a heuristic.

**Not yet:** rules for several keys at once (search them one key at a time with `--key`),
a learning-cost term per rule, swap moves between rules, and co-optimization in one command.

## Results

ansi-30, default profile, `english`, 2026-10-21 (score: lower is better; Graphite 19.68,
Sturdy 20.08 on this profile).

| Layout | Rules | Score | SFB |
|---|---|---|---|
| Sturdy | — | 20.08 | 0.89 % |
| Magic Sturdy, key without rules | — | 20.11 | 0.69 % |
| Magic Sturdy, published rules (14) | `ao cy dy eu gy lk oa py rl sk ue yp ks tm` | 17.95 | 0.27 % |
| Magic Sturdy, searched rules (23) | `ue rl oa dy ly e' cy ab my py qu gy tm sm i. wd kl yp ., 'e xt nf` | 17.06 | 0.23 % |
| Magic Sturdy, searched, `-p sfb` | `ue rl oa dy e' cy py ay tm gy i. lr wd yd 'e ., m, xv qu` | — | 0.21 % |

- From scratch the search recovers 9 of the 14 published rules (`ue rl oa dy cy py gy tm`
  among its first 14 picks, `yp` later); `ab` beats the published `ao` here, and it adds
  rules for punctuation (`e'`, `i.`, `.,`) and rarer SFBs.
- Sturdy with a key on a dropped letter (`gecko magic sturdy --on <sym>`): `z` 16.02, `j` 16.44,
  `q` 16.78, `;` 17.02, `x` 17.42 (from ≈ 20). Rules like `n.` and `h.` fix Sturdy's
  right-index `.`.
- Co-optimization (`--on z` → `optimize --time 20s` → `magic` → `optimize --time 20s`):
  16.02 → 15.51 → 14.41 → 14.38, SFB 0.23 %:
  ```
  f l n d k ' p u o ,
  s r h t v g c e a i
  ★ x b m j y w ; q .
  ★ wh ue oa dy rl e; ag cy qu py t, ls ik m, go hn nh yd 'a fy ., bl k, ,'
  ```
  The same 20 s optimize without a key reaches 16.16 (SFB 0.56 %). Runs
  `20261021-115821-sa-bcbc80dd`, `20261021-115918-sa-5bfd1a3b`, baseline
  `20261021-115942-sa-40acde86` (local, `runs/` is not committed).
- Caveat: 20+ rules are a lot to learn; the score has no cost per rule. Use `--max-rules`
  (Magic Sturdy has 14) or a larger `--min-gain` to trade gain for learnability.
