# A General Cost Function for Keyboard Layouts: Sensitivity, Calibration and Validity

*Skeleton. Focus paper 2. Topics [T1, T5, T8, T11](../topics.md). Status: scorer, profiles, judges and
the pairwise fit exist; sweeps, timing model and a user study do not.*

## Abstract (draft)
Layout tools disagree because their cost functions disagree. We define a general cost function: a
weighted sum of geometry-derived metrics over mono, bigram, skipgram and trigram statistics plus a
few non-linear balance terms, parameterized by a board, a corpus and a profile, and we ask what
makes such a function good. We test it three ways: **sensitivity** (how far the optimum moves with
the weights), **validity** (agreement with independent judges: Engram comfort, keystroke timing,
community preference) and **robustness** (transfer across corpora and boards). [TODO: key
findings.] We give a recipe for a defensible default profile and report where the evidence is weak.

## Research questions
1. RQ1 *Expressiveness*: which metric families are needed (ablation: drop a family, measure what
   changes in rankings and optima)?
2. RQ2 *Sensitivity*: how many optimum families does the plausible weight space contain; which
   weights decide between them (Sobol indices)?
3. RQ3 *Validity*: how well does the score predict (a) IKI from 136M keystrokes, (b) Engram
   pairwise comfort, (c) community likes, (d) a small user study?
4. RQ4 *Goodhart*: does more search effort widen the gap between proxy and judge?
5. RQ5 *Transfer*: does a profile fitted on one corpus/board hold on others (english, shai, reddit,
   mt-10k, french, code; ANSI-30, Corne)?

## Hypotheses (not results)
H1 a few families explain most ranking changes. H2 two or three weights dominate the optimum.
H3 geometry-based metrics transfer better than trigram metrics. H4 learned pair-feature weights
beat hand-set weights on held-out typists, but only modestly. H5 the proxy-vs-judge gap grows with
search effort unless the judge is part of the objective.

## Method
- Definition: `context/objective.md` (folding, spread-normalized weights, non-linear terms).
- Sensitivity: Latin-hypercube over log-scaled spread-normalized weights; archive once with
  `gecko explore`, re-rank under each vector (`gecko atlas -p`); embedding by finger distance.
- Validity: IKI regression on geometric pair features, split by typist; pairwise logistic fit
  (`gecko profile fit --target likes`) with bootstrap CIs; judges never used to train and test the
  same claim.
- Goodhart: budget grid 1 s … 10 min; optimize profile A, score on B, C and E05.
- Pre-register the analysis (cluster definition, cutoffs) before running.

## Existing evidence
Held-out pairwise accuracy 53.78 % (learned) vs 51.96 % (default) vs 49.40 % (Engram E05)
(`research/experiments/2026-10-21-profile-ranking.md`; no CIs yet). An earlier optimum (8.396)
ranked last on E05 after the profile changed; B11 at weight 1 re-sorts Halmak and Colemak
(`context/objective.md`).

## Figures
F1 metric-family ablation · F2 optimum embedding by weight vector · F3 Sobol bars · F4 regret
matrix · F5 validity: predicted vs observed (IKI heatmap by finger pair; accuracy with CIs) ·
F6 proxy-vs-judge along search · F7 transfer matrix · F8 consensus keyboard.

## Threats
Timing data is from QWERTY typists (learn geometric, never per-letter costs); likes measure
popularity; E05 comes from one study; hand model parameters from literature; no user study yet.

## Needed
`gecko sweep` or script; E01 timing model (#31, #61); bootstrap CIs; ideally a 10–20 person study.
