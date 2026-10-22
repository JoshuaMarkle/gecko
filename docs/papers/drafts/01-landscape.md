# The Landscape of Keyboard Layouts: Local-Optima Networks and Quality Diversity

*Skeleton. Focus paper 1. Topics [T3, T4](../topics.md). Status: tooling complete (`explore`, `atlas`, `qd`, `nsga2`); runs pending.*

## Abstract (draft)
We characterize the search space of keyboard layouts (30! ≈ 2.65 × 10³² placements) with a
random-walk correlation analysis, a local-optima network sampled by breakout local search, and
a MAP-Elites map over ergonomic behaviours. [TODO: fitness-distance correlation value; number of
funnels; fraction of local optima within 1 % of best; Pareto front shape.] Popular
community layouts [TODO: lie on / behind] the front. We release the atlases and a viewer.

## Research questions
RQ1 Big-valley or multi-funnel? (FDC, funnel decomposition.) RQ2 How many distinct near-optimal
layouts exist and how different are they (Hamming, finger distance, swap distance)? RQ3 What do
you pay to get more rolls or less index load (QD, Pareto)? RQ4 Where do the ~4,100 community
layouts sit on the map?

## Hypotheses
H1 positive FDC (big valley). H2 a few funnels differing in vowel placement. H3 the number of
near-optimal optima ≫ the number of used reference layouts. H4 the roll/SFB front is short and
convex.

## Method
`gecko landscape` (random distribution, correlation length), `gecko explore` (10 min × 10 seeds
per setting), `gecko atlas` (families, consensus, funnels), `gecko qd --axes roll_ratio,index_load
--bins 16`, `gecko optimize --algo nsga2`. Canonicalize under left/right mirroring only when
provably symmetric. Community layouts from `data/community/cmini.json`, scored on the same tables.

## Existing evidence
Space statistics and distance definitions (`research/studies/search-space-visualization.md`);
exploration and QD specs (`context/algorithms/explore.md`, `map-elites.md`).

## Figures
F1 local-optima network · F2 FDC · F3 score density vs optima · F4 autocorrelation · F5 consensus
keyboard and contested slots · F6 MAP-Elites heatmap with references and community density ·
F7 Pareto fronts.

## Threats
Sampling bias of BLS (kicks define the network edges); results are for one profile (see
[paper 2](02-cost-function.md)); the descriptor choice shapes the QD map.
