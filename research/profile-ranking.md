# Pairwise profile calibration (#35)

`gecko profile fit --target likes --out learned --seed 1 --json` learns a profile from
`data/community/cmini.json`. The original `--target E05` judge regression remains available.
The learned profile is opt-in and does not change the default objective.

Unique positively liked layouts supply preference pairs: higher likes prefers a lower
score; tied likes and zero-like layouts supply no labels. Duplicate/mirror groups retain
one representative. Sort by name, shuffle with GECKO's seeded RNG, hold out floor(N/5)
whole layouts, then compute scales and training pairs using training layouts alone. No
pair containing a held-out layout enters training. Each distinct-count pair has one vote.

Features are the scorer's signed unit-weight metric costs (including nonlinear balance
costs), standardized by training-only standard deviation. Constant, information-only and
E* comfort metrics are excluded. Minimize mean `log(1 + exp(-w·(worse−better)))` plus
`0.01/2 · ||w||²` with nonnegative projected gradient descent, a global Lipschitz step
and at most 800 iterations. The exact f32 exported weights/scales are evaluated. Prediction
ties get half credit; absent test pairs report null. Stable softplus avoids overflow.

## Results (2026-10-21)

ansi-30 / english / default, seed 1: 303 liked groups → 243 training / 60 held out.
19,293 training pairs, 1,324 held-out pairs. Training accuracy **55.42%**, held-out
**53.78%**, original profile **51.96%**, independent Engram E05 **49.40%** on those same
held-out pairs. Mean training logistic loss 0.67424 (zero weights: 0.69315).

Exact JSON report, layout identities, input content hashes and fitting options live in
`bench/profile-fit-2026-10-21/fit.json`; final weights are `data/profiles/learned.toml`.
Re-run the command above with the same inputs and seed to reproduce both. Changing
held-out likes leaves weights/scales/training loss unchanged (regression test).

The gain is small. Likes measure popularity and exposure as well as preference; correlated
pairs are not independent observations, so pair count is not a confidence interval.
Layouts are evaluated on the selected geometry with its standard finger assignments:
community angle-mod fingerings await #47. E05 is an independent comparison, not a held-out
training label. This model is useful as a separate ranking experiment, not evidence of
better comfort or typing speed. More seeds and user preference data are needed.
