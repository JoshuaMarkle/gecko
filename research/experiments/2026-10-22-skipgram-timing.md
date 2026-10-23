# Same-finger skipgrams: timing by context, finger and distance

| | |
|---|---|
| Date | 2026-10-22 |
| Question | Does any kind of same-finger skipgram (SFS) slow typing, and what would show a comfort cost? |
| Answer | No timing cost in any split. The upper bound is +0.9 % (touch) to +2.6 % (fast touch) of the third keystroke, against +55 % for an SFB. SFS weights rest on comfort, which is unmeasured. |
| Command | `python3 bench/cost-validity-2026-10-22/scripts/a14_skipgrams.py` (paths as in that README) |
| Data | Aalto 136M trigram aggregates (letter trigrams, n ≥ 200; touch 1,847 trigrams with 243 SFS, touch ≥ 70 WPM 1,334 with 188) |
| Artifacts | `bench/cost-validity-2026-10-22/skipgram_timing.json` |

Method as in the [validity study](2026-10-22-cost-function-validity.md). The outcome is the mean
log relative interval of the third keystroke, or of the whole a→c span. The model controls log
trigram frequency, the median time of bigram b→c (and a→b for the span), and the trigram class
(roll, one-hand, redirect, bad redirect vs alternation). Trigrams with an SFB or a repeated key
are left out. Effects are % slower; 95 % bootstrap CIs, 1,000 resamples.

## Results

| SFS effect (%) | touch | touch ≥ 70 WPM |
|---|---|---|
| third keystroke, all SFS | −1.1 (−3.0, +0.9) | +0.5 (−1.5, +2.6) |
| a→c span, all SFS | −0.9 (−2.4, +0.6) | +0.8 (−0.7, +2.4) |
| middle key on the other hand (T02) | −1.1 (−3.6, +1.6), n 149 | −0.1 (−2.5, +2.8), n 110 |
| middle key on the same hand | −1.2 (−4.4, +2.2), n 94 | +1.4 (−2.4, +5.1), n 78 |
| pinky or ring | −3.6 (−9.1, +1.6), n 28 | −0.6 (−5.9, +4.1), n 21 |
| middle or index | −0.9 (−2.8, +1.2), n 215 | +0.6 (−1.5, +2.8), n 167 |
| same row (sideways, e.g. f–g) | +0.7 (−3.6, +5.0), n 34 | +3.6 (−0.5, +7.5), n 28 |
| row change | −1.6 (−3.7, +0.7), n 209 | −0.3 (−2.5, +2.1), n 160 |

- **No split shows a reliable cost.** The one hint is the fast typists' same-row (lateral) SFS:
  +3.6 %, with a CI that touches 0, on 28 trigrams.
- **Scale.** An SFB adds 0.55 of a median interval. In the 2026-10-22 default profile a
  percent of SFS costs 0.12 of a percent of SFB (`S01 / scale` ÷ `B01 / scale`). Timing would
  allow at most about 0.05 (2.6 % ÷ 55 %). The rest is a comfort assumption: a finger that has
  to travel again soon may tire or strain without typing slower.

## What would measure it

Timing is the wrong instrument when typists hide the cost by moving early. A comfort measurement
would:

1. Use pairwise choice, as Engram did for bigrams. Typists type two short nonsense strings and
   pick the more comfortable. Pairs differ only in whether keys 1 and 3 share a finger, matched
   on bigram content and row changes.
2. Cover the splits above: in alternation vs same hand, pinky/ring vs middle/index, lateral vs
   vertical, each with about 20 matched pairs.
3. Recruit 20–30 touch typists on their own layout, so positions are fluent. A Bradley–Terry fit
   gives a per-split comfort cost on Engram's E05 scale.
4. Add a short fatigue block (the same SFS string repeated for 60 s, with a rating) if the
   pairwise cost is near zero.

The web UI could host it once it records keystrokes, which it does not do yet. Until such data
exists, the S01/S02 weights stay a stated assumption
([open questions](../OPEN-QUESTIONS.md)).
