# Lower bounds and optimality gap (B6, #37)

`gecko-core/src/bound.rs`. After every `gecko optimize` run, the best layout gets a lower
bound on the objective. It is printed below the breakdown and stored in `best.json` → `bound`
(`method`, `symbols`, `iters`, `total`/`pair` = `{lower, gilmore_lawler, value, gap_pct}`,
`elapsed_ms`). `gap_pct = (value − lower) / |value|`: the layout is at most that far above the
optimum. It is omitted with more than 48 movable symbols or a negatively weighted load metric.

## Method

1. **QAP form** (`Scorer::qap_terms`). Score = `Σ M[x, p(x)] + Σ_{x≠y} Σ_ch F_ch[x, y]·C_ch[p(x), p(y)]`.
   The channels are bigrams, skipgrams, far skipgrams, Y02, Shift pairs and Y03 brackets.
   Same-symbol pairs fold into `M`.
2. **Trigrams are relaxed to pairs**, so the result is still a lower bound. A trigram x y z with
   distinct symbols is split into thirds: (x, y) at (i, j) costs `min_k c3[i, j, k]`, (y, z)
   costs `min_i c3`, and (x, z) costs `min_j c3`, with the free key on any other slot.
   Trigrams with a repeated symbol are exact pair or mono terms.
3. **Load terms** (M02, M03, M04, M07, M09) are ≥ 0 when their weights are ≥ 0, so they are
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

- A pair-only profile (default without T*, M02, M04) gives the same picture. SA's optimum
  there is 19.276 (two seeds agree), and the bound is 5.81 after 50 iterations (69.9 %). So the
  weakness is the bound, not the search. Gilmore–Lawler is known to be weak on dense QAPs, and
  dual ascent closes only ~1.3 of ~14.8 points.
- Ascent convergence (pair-only, 30 free): 4.49 → 5.09 (1) → 5.71 (10) → 5.77 (20) → 5.81 (50).
  Each iteration takes ~25 ms at m = 30, so `optimize` spends ~0.9 s on both bounds.
- With the 30 symbols all free, the gap does not certify anything. With 10 free keys the pair
  part is certified within 1.7 %. The remaining total gap there (12.9 %) is mostly the trigram
  relaxation: the trigram terms net −2.1 in the best layout and are bounded far below that.
  The load terms contribute only 0.23.
- Next: exact optima on small free sets (#43, MIP oracle) use these bounds for pruning.
  Tighter full-board bounds would need RLT-2 or an SDP. That work is not planned.
