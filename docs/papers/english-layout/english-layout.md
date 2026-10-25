# The best keyboard layout for English: one optimum across seeds, weights and text sources

*Experiment report · 25 October 2026 · GECKO commit baad493 + this study · profile `default` (the evidence-fitted weights of 24 October) · ansi-30 · `prose` corpus (web 50, chat 30, books 20). Data and scripts: `bench/english-layout-2026-10-24/`.*

## Abstract

We search for the best arrangement of the 26 letters and four punctuation keys on a standard row-staggered keyboard for English. The cost function is GECKO's evidence-fitted profile; the text is a new mix of web, chat and book English with punctuation and case. **One layout comes out of every search:**

```
c w d l x   j y o u .
g s t n v   p h e i a
f b m r q   z k ' / ,
```

All 20 seeds of each of three punctuation sets reach it, and so do 84 of 104 re-optimizations under perturbed weights (median: 2 keys moved). The letters are the same whichever four punctuation symbols share the board. It scores 18 % lower than the best community layout with the same symbols (Gallium) and 23 % lower than Graphite. Exact search certifies it optimal over the placements of its 16 most frequent letters. It wins on every English source we tried, including sources it was not searched on. Its margin comes from measured comfort (Engram), outward rolls and fewer pinky–ring pairs, paid for with slightly more same-finger bigrams (1.18 % vs Gallium's 0.92 %) and half as many inward rolls. **The claim has limits that matter.** The fitted timing model makes it only 0.6 % faster per keystroke than Gallium. Two held-out empirical timing judges, built from QWERTY typists, disagree with each other by more than the spread between good layouts, so they cannot confirm or refute the ranking. Text source matters more than seeds or weights: optimizing on books alone or on chat alone gives a different layout that costs 4–6 % on the mix. And a profile that weights SFB heavily (`sfb`) prefers the low-SFB community layouts. The modern layouts (Gallium, Graphite, Galite, Dhorf) are close to this optimum on speed; the classics are far from it on everything.

## Question and setup

**Question.** Which arrangement of the 26 letters and four punctuation keys on the 30 main keys of a standard row-staggered keyboard is best for typing English, how sure can we be, and how much better is it than the layouts people use?

**Board.** `ansi-30`: the three letter rows of an ANSI keyboard with traditional fingering. Space, Shift and the number row are off the modelled keys, as in every community analyzer, so the result is a letter layout that drops into a normal keyboard.

**Text.** `prose` mixes three English sources, each with punctuation and case kept: modern web text (Shai's iWeb sample, 90 M words, 50 %), informal chat (Reddit comments, 10 M words, 30 %) and running book text (ten Project Gutenberg novels, 20 %). Each source is also optimized alone, and so are the three older 33-symbol corpora (`shai`, `reddit`, `english`), to see how much the answer depends on what kind of English is typed.

**Cost.** `default`, GECKO's evidence-fitted profile. Its pair weights come from key-pair typing times of about 49,000 QWERTY typists (Aalto 136M keystrokes). Its comfort term is Engram's measured pairwise preferences (61 people). The rest was chosen by a bilevel search judged on held-out timing and comfort. SFB costs +17 % time per pair, alternating hands saves 8 %, outward rolls pay slightly more than inward ones, and same-finger skipgrams are nearly free (`research/experiments/2026-10-24-cost-function-optimization.md`).

**Symbols.** The four non-letter keys change the score (a rare `;` costs less than a frequent `'`), so three sets are searched: `apos` (`' , . /`, the headline: the apostrophe is the most frequent English symbol after `. ,`), `qwerty` (`, . / ;`) and `semi` (`' , . ;`). Each named layout is compared with the optimum of its own set.

**Search.** Breakout local search (BLS), 25 M evaluations per run, 20 seeds per set on `prose` and 10 per other corpus. Every run is deterministic and replays with `gecko verify`.

**Judges.** Besides the score: (1) two empirical timing judges GECKO never optimizes, built from measured pair times of the *other* half of the Aalto typists (J_B and J_B+l, which differ in how they control QWERTY practice; built in the cost-function study); (2) Engram's measured discomfort (in the objective, so not independent); (3) the rank under three other profiles (`balanced`, `sfb`, `engram`); (4) rank intervals under random weight noise (2,000 draws, σ 0.5).

**Layouts compared.** Six classics (QWERTY, Dvorak, Colemak, Colemak-DH, Workman, Halmak) and eleven modern community layouts (Canary, Graphite, Gallium, Galite, Sturdy, Semimak-JQ, Dhorf, Focal, Hands Down reference, MTGAP, APTv3), with the same key strings as the layout-ranking report of 24 October.

## The optimum

![The optimum and three layouts for scale. Keys are shaded by letter frequency in `prose` (darker = more frequent). Scores are under `default` on `prose` (lower is better). Gallium is the best community layout.](assets/keyboards.pdf)

```
c w d l x   j y o u .        apos set: ' , . /  (headline)      9.521
g s t n v   p h e i a        qwerty set: ; replaces '           9.138
f b m r q   z k ' / ,        semi set:   ; replaces /           9.485
```

Runs: `20261025-034807-bls-04623218` (apos), `20261025-034958-bls-c91dd5a4` (qwerty), `20261025-035150-bls-2e8e18f8` (semi); each is the best of 20 seeds, and all 20 seeds of each set reach the same layout.

**What it does.**

- **Vowels and `h` on the right home row, consonants on the left** (`h e i a` under the right hand, `s t n` under the left): almost every word alternates hands at a vowel. Hand alternation is 70 %, the most of any layout here except Dvorak.
- **Outward rolls over inward ones.** 31 % of trigrams roll outward and 12 % inward. Every community layout rolls inward more (20–33 %). The profile pays 2.3× more per spread for an outward roll, because the Aalto timing shows outward rolls speed the next keystroke more (−1.4 % vs −0.3 %).
- **Rare letters in the centre columns.** `x v q` and `j p z` fill the index stretch columns, so the hard lateral reaches carry little. `q` sits on a centre key, unlike any community layout.
- **Comfort.** Engram's measured discomfort is 3.98, against 5.2–5.4 for the best community layouts and 12.5 for QWERTY.
- **What it pays.** SFB 1.18 % (Gallium 0.92 %, Dhorf 0.70 %) and same-finger skipgrams 8.4 % (Gallium 6.3 %). The profile charges SFB +17 % time per pair and finds no measurable cost for skipgrams, so it gives up a little SFB for comfort and rolls.

![Selected metrics: the optimum (orange), modern layouts (blue) and classics (grey). The optimum leads on hand alternation, outward rolls, pinky–ring pairs and comfort, and trails on inward rolls, SFB and same-finger skipgrams.](assets/metrics.pdf)

Against Gallium (same symbol set as the semi optimum), the score difference of −2.11 splits as: Engram comfort −2.15, outward rolls −1.19, pinky–ring −0.82, alternation −0.35, stretch −0.19; against it SFB +0.95, inward rolls +1.22, finger load +0.38, effort +0.23. Against Graphite (apos) the pattern is the same: comfort −2.27, outward rolls −1.35, inward rolls +1.30.

## How sure is it?

**Search.** 20 of 20 seeds reach the same layout in each symbol set. On the source corpora each of 10 seeds agrees as well (`grid.log`).

**Exactness.** `gecko certify` fixes all but the k most frequent letters and searches every placement of those k by branch and bound. With the 16 most frequent letters free (`etaoinsrhlducmgy`, 2.1·10¹³ placements) the optimum is **certified optimal** (9.3 M nodes, 135 s); with 14 free, in 14 s. With 18 free the search did not finish in 10 minutes (it found nothing better; the larger `prose` trigram table weakens the bound compared with Phase 2, where 18 letters certified on `english`). The whole board cannot be certified: the reward terms make the full lower bound negative.

**Weights.** `gecko sweep` re-optimized from the optimum under 104 weight vectors: each weight dropped, halved and doubled, and 40 random log-normal draws (σ 0.5, each weight within ×0.37–×2.7). 84 of the 104 optima are in the optimum's family (within 10 keys); the median optimum moves 2 keys and loses 0.014 score points against re-optimizing. The weights that move it most are SFB (dropping it moves 28 keys), outward rolls (dropping moves 27), Engram comfort (dropping moves 22) and doubling inward rolls (25 keys). So the layout is stable under weight noise, but its shape follows from three modelling choices: how much SFB costs, how much outward rolls are worth against inward ones, and how much measured comfort counts.

![Weight-sweep consensus: share of the 104 perturbed optima (mirror-aligned) that put the same letter on each key. Darker = more stable. The home rows and the vowel block are nearly fixed; `w b f` and the centre columns move most.](assets/consensus.pdf)

**Text.** Each English source was optimized on its own (10 seeds each, all agreeing). The component corpora give exactly the same optima as their older 33-symbol counterparts (`prose-web` = `shai`, `prose-chat` = `reddit`, `prose-books` = `english`), so punctuation and case do not move the letters. The sources disagree with one another, though:

```
web    b g d l x  j y o u .     chat   c b d l x  j y o u q     books  w g n c v  z u o , q
       c s t n w  p h e i a            g s t n v  p h a i e            d t r s f  y h e i a
       f v m r q  z k ' / ,            w f m r z  ' k . , /            p b l m j  x k / ' .
```

Web and chat share the left-hand consonant block and the right-hand vowels with the `prose` optimum. Books, with dialogue and long words, gives a different family (`d t r s` home row, vowels `h e i a` kept), the same one the 24 October ranking reported for `english`.

![Each source's optimum (rows) scored on every source (columns), as % above the best optimum on that source. The `prose` optimum is within 3.3 % everywhere; each single-source optimum costs 5–19 % on the others. The best community layouts with the same symbols (bottom) are 28–107 % above.](assets/corpora.pdf)

The `prose` optimum is the robust choice: at most 3.3 % above the best layout on any source, while each single-source optimum loses 5–19 % on some other source. On every source it is still far ahead of every community layout (Graphite +28–48 %). So "which English" matters more than seeds or weights, and a mixed corpus is worth its trouble.

## Against the layouts people use

| layout | set | score | gap | rank (noise) | E01 time | Engram | SFB % | alt. % | rolls in/out % |
|---|---|---:|---:|---|---:|---:|---:|---:|---|
| *GECKO optimum* | apos | *9.52* | – | *3–4* | *6.74* | *3.98* | *1.18* | *70* | *12/31* |
| *GECKO optimum* | qwerty | *9.14* | – | *1–1* | *6.71* | *3.96* | *1.14* | *70* | *12/31* |
| Gallium | semi | 11.60 | +22 % | 2–5 | 6.78 | 5.22 | 0.92 | 68 | 20/25 |
| Graphite | apos | 12.43 | +31 % | 5–8 | 6.84 | 5.29 | 1.23 | 68 | 21/24 |
| Galite | qwerty | 11.94 | +31 % | 4–7 | 6.81 | 5.21 | 1.20 | 68 | 21/24 |
| Dhorf | semi | 12.71 | +34 % | 4–7 | 7.13 | 5.40 | 0.70 | 67 | 21/22 |
| Focal | semi | 13.46 | +42 % | 6–8 | 7.28 | 5.43 | 0.80 | 66 | 23/23 |
| Hands Down (ref) | apos | 15.76 | +66 % | 9–11 | 7.50 | 5.32 | 1.13 | 67 | 23/21 |
| Sturdy | semi | 17.00 | +79 % | 9–12 | 7.58 | 6.42 | 0.91 | 63 | 23/26 |
| Semimak-JQ | apos | 17.32 | +82 % | 10–13 | 7.36 | 5.89 | 0.87 | 66 | 20/22 |
| Canary | qwerty | 18.12 | +98 % | 10–14 | 7.88 | 6.98 | 1.15 | 61 | 23/25 |
| MTGAP | semi | 18.82 | +98 % | 9–15 | 7.59 | 5.24 | 1.40 | 65 | 29/15 |
| APTv3 | apos | 19.77 | +108 % | 12–15 | 7.63 | 6.60 | 1.23 | 62 | 33/14 |
| Dvorak | semi | 21.92 | +131 % | 11–17 | 7.41 | 6.21 | 2.79 | 70 | 25/14 |
| Halmak | qwerty | 25.71 | +181 % | 16–18 | 8.14 | 6.28 | 2.96 | 68 | 20/18 |
| Colemak-DH | qwerty | 25.97 | +184 % | 15–17 | 8.05 | 8.39 | 1.48 | 58 | 25/21 |
| Colemak | qwerty | 27.28 | +199 % | 16–18 | 8.11 | 8.69 | 1.48 | 58 | 25/21 |
| Workman | qwerty | 31.04 | +240 % | 19–19 | 9.08 | 8.45 | 3.07 | 55 | 24/20 |
| QWERTY | qwerty | 65.12 | +613 % | 20–20 | 13.15 | 12.49 | 6.57 | 53 | 21/17 |

Score is `default` on `prose`, lower is better. The gap compares each layout with the GECKO optimum over its own four punctuation keys (qwerty 9.138, apos 9.521, semi 9.485): a layout with `;` instead of `'` scores lower because `;` is rare, not because it is better. Rank (noise) is the 95 % range of the rank among these 20 layouts (the three GECKO set optima included) over 2,000 random weight draws (σ 0.5). E01 is GECKO's fitted typing-time model (% of a median keystroke interval; not weighted in the profile). Same-finger skipgrams and the scores under other profiles are in `bench/english-layout-2026-10-24/data.json`.

![Gap to the GECKO optimum with the same four punctuation keys (log scale above 100 %). Labels give the score and the rank range under weight noise.](assets/ranking.pdf)

**Reading the comparison.**

- **The modern families are close; the classics are not.** Gallium, Graphite, Galite and Dhorf are 22–34 % above the optimum in score but within 0.1–0.4 points of it in predicted typing time (E01 6.78–7.13 vs 6.74). Colemak, Workman and Dvorak are 130–240 % above and 0.7–2.3 points slower; QWERTY is in a class of its own.
- **The ranking of community layouts matches the 24 October report** on the old `english` corpus: Gallium, Galite, Graphite and Dhorf lead, Sturdy and Canary lose ground on comfort, and the inward-roll layouts (APTv3, MTGAP) fall behind. The new corpus changes the ranks of only a few adjacent pairs.
- **Under other profiles.** Under `balanced` (the previous default) the optimum (qwerty set, 13.99) and Gallium (14.09) are a tie, and both beat the rest. Under `engram` (comfort only) the optimum leads by more. Under `sfb` (SFB-first), Dhorf (15.7), Semimak-JQ (16.1) and Gallium (16.6) beat the optimum (20.8): if same-finger bigrams cost much more than the timing data says, the low-SFB community layouts are better.
- **Weight noise.** Gallium's rank interval (2–5) overlaps the apos optimum's (3–4): under some weight draws Gallium beats it. The optimum with the qwerty set ranks first in every draw.

**Independent timing judges.**

GECKO's E01 is fitted to the same data as the profile. The cost-function study also built empirical judges from the other half of the typists: mean measured pair times, shrunk toward a structural model, with QWERTY practice controlled either by pair frequency (J_B) or by pair and letter frequency (J_B+l).

![Two held-out timing judges against measured comfort. With pair-frequency control (left) the optimum is 2.5 % faster than QWERTY but 1 % slower than Gallium and Dhorf; with letter-frequency control (right) it is 2.2 % slower than QWERTY. The two controls disagree by more than the spread among good layouts.](assets/judges.pdf)

The judges disagree with each other. J_B puts every good layout 2–3.6 % faster than QWERTY and the optimum among the slower good layouts. J_B+l puts QWERTY near the middle and the optimum second-slowest. The cost-function study traced this to practice: these typists all learned QWERTY, so a pair's measured time mixes motor cost with how often the typist has typed that exact pair. A judge built from QWERTY typists cannot separate layouts whose differences are this small. **We therefore do not claim the optimum is faster than Gallium.** We claim it is better under the evidence-fitted cost, and that its predicted speed is equal to the best community layouts'. Testing the difference needs typists who learned the layouts (W2.4 and W5 of the research plan).

## Limits

- **"Best" is relative to the cost function.** The optimum's lead rests on Engram comfort (61 people, 12 keys per hand, mirrored) and on the profile's preference for outward rolls. If inward rolls feel better, as community designers believe, or SFB costs more than the timing data says, Gallium and Dhorf are as good or better.
- **No speed claim.** Predicted time separates the top layouts by under 0.4 points (% of a keystroke interval); the empirical judges cannot separate them (above).
- **30 keys only.** Number row, Shift pairs and the right-hand punctuation keys are not modelled. A full-board study needs a standard-board geometry (research plan W0).
- **Corpus.** `prose` is 50 % one web sample; web and book English give different optima. A held-out source (email, news) would test the mix further.
- **Learning cost** is not modelled. The optimum shares few keys with QWERTY (no shortcut keys kept: `z x c v` are scattered).

## Reproduce

```
cd bench/english-layout-2026-10-24
./grid.sh 2> grid.log          # 60 + 60 BLS runs (prose x 3 sets x 20 seeds; 6 sources x 10)
./post.sh                      # sweep, certify, gather.py -> data.json
gecko verify run:20261025-034807-bls-04623218
MPLCONFIGDIR=/tmp/gecko-mpl python3 -I docs/papers/english-layout/figures.py
python3 -I docs/papers/english-layout/build.py
```
