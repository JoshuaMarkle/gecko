# Lower bounds and optimality gap (B6, #37)

`gecko-core/src/bound.rs`. After every `gecko optimize` run, the best layout gets a lower
bound on the objective. It is printed below the breakdown and stored in `best.json` → `bound`
(`method`, `symbols`, `iters`, `total`/`pair` = `{lower, gilmore_lawler, value, gap_pct}`,
`elapsed_ms`). `gap_pct = (value − lower) / |value|`: the layout is at most that far above the
optimum. It is omitted with more than 48 movable symbols or a negatively weighted load metric.

## Method

1. **QAP form** (`Scorer::qap_terms`). Score = `Σ M[x, p(x)] + Σ_{x≠y} Σ_ch F_ch[x, y]·C_ch[p(x), p(y)]`.
   The channels are bigrams, skipgrams, far skipgrams, `symbol_bigram`, Shift pairs and `brackets`.
   Same-symbol pairs fold into `M`.
2. **Trigrams are relaxed to pairs**, so the result is still a lower bound. A trigram x y z with
   distinct symbols is split into thirds: (x, y) at (i, j) costs `min_k c3[i, j, k]`, (y, z)
   costs `min_i c3`, and (x, z) costs `min_j c3`, with the free key on any other slot.
   Trigrams with a repeated symbol are exact pair or mono terms.
3. **Load terms** (`finger_target`, `finger_evenness`, `hand_balance`, `finger_peak`, `thumb_load`) are ≥ 0 when their weights are ≥ 0, so they are
   bounded by 0 (dropped).
4. **Pins**: pinned symbols are constants. Their terms with movable symbols become linear terms.
5. **Gilmore–Lawler**. For each movable x on free slot i:
   `l[x, i] = M'[x, i] + LAP over the other symbols/slots of ½(Q(x,y,i,j) + Q(y,x,j,i))`.
   The bound is the LAP over `l` (Hungarian algorithm, O(m³)). There are m² inner LAPs.
6. **Dual ascent** (Hahn–Grant / Carraresi–Malucelli style; 20 iterations, `ASCENT_ITERS`).
   The LAP duals rewrite the objective exactly as `K + Σ lin + Σ R` with `lin, R ≥ 0`, so `K`
   is a bound. Then R is symmetrized, each `lin[x, i]` is spread over its m − 1 pair terms,
   and the LAPs are solved again. `K` does not decrease. Memory is m⁴ f64 (6.5 MB at 30).

Two bounds are reported:

- `total` bounds the whole objective and certifies the run.
- `pair` bounds only the mono + bigram/skipgram part (`Scorer::parts().0`), which drops
  trigram and load terms. It compares the best layout's pair part with the best pair part any
  layout could reach. The issue goal ("within x % of optimal for the bigram part") reads this
  number. It is not a certificate for the run, because the optimizer trades pair cost for trigrams.

Tests (`tests/scorer.rs`): on the 6-key board, both bounds are ≤ the brute-force optimum, with
and without pins. On ansi-30 they are ≤ the score of 200 random layouts. The Hungarian algorithm
is checked against brute force (`bound::tests`).

## Results (2026-10-21, ansi-30 / english / default, SA 20 s seed 1, run `20261021-171218-sa-a3306812`)

| free symbols | total: lower (plain GL) | total gap | pair: lower (plain GL) | pair gap |
|---|---|---|---|---|
| 30 (none pinned) | 1.71 (0.24) vs 17.561 | 90.3 % | 5.78 (4.49) vs 19.453 | 70.3 % |
| 16 (pin `xvqjzyw/;,.bmk`) | 13.26 (11.31) | 24.5 % | 17.11 (15.27) | 12.1 % |
| 10 (pin 20, free `nsrhtceaio`) | 15.30 (15.19) | 12.9 % | 19.12 (19.05) | 1.7 % |

- A pair-only profile (default without T*, `finger_target`, `hand_balance`) gives the same picture. SA's optimum
  there is 19.276 (two seeds agree), and the bound is 5.81 after 50 iterations (69.9 %). So the
  weakness is the bound, not the search. Gilmore–Lawler is known to be weak on dense QAPs, and
  dual ascent closes only ~1.3 of ~14.8 points.
- Ascent convergence (pair-only, 30 free): 4.49 → 5.09 (1) → 5.71 (10) → 5.77 (20) → 5.81 (50).
  Each iteration takes ~25 ms at m = 30, so `optimize` spends ~0.9 s on both bounds.
- With the 30 symbols all free, the gap does not certify anything. With 10 free keys the pair
  part is certified within 1.7 %. The remaining total gap there (12.9 %) is mostly the trigram
  relaxation: the trigram terms net −2.1 in the best layout and are bounded far below that.
  The load terms contribute only 0.23.
- Exact optima on small free sets: see "Certified sub-problems" below. Tighter full-board
  bounds would need RLT-2 or an SDP. That work is not planned.

## Certified sub-problems (B7, #43)

`gecko-core/src/certify.rs`, `gecko certify run:<id>` ([cli.md](../cli.md)). Keep all but k
symbols of a layout on their slots and find the best placement of the k free ones, exactly.

- **Branch and bound**, depth first, one free symbol per level (most frequent first), child
  slots cheapest first. The root runs the dual ascent above once (`bound::ascent`), which
  rewrites the objective exactly as `K + Σ lin + Σ R` with every term ≥ 0. A node's bound is
  the placed symbols' cost in that form plus Gilmore–Lawler over the rest (O((k − d)⁵)). A node
  is cut when its bound reaches the incumbent (the start layout at first) minus `tol`
  (1e-9 relative: smaller "improvements" count as ties).
- **Leaves are exact.** `full` keeps a real `State` in step with swap deltas (`Scorer::delta`;
  score and loads are restored after each undo, so there is no drift) and compares
  `Scorer::full`. `pair` sums the QAP form, which is exact for the pair part. The reported
  optimum is re-scored from scratch.
- **Two kinds, never mixed.** `full` proves the optimum of the real objective. `pair` proves the
  optimum of `Scorer::parts().0` only and does not certify the run's score.
- A time or node limit gives the best placement found and a lower bound (the smallest bound on
  the open path: in practice the root bound), not a certificate.
- Chosen over a HiGHS MIP: decision in [decisions/search.md](../decisions/search.md).

Tests (`tests/certify.rs`): equal to exhaustive enumeration for both kinds on the 6-key board and on
ansi-30 with 5–7 free symbols; pins respected; deterministic; "already optimal" survives every swap.
An `--ignored` test enumerates the 10-free instance below (3.6 M placements) and agrees.

### Results (2026-10-22, run `20261021-171218-sa-a3306812`, ansi-30 / english / default, 1 thread)

Free `nsrhtceaio` (the bounds table's 10, 20 pinned):

| kind | run value | certified optimum | true gap | bound gap (above) | nodes | leaves | time |
|---|---|---|---|---|---|---|---|
| full | 17.5610 | 17.5610 | 0 % | 12.9 % | 290 | 26 | 0.03 s |
| pair | 19.4533 | 19.4533 | 0 % | 1.7 % | 53 | 0 | < 0.01 s |

**SA found the optimum.** No placement of these 10 symbols beats the run, on the score or on
the pair part. So both bound gaps (12.9 %, 1.7 %) are entirely bound weakness. The search
still finishes fast: once a frequent letter is on a wrong slot, the placed cost plus the bound
passes the incumbent within a few levels, so the weak root bound does not matter.

Default free sets (`--free k`: the k most frequent letters, `etaoinshrdlumwcf…`), all certified
optimal unless noted:

| k | placements | full: nodes / time | full root bound | pair: nodes / time |
|---|---|---|---|---|
| 10 | 3.6 M | 490 / 0.01 s | 15.21 | 53 / < 0.01 s |
| 12 | 4.8·10⁸ | 2.5 k / 0.1 s | 15.06 | 76 / < 0.01 s |
| 14 | 8.7·10¹⁰ | 6.7 k / 0.5 s | 14.50 | 273 / 0.1 s |
| 16 | 2.1·10¹³ | 28.5 k / 3.3 s | 13.56 | 1.5 k / 0.6 s |
| 18 | 6.4·10¹⁵ | 81 k / 12.7 s | 12.71 | 3.3 k / 2.2 s |
| 20 | 2.4·10¹⁸ | 418 k / 74 s | 11.96 | 9.3 k / 10 s |
| 22 | 1.1·10²¹ | stopped at 300 s (2.4 M nodes, 5 % cut) | 10.83 | 41.6 k / 66 s |
| 24 | 6.2·10²³ | not run (22 already stops) | | stopped at 300 s (156 k nodes, 3 % cut); root 13.08 |
| 30 | 2.7·10³² | stopped at 20 s (3.3 k nodes), lower 1.72 | 1.72 | stopped at 60 s (12 k nodes), lower 5.78 |

- Where it stops: `full` is practical up to k ≈ 20 (about a minute); at 22 it does not finish in
  5 minutes. Time grows about 5× per 2 extra symbols. `pair` reaches k = 22 in about a minute and stops at 24.
  `full` is the harder kind because the relaxed trigrams make its bound weaker: its root gap is
  32 % at k = 20, against 19 % for `pair`.
- A stopped run's lower bound is the root bound, so it says little more than `optimize`'s
  bound. A useful partial certificate would need bounds on the open subtrees; not done.
- k = 12 asked by the issue: certified in 0.1 s (full) / < 0.01 s (pair).
