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

Layer modes 2026-10-21 (same setup, `--pin 'qwfpbjluy;arstgmneiozxcdvkh,./{esc}{space}{enter}{bspc}'`):

| start layout | start | Y01 / Y02 start | best | Y01 / Y02 best | run |
|---|---|---|---|---|---|
| `corne-code` (hold, hold) | 34.160 | 11.0 / 25.0 | 27.971 | 9.8 / 22.9 | `20261021-063224-sa-d8fe64ac` |
| `corne-code-numtoggle` (hold, toggle) | 34.583 | 11.8 / 25.0 | 28.809 | 11.5 / 23.2 | `20261021-063305-sa-61baea70` |
| `corne-code-oneshot` (one-shot ×2) | 42.731 | 14.3 / 80.5 | 35.987 | 12.8 / 80.5 | `20261021-063245-sa-7e873956` |

Hold wins on code: symbols come in runs (`()`, `->`, `!=`), which one-shot pays a tap for
each, and Y02 counts every one-shot symbol pair as awkward (80 % is its floor once the base
is pinned). SA does use the thumb keys one-shot frees (`"`, `<`, `3`, `:` land there), but
that cannot make up for the extra taps. A toggled number layer costs a little: numbers in code are short runs,
so the exit tap outweighs saved holds. One-shot should pay off only for layers typed one
symbol at a time, which this corpus/profile doesn't reward.

With Y03 (paired brackets, `code` weight 0.5) on 2026-10-21, same command: 34.160 → 15.902
(run `20261021-070903-sa-80e6b876`, Y03 0 %): `( )`, `[ ]` and `< >` end up mirrored across the
hands of layer ②, `{ }` adjacent.

Digits and arrows 2026-10-21 (D7; `--geometry corne-36 --corpus code-nav --profile code`, base,
thumbs and the symbol layer pinned, `--time 20s` / `10s`, seed 1; arrows are modelled, see
`code-nav` in representation.md):

| layout | score | B01 SFB | Y01 | run |
|---|---|---|---|---|
| `layered/corne-num` (numpad left, 0 on thumb, arrows on right home row) | 38.980 | 3.22 % | 11.65 % | — |
| `layered/corne-numrow` (digits across home row, arrows right top row) | 38.409 | 3.13 % | 11.57 % | — |
| numpad digits pinned, arrows free | 37.713 | | | `20261021-114116-sa-cb12a738` |
| number-row digits pinned, arrows free | 36.696 | | | `20261021-114127-sa-cb12a738` |
| digits and arrows free (both starts reach it) | 35.964 | 2.74 % | 11.99 % | `20261021-114041-sa-b8fef3b2` |

The number row beats the numpad by ~0.6–1.0 points under this profile: numpad columns put
1/4/7, 2/5/8, 3/6/9 on one finger each, so `25`, `28`, `14`/`41`, `36`/`63` and letter–digit
pairs like `s2` are SFBs (digit/arrow SFBs 0.53 % vs 0.44 % of bigrams; the Vim arrows' `←↓`
share the right index in both). Free, SA scatters digits over the low-traffic slots of both layers (`0` on the
right inner thumb of the symbol layer, `9` on its bottom row, `1325··8` on the num home row)
and splits the arrows (`←↑` right home, `↓` left thumb, `→` left top pinky): nothing in the
objective rewards a learnable shape (ordered row, arrow cluster), and modelled arrows follow
letters at random, so they drift to keys no frequent letter shares a finger with. Practical
reading: prefer a number row on the layer; arrow placement needs a cluster constraint or real
cursor-movement data before the optimizer's choice means much.
