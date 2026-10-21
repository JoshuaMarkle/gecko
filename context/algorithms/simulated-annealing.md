# Simulated annealing

**Idea:** random swaps; always accept improvements, accept a worsening Δ with
probability `exp(−Δ/T)`. T falls over time so the search moves from exploring to refining.

**Schedule:** exponential `T(i) = T0 · exp(−k·i/N)` (carpalx). Set T0 from data: sample
random swaps at start, choose T0 so ~50–80% of worsening moves are accepted. End
temperature so acceptance of worsening moves is < 0.1%. Finish with a steepest-descent
polish.

**Parameters:** `T0` (or `accept0`), `T_end` (or `accept_end`), `steps`, `restarts`,
`reheat` (optional restart from best at higher T), `neighborhood`.

**Pitfalls:** T0 must be scaled to the score range (carpalx author's main warning); too
short a schedule = a noisy hill climb. Many independent runs beat one long run for
finding diverse good layouts (Kühlmak).

**Variants to try:** adaptive cooling (keep acceptance rate on a target curve), parallel
tempering (replicas at several temperatures exchange states).

## Results

Implemented: `optimize/anneal.rs`. `steps = 0` (default) = auto, ≈ 4 restarts per worker in the budget; T updated every 256 steps. Best under the current default profile (spread units, B14, S02, E05): 15.704,
`jkndfzquo, rthcpxseai lmbwvyg/;.` (20 s, seeds 1 and 2 agree). Ranks 1st on english/shai/mt-10k,
2nd on mt-450k, and edges out Graphite on measured comfort (E05 5.22 vs 5.25). The previous
profile's optimum (8.396 then) ranked last of all layouts on E05. Adding S04 (skip stretch,
0.19) leaves the optimum unchanged up to `,`/`.` (16.476, run `20261021-062404-sa-cf288ac0`,
15 s seed 1); its S04 7.71 sits mid-pack among references (Colemak-DH 6.56 … QWERTY 13.07).

Leaderboard 2026-10-20 (`gecko bench --time 5s --seeds 3`, 20 threads, i9-13900H, default
profile, english corpus, ansi-30; lower is better; Semimak 11.85, Graphite 12.26, QWERTY 53.31):

| algo | mean | best | stdev | M evals/s |
|---|---|---|---|---|
| hc | 8.603 | 8.539 | 0.077 | 3.2 |
| sa | 8.416 | 8.408 | 0.005 | 3.2 |
| ga | 8.890 | 8.566 | 0.249 | 0.9 |

Thumb boards 2026-10-20 (`--geometry corne-36 --time 20s`, default profile, english incl.
space; Semimak 12.34, Sturdy 12.43 with Miryoku thumbs):

| pins | score | layout | run |
|---|---|---|---|
| all six thumbs (`{space}{enter}{bspc}{tab}{esc}{none}`) | 9.857 | `jmypvqbuo, rtsdwxneai lkfcgzh/;. ⎋␣⇥⏎⌫·` | `20261020-222832-sa-705705b1` |
| tokens only (empty thumb free) | 8.066 | `bldvz·,ajy nrtmg.ioeh pxkcw;uq/f ⎋␣⇥⏎⌫s` | `20261020-222852-sa-1ef8fe2f` |

With the empty right-outer thumb movable, SA moves a frequent letter there (`s`, effort 1.8)
and leaves a top-row slot empty: a letter on a thumb is worth ~1.8 points under the default
profile. Thumb-trigram and thumb-load metrics (M09) don't exist yet, so this probably
overrates thumb letters.

Layers 2026-10-20 (`--from layered/corne-code --geometry corne-36 --corpus code --profile code
--time 20s`, base letters and thumbs pinned; 102 slots, 2.3 M evals/s on 20 threads): start
score 34.16 (Y01 11.0 %, Y02 25.0 %) → 27.972 (run `20261020-224013-sa-0d1258cd`). SA swaps
which layer holds digits and which holds symbols: most digits end up on ① (left inner thumb), the
code symbols on ② (right outer thumb). It converges in ~4 s; the top 5 differ in where a few
rare symbols and digits sit.
