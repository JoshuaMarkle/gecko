# Phase 2 evaluation: does the search beat the best known layouts? (#141)

2026-10-23, GECKO `4e8ace4` (binary built from a clean export of that commit), default
profile of that date, now `balanced` (rerun with `-p balanced`). The 2026-10-24 default's optimum
scores 8.074 on ansi-30/english and beats galite-reference by 32 % under it
([cost-function optimization](../research/experiments/2026-10-24-cost-function-optimization.md)). Data, scripts and the 12 winning runs: `bench/phase2-2026-10-23/` (README there has
the commands). Phase 1 (#140, `objective.md` §Validation) showed the scorer agrees with cmini
and Oxeylyzer, so "better under our profile" can now be read as a claim about the search.

**Answer: yes, on both boards and both corpora.** BLS with 25 M evaluations reaches the same
layout from (nearly) every one of 30 seeds, and even its worst seed beats the best known layout
with the same symbols by 5–16 %. The gain survives a holdout corpus. The exact bounds can
certify the result only on sub-problems (18 free letters on ansi-30), not on the whole board.

## Setup

- **Algorithm:** BLS, the leaderboard's best: `gecko bench --algos sa,bls,tabu,memetic
  --seeds 5 --time 5s --threads 4` at this commit gives BLS stdev 0 on ansi-30 and corne-36
  (SA ties on ansi-30, trails on corne-36; tabu and memetic are worse). 4 threads, `--evals`
  budgets so every run is deterministic and replays with `gecko verify`.
- **Cells:** ansi-30 and corne-36 × `english` and `shai` × 30 seeds (1–30). Budgets 1 M, 5 M and
  25 M evaluations (about 1 s, 7 s and 60 s wall clock; 12 jobs shared 20 cores, so times are
  inflated). On corne-36 the thumb keys are pinned to the board's fills, so the search places
  the same 30 symbols as the 30-key references.
- **Holdout:** `--holdout shai` for english searches and vice versa; then all five corpora.
- **Symbol sets.** A layout's score depends on which four punctuation symbols share the 30 keys
  (an empty key, or `;` in place of the more frequent `'`, drops cost). Comparing layouts with
  different sets is not a fair comparison, so each reference is compared with a search over its own symbols:

  | set | symbols | named references | cmini layouts in set |
  |---|---|---|---:|
  | qwerty | `, . / ;` | Canary (QWERTY's set, the default start) | 271 |
  | apos | `' , . /` | Graphite, Semimak | 356 |
  | semi | `' , . ;` | Sturdy | 620 |

  "Best cmini" is the best of `data/community/cmini.json` in the set: 30 distinct symbols, all
  26 letters, no empty key, angle-mod layouts excluded (their fingering is not the board's).
  Without the filter the top cmini "layouts" are drafts with three blank keys (`gralmak-qzb-test`
  11.21 on ansi-30/english), which look better only because punctuation is off the layout.
  The qwerty set ran all three budgets, apos and semi 25 M only.

## Results (25 M evaluations, 30 seeds)

| board | corpus | set | best | worst seed | seeds at best | best known (same set) | margin, best / worst seed |
|---|---|---|---:|---:|---:|---|---|
| ansi-30 | english | qwerty | 12.154 | 12.154 | 30 | galite-reference 14.587 | 16.7 % / 16.7 % |
| ansi-30 | english | apos | 12.292 | 12.292 | 30 | gralmaks 13.578 (Graphite 14.742) | 9.5 % / 9.5 % |
| ansi-30 | english | semi | 12.443 | 12.443 | 30 | nrts-oxey 13.870 (Sturdy 17.670) | 10.3 % / 10.3 % |
| ansi-30 | shai | qwerty | 12.642 | 12.753 | 24 | galite-reference 14.178 | 10.8 % / 10.0 % |
| ansi-30 | shai | apos | 12.826 | 13.042 | 24 | grapheti 13.757 (Graphite 14.501) | 6.8 % / 5.2 % |
| ansi-30 | shai | semi | 12.822 | 12.932 | 25 | galliumb 13.629 | 5.9 % / 5.1 % |
| corne-36 | english | qwerty | 9.863 | 9.929 | 27 | gallium-stdpct 11.748 | 16.0 % / 15.5 % |
| corne-36 | english | apos | 9.943 | 10.009 | 28 | graphite_taglish 11.120 (Graphite 11.993) | 10.6 % / 10.0 % |
| corne-36 | english | semi | 10.022 | 10.058 | 28 | gallium_sabo 11.201 | 10.5 % / 10.2 % |
| corne-36 | shai | qwerty | 10.289 | 10.289 | 30 | galite-reference 11.678 | 11.9 % / 11.9 % |
| corne-36 | shai | apos | 10.387 | 10.387 | 30 | graphite_taglish 11.204 | 7.3 % / 7.3 % |
| corne-36 | shai | semi | 10.387 | 10.387 | 30 | galliumb 11.229 | 7.5 % / 7.5 % |

Semimak and Canary score 14–20 in every cell; full table: `summary.json`, `refs.json`.

- **Budget.** At 1 M the median seed is already better than every known layout (ansi-30/english
  median 12.316, worst 12.917, vs 14.587), but only 5 of 30 seeds reach the best. At 5 M 20 of 30 do;
  at 25 M 24–30 of 30 do. The worst 25 M seed is within 1.7 % of the best in every cell.
- **Symbol sets do not change the letters.** Within each board and corpus, the three sets' winners
  have the same letter arrangement and differ only in the punctuation (e.g. ansi-30/english
  `lwdgb zquo. / rntsf pheai / jxmcv ky/;,`). So the letter layout is robust to the choice
  of punctuation.
- **The ranking of references matches Phase 1:** the gallium/graphite family dominates the community
  bests; named Graphite is never the best of its own set (variants of it are).

### Where the gain comes from (`breakdown.json`)

Weighted cost, winner minus best known; the same pattern in all 12 cells:

- **Gains:** stretch (B14, −0.4 to −0.7), scissors (B05, −0.3 to −0.5) and half scissors (B06,
  −0.1 to −0.6), row jumps (B10) on shai, and pinky–ring pairs (B09) in about half the cells
  (it is a cost in the others). On corne-36/english also
  outward rolls (T04, −0.5). Lateral and vertical stretches are where hand-tuned layouts leave
  the most.
- **Costs:** the winner pays slightly more SFB (B01, +0.1 to +0.3; e.g. 0.94 % vs 0.87 % on ansi-30/
  english/apos) and effort (M01, +0.1 to +0.4) for that. Against the qwerty-set references, which
  are weaker, SFB is a gain as well (1.12 % → 0.89 %).
- So the margin is not one metric being gamed: it is many small stretch and scissor terms that the
  community layouts never optimized for, paid for with a little SFB. Whether typists value that
  trade the way the profile does is a profile question (`objective.md`), not a search question.

### Holdout: is the gain corpus overfitting?

| board | search corpus | winner on holdout | best known on holdout |
|---|---|---:|---:|
| ansi-30 | english | 13.273 (shai) | 14.178 |
| ansi-30 | shai | 12.818 (english) | 14.587 |
| corne-36 | english | 10.546 (shai) | 11.678 |
| corne-36 | shai | 10.588 (english) | 11.748 |

The winner beats every known layout on the corpus it never saw (qwerty set; apos and semi agree,
`summary.json`). Across all five corpora (`corpora.txt`, qwerty set vs galite-reference,
gallium-stdpct, Graphite, Canary):

- The **shai** winner ranks 1st or 2nd on every corpus on both boards (mean rank 1.4 / 1.6).
- The **english** winner is 1st on english and english-cased on both boards; on reddit it is
  2nd on ansi-30 and 1st on corne-36. It is **5th of 6 on `mt-450k`**
  on ansi-30 (20.200 vs galite-reference 19.948) and 4th on corne-36 (16.064 vs 15.872). `mt-450k`
  is a word list with every word once, so it weighs rare words heavily. The english
  winner transfers less well to this word-list distribution; shai (iweb, broad) generalizes.
  **Search on `shai`** when one layout has to serve several kinds of text.

### Bounds: how close to optimal?

- **Whole board: no useful certificate.** The total lower bound (`bound.total.lower`) is negative
  (−5.6 on ansi-30, −2.9 on corne-36). The reward metrics (rolls, alternates) are negative
  terms and the trigram relaxation bounds them far below what any layout gets. A gap over 100 %
  says nothing. Without the exact bound, the evidence for near-optimality is the 30-seed agreement.
- **Sub-problems: certified.** `gecko certify run:<winner> --free 18 --kind full` (the 18 most
  frequent letters free, the rest fixed; 6.4·10¹⁵ placements):

  | board / corpus | full, k = 18 | pair part, k = 22 |
  |---|---|---|
  | ansi-30 / english | **optimal** (5.3 M nodes, 127 s) | a better pair part exists (21.618 vs 21.733) |
  | ansi-30 / shai | **optimal** (1.3 M nodes, 93 s) | optimal |
  | corne-36 / english | not finished in 300 s (93 M nodes) | optimal |
  | corne-36 / shai | not finished in 300 s (41 M nodes) | a better pair part exists (14.741 vs 14.785) |

  The pair kind only covers bigrams and skipgrams; an "improved" pair part is expected because
  the search trades pair cost for trigram and load terms. It is not evidence of a better
  score. Corne-36 is harder for branch and bound: its root bound is weaker (3.4 vs 4.7 on
  english) and fewer subtrees are cut.

## Reproducibility

All 12 winning 25 M runs (`winners.txt`, copies in `runs/`) pass `gecko verify`. The other 588
runs are in the local `runs/` only; `seeds-*.json` records each one's id, score and layout.

**Scorer invariance bug found, fixed 2026-10-24.** The same layout scored 12.153876 when the
search started from it and 12.154293 when it started from QWERTY (relative 3·10⁻⁵). The cause
was in `truncate_coverage` (`scorer/build.rs`): it drops trigrams below the coverage cut and
broke frequency ties by layout symbol index, which follows the start layout's order, so the
kept trigram set depended on the start (`Scorer::coarse` had the same tie-break). Both now
break ties by characters; the two starts agree to 1.6·10⁻⁷ (12.1539333 / 12.1539335). Scores
shift by at most 10⁻³ (trigram metrics; reference ranking unchanged), far below every margin
above. The winning layouts are unchanged, but 6 of the 12 runs reproduce only with a binary
from before the fix (`4e8ace4`), since their recorded scores moved in the 4th–7th decimal.

## Limits

- One profile (`balanced`, the default at the time). The margins are "better under GECKO's profile". The profile's weights
  are fit to community preferences only loosely (`objective.md`), and the gain is mostly in
  stretch/scissor terms that analyzers like cmini weigh less.
- Community layouts are scored on GECKO's boards. Many were tuned for ortho boards or other
  analyzers, so they would do better on their own target.
- No typing test: whether people type the winners faster or more comfortably is outside Phase 2
  (`goals.md`).
