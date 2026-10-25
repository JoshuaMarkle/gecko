# What layout designers value: inverse optimization of 17 layouts, and the case for inward rolls

*Experiment report · 25 October 2026 · GECKO commit 11d77b2 + this study · profile `default` · ansi-30 · `prose` corpus. Data and scripts: `bench/inverse-fit-2026-10-25/`.*

## Abstract

GECKO's optimizer answers "which layout is best under these weights". This study asks the inverse: **under which weights is a given layout best?** A new tool, `gecko fit-layout`, fits each layout its own weights. It finds the smallest change of the default profile, under a log-normal prior, that makes the layout beat its swaps and every layout a search finds, then searches again under the new weights until nothing better turns up. Each layout gets a parameter set that can score any other layout, and a test of whether it is optimal under its own preferences. We fit the 17 layouts of the English study and GECKO's optimum. **No community layout is optimal under any weights near the default.** The best come close: Galite, Graphite, Dhorf and Gallium are within 0.5–1.2 % of optimal under their fitted weights, while the classics stay 6–300 % away. The designers' revealed preferences agree with one another and differ from GECKO's default in one consistent direction. They weight Engram's measured comfort 7× less (×0.14 in 11 of 11 layouts), hand alternation, pinky–ring pairs, stretches and row jumps 3–4× less, and same-finger skipgrams 5× more. **Inward rolls** come out weighted ×1.4 against ×1.1 for outward rolls (9 of 11 modern layouts up), a real but modest preference. Nothing in the measured data explains it. In the Aalto timing an inward roll is 4.7 % *slower* than the same keys rolled outward, all of it from rolls that skip a finger or involve the pinky. In Engram's comfort choices inward and outward tie (P = 0.51). Weighting inward rolls up gives community-like roll levels, but the layouts get slower on held-out timing and less comfortable. The one place the data leans inward is adjacent fingers. Gallium is the consensus layout: within 1.2–3 % of the best under the fitted weights of five of the six other leading modern layouts.

## Question and setup

**Questions.** (1) Under which weights is each well-known layout optimal, and is it optimal under any nearby weights at all? (2) What do the designers' choices say they value, compared with GECKO's evidence-fitted default? (3) Community designers prefer inward rolls; GECKO's default pays outward rolls 2.3× more per spread. Is there a reason in the data?

**Inverse fit** (`gecko fit-layout`, `context/objective.md` Inverse fit). Each weighted metric's weight is `w₀·e^θ` (w₀ from `default`), with a log-normal prior θ ~ N(0, σ²). The loss adds a squared hinge `½·max(0, Δ/T)²` for each competitor the layout loses to (Δ = score of the layout minus score of the competitor). The fit is the smallest change of weights that makes the layout win. Competitors already beaten add nothing, so an optimum of `default` keeps the default weights exactly. Multipliers are centred (geometric mean ×1), because only weight ratios change which layout wins. Competitors come from cutting planes: the layout's 600 tightest swaps, then rounds of breakout local search (2 seeds × 4 M evaluations from the layout, random restarts) under the current fit. Each round adds every layout that beats it, until a round finds nothing or 12 rounds pass. T is 5 % of the median swap |Δ|. A Laplace approximation gives a 95 % interval per multiplier. When a weight's posterior sd stays above 0.8 σ, the layout says little about that weight.

**Settings.** ansi-30, `prose`, `default`. Prior σ = 0.5 (headline; each weight within ×0.37–×2.7 a priori, as in the weight-noise studies) and σ = 1.0 (sensitivity). The bound on each multiplier is ×100, because `default` gives skipgrams and redirects weights near zero (0.04 and 0.006) and the fits need room to raise them. **Layouts:** the six classics, eleven modern layouts and GECKO's three `prose` optima from the English study, with the same key strings.

**Validation.** A planted test (`crates/gecko-core/tests/inverse.rs`): a layout optimized with inward rolls weighted ×4 is fitted back from `default`. The fit raises the inward-roll multiplier (> ×1.3) above the outward one. GECKO's own optima come out self-consistent, with all multipliers at ×1.00.

## Is any layout optimal for its own preferences?

![Fitted multipliers per layout (rows) and weight (columns), log colour scale: orange = the layout implies the designer cared more than `default`, blue = less. A dot = the layout says little about that weight (posterior sd > 0.8 σ). The number after each name is the gap that remains under its fitted weights. Columns are ordered by the modern layouts' mean change.](assets/multipliers.pdf)

| layout | gap under `default` | gap under its fitted weights | keys to that better layout | log distance |
|---|---:|---:|---:|---:|
| Galite | 30.7 % | 0.53 % | 13 | 0.84 |
| Graphite | 30.6 % | 0.56 % | 9 | 0.85 |
| Dhorf | 34.0 % | 0.94 %† | 16 | 0.59 |
| Gallium | 22.3 % | 1.18 %† | 8 | 0.37 |
| Semimak-JQ | 81.9 % | 3.96 % | 23 | 1.76 |
| Sturdy | 79.2 % | 4.17 % | 17 | 1.90 |
| MTGAP | 98.4 % | 5.64 % | 21 | 1.76 |
| Canary | 98.3 % | 14.4 % | 22 | 1.56 |
| APTv3 | 107.6 % | 19.9 % | 23 | 1.65 |
| Focal | 41.9 % | 27.5 % (σ 1: 4.3 %) | 20 | 1.36 |
| Hands Down (ref) | 65.5 % | 96.2 % | 25 | 1.48 |
| Colemak-DH | 184.1 % | 5.67 % | 22 | 1.70 |
| Dvorak | 131.1 % | 8.74 % | 25 | 1.86 |
| Workman | 239.7 % | 10.4 % | 25 | 1.91 |
| Colemak | 198.6 % | 17.8 % | 26 | 1.77 |
| Halmak | 181.4 % | 25.9 % | 24 | 2.12 |
| QWERTY | 612.6 % | 299.9 % | 29 | 1.74 |
| GECKO optimum (all three sets) | 0 | 0 (self-consistent) | 0 | 0.00 |

Gap: % above the best layout the search found, under that weight vector. Log distance: RMS of the log multipliers (0.37 ≈ ×1.45 per weight on average). †Converged: the last round found no new competitor, yet a better layout remains, so no weights near `default` make the layout optimal. The other community fits were still adding competitors after 12 rounds, so their gaps are upper bounds.

- **None of the 17 is self-consistent**, but the four leading modern layouts come within about 1 %. Their designers optimized something close to GECKO's metric space with different weights. Gallium needs the smallest change of weights (log distance 0.37), so its design is closest to GECKO's default.
- **The classics cannot be explained** by any nearby weights: 6–300 % remain. Colemak, Workman and QWERTY were designed around QWERTY similarity and other aims that this profile does not weight (`familiar` is 0).
- **Hands Down** did not fit: every round found far better layouts under the new weights, and the last gap (96 %) is larger than under `default`. Its ansi-30 version drops the thumb letter the design is built around. We do not interpret its weights.

## What the designers value

Geometric-mean multiplier over the eleven modern layouts (σ = 0.5; σ = 1.0 in brackets), with how many layouts move each weight up:

| weight | `default` | modern ×(mean) | up in | reading |
|---|---:|---:|---:|---|
| Engram comfort | 2.08 | ×0.14 (×0.12) | 0/11 | the largest disagreement |
| pinky–ring | 1.18 | ×0.25 (×0.24) | 1/11 | |
| hand alternation | 1.00 | ×0.28 (×0.26) | 0/11 | same-hand flow is fine |
| stretch | 0.50 | ×0.33 (×0.31) | 1/11 | |
| row jump | 0.29 | ×0.34 (×0.30) | 1/11 | |
| SFB | 2.87 | ×0.54 (×0.53) | 3/11 | still a large weight |
| key effort | 0.32 | ×1.69 (×1.71) | 9/11 | |
| home row | 0.04 | ×2.05 (×2.42) | 11/11 | |
| trigram alternates | 0.06 | ×4.07 (×4.28) | 11/11 | |
| SFS (skipgrams) | 0.04 | ×4.97 (×5.15) | 11/11 | Graphite ×19, Semimak ×51 |
| redirects | 0.006 | ×8.83 (×12.6) | 11/11 | near-zero base |
| **rolls in** | 0.31 | **×1.42 (×1.40)** | 9/11 | |
| rolls out | 0.72 | ×1.08 (×1.08) | 6/11 | |

The preferences hold under both priors and point the same way in nearly every layout. In GECKO's terms, community designers spend measured comfort, alternation and some SFB on fewer skipgrams and redirects and more home-row use. Those are the statistics community analyzers report first. The default profile found no timing or comfort cost for skipgrams and weights Engram heavily, which is why GECKO's optimum differs from all of them.

## Inward rolls

![Left: an inward roll's pair time minus the same two keys rolled outward (Aalto, fast QWERTY typists, practice-adjusted; 51 key pairs). Middle: Engram's measured comfort choices, P(inward preferred over its reverse) (30 measured left-hand pairs). Right: each layout's fitted rolls-in ÷ rolls-out multiplier (blue: modern, grey: classic).](assets/rolls.pdf)

**What the designers imply.** 16 of 17 named layouts weight inward rolls relative to outward ones more than `default` does: ×1.2–×2.4, Dhorf the exception (×0.8). The fitted ratio correlates with the layout's own ratio of inward to outward rolls (r = 0.57 over the modern layouts). In absolute terms the modern fits still pay outward rolls more per spread (about 0.43 against 0.77 after the multipliers). So the community preference, seen through GECKO's metrics, is "less outward-tilted than the default", not "inward over outward".

**Timing does not support it.** For each same-hand key pair, the pair typed inward is compared with the same two keys typed outward. The measure is the mean log interval relative to the typist's median, after removing the practice effect (slope on log pair count). Inward is **4.7 % slower** (95 % CI +0.2 to +10.1 %; inward faster in 15 of 51 pairs). The difference comes from rolls that skip a finger (+10 %, CI +4.8 to +16 %) or involve the pinky (+11 %). Adjacent-finger rolls are equal (+0.7 %, CI −5 to +8 %). The keystroke after a roll is no faster for inward rolls either: +4.6 % against its pair mean, outward +2.9 %. These typists learned QWERTY, so practice can still confound direction. The practice slope controls only for how often each pair occurs.

**Comfort does not support it.** In Engram's 4,490 pairwise choices (61 people), an inward pair beats its reverse with P = 0.51 (CI 0.45–0.57). Adjacent-finger inward rolls lean comfortable (P = 0.54) and skip-finger ones lean uncomfortable (P = 0.46); neither is significant. Engram's *formula*, which many analyzers copy, scores inward 1.0 and outward 0.78. Its own measured choices do not show that gap.

![The price of inward rolls: the optimum under `default` with the rolls-in weight ×1 to ×16 (best of 4 BLS seeds × 25 M). Left: its score under plain `default`. Middle: its trigram rolls; grey band = the community layouts' rolls in (20–33 %). Right: predicted time from GECKO's fitted bigram model (blue) and the held-out timing judge J_B (orange, % vs QWERTY; lower = faster).](assets/price.pdf)

**What inward rolls cost.** Weighting rolls in ×2 already gives community-like rolls (24 % in, 20 % out) for +9 % on `default`; ×4 gives 34 / 10 % for +26 %. The fitted timing model calls these layouts faster (6.74 → 6.42), but only through lower SFB (1.18 → 0.99 %): the model has no roll term. The held-out judge, built from timing the profile never saw, calls them slower (J_B −12.2 % → −7.5 % and −7.9 % vs QWERTY), and Engram discomfort rises (3.98 → 4.95).

**So why do designers prefer inward rolls?** Not, as far as these data show, because inward rolls are faster or more comfortable in general. The likely reasons are conventions and a narrower effect. Most community analyzers weight inward rolls above outward ones by assumption (Oxeylyzer's trigram weights, Engram's formula; `research/SYNTHESIS.md` §2). A designer who tunes against such an analyzer reveals its weights. The one place the data lean inward is adjacent fingers, where comfort favours inward slightly and time is equal. Inward rolls that skip a finger or reach the pinky are measurably slower. That suggests a split metric (adjacent vs. skip-finger rolls) rather than a higher inward weight. Both datasets come from QWERTY typists on QWERTY-typed text, and a roll's feel to someone who learned the layout is exactly what they cannot measure.

## One layout under another's preferences

![Each layout (rows) scored under each layout's fitted weights (columns), as % above the best layout known under those weights. Read a column as "what this designer would think of the others"; read a row as "how this layout fares under each designer's preferences". Focal's column comes from a fit that did not converge at σ = 0.5.](assets/cross.pdf)

- **Gallium is the consensus layout**: within 1.2–3.0 % of the best under the fitted weights of Galite, Graphite, Dhorf, Semimak-JQ and Sturdy, and under its own.
- **Galite and Graphite share preferences** (log distance 0.08); Galite beats Graphite under Graphite's own weights.
- **GECKO's optimum** is 4–8 % behind under the Graphite, Galite, Gallium and Dhorf fits and 17–23 % behind under Semimak and Sturdy, whose fits weight skipgrams heavily. Under `default` the community layouts are 22–108 % behind GECKO's optimum. Each design is good under its own preferences and fair under its neighbours'.
- **Preferences are layout-specific**: a modern layout scored under another modern layout's fit is a median 18 % above that fit's best, against 4 % under its own.

## Limits

- **Only weights are fitted.** Table parameters (SFB distance shape, row offsets, finger targets) stay at `default`'s, and a metric the profile does not have cannot be discovered. Revealed preferences are read through GECKO's metric space.
- **The search is not exact**, and most community fits were still adding competitors after 12 rounds; their gaps are upper bounds. "Not optimal under nearby weights" is proven only in the converged cases (Gallium, Dhorf), and only up to what the search finds.
- **Identifiability.** A layout pins down only the weights its competitors trade on; dotted cells in the first figure fall back to the prior. Weights near zero in `default` (skipgrams, redirects) need large multipliers, and their fitted values carry more prior than data.
- **Roll evidence is QWERTY evidence.** Timing comes from QWERTY typists and is practice-adjusted only through pair counts; comfort covers 12 left-hand keys. Neither measures rolls on a layout the typist learned.

## Next steps

1. **Split rolls by adjacency.** Separate adjacent-finger rolls from finger-skipping and pinky rolls (in and out), fit their weights on the Aalto data, and re-run the English optimum.
2. **Revisit the Engram weight.** It is the largest disagreement with every designer: 2.08 in `default`, implied ×0.14. Its comfort data cover 12 keys (`research/OPEN-QUESTIONS.md`). A 30-key comfort study (research plan W2.1) would settle whether GECKO or the designers are right.
3. **Use fitted profiles as judges.** `--profiles` writes each fit as a profile, so a new layout can be scored with Gallium's or Graphite's preferences beside `default`.
4. **Fit more layouts** (the cmini bundle) to map the community's preference space and see which designers cluster.

## Reproduce

```
cd bench/inverse-fit-2026-10-25
python3 -I roll_timing.py      # needs ~/projects/gecko-workspace/costfit/timing30.npz
./grid.sh                      # 20 layouts x sigma 0.5, 1.0 -> fits-sigma*.json, profiles-sigma*/
./inroll_price.sh              # 5 rolls-in weights x 4 seeds -> price/
python3 -I analyze.py && python3 -I judges.py
MPLCONFIGDIR=/tmp/gecko-mpl python3 -I ../../docs/papers/inverse-fit/figures.py
python3 -I ../../docs/papers/inverse-fit/build.py
```
