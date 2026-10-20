# research/sources/

One file per source (or tight group of sources). Each file: what it is, key ideas, numbers
worth keeping, how to use it in GECKO, where the code/data is. Start from
[`../SYNTHESIS.md`](../SYNTHESIS.md); open a source file only when you implement its idea.

Read 2026-10-20. Repos were cloned and read; a few sites were blocked from the research
environment, so some details come from abstracts or mirrors and are marked **(verify)**.

## Analyzers and optimizers

| File | Source | Take |
|---|---|---|
| [oxeylyzer.md](oxeylyzer.md) | Oxeylyzer 1 + 2 (Rust) | Geometric stretch formula for any board; trigram weights; depth-N greedy |
| [kuehlmak.md](kuehlmak.md) | Kühlmak (Rust) | Full same-hand bigram classes; disjoint bigrams; threshold annealing with adaptive fidelity |
| [klo-dariogoetz.md](klo-dariogoetz.md) | Keyboard Layout Optimizer (Rust) | Finger-pair movement table; finger-length scissor rule; modifier n-gram expansion; key cost matrices |
| [genkey-semimak.md](genkey-semimak.md) | genkey + Semimak blog | Finger speed with per-finger KPS; skipgram decay 1/2^g; personal dexterity |
| [carpalx.md](carpalx.md) | carpalx | Triad effort model with default parameters |
| [cmini.md](cmini.md) | cmini bot | ~4100 community layouts + many corpora; finger-combo trigram table |
| [engram-preference-study.md](engram-preference-study.md) | Engram study + repos | Crowdsourced comfort → Bradley–Terry → reusable pair-score CSV |
| [halmak-keyboard-genetics.md](halmak-keyboard-genetics.md) | keyboard-genetics | What a thin objective produces |

## Community knowledge

| File | Source | Take |
|---|---|---|
| [keyboard-layouts-doc.md](keyboard-layouts-doc.md) | ec0vid's Layouts Doc | Shared metric definitions; distance examples; design rules |
| [community-design-notes.md](community-design-notes.md) | Hands Down, Gallium, Graphite | No single stat; corpus dependence; alt-fingering; thumb letters |
| [corpora.md](corpora.md) | Corpora shipped by analyzers | Which corpora to add for comparable numbers |

## Human data and biomechanics

| File | Source | Take |
|---|---|---|
| [dhakal-136m-keystrokes.md](dhakal-136m-keystrokes.md) | 136M keystrokes, CHI 2018 | IKI by hand/alternation; rollover ≈ 50 % for fast typists |
| [feit-how-we-type.md](feit-how-we-type.md) | How We Type, CHI 2016 | Consistent finger mapping, preparation, low hand motion predict speed |
| [hager-ross-finger-independence.md](hager-ross-finger-independence.md) | Finger independence, 2000 | Ring least independent, then middle |
| [split-keyboard-ergonomics.md](split-keyboard-ergonomics.md) | Rempel, Marklin, AOTA | Slope 0 to −4°, opening ≈ 15° |
| [hand-models.md](hand-models.md) | Buchholz, MediaPipe, MANO, MyoSuite | Minimal kinematic hand model for reach costs |
| [fitts-and-movement-models.md](fitts-and-movement-models.md) | Zhai et al., FFitts | Fitts only for same-finger travel and thumbs |

## Optimization and ML

| File | Source | Take |
|---|---|---|
| [qap-metaheuristics.md](qap-metaheuristics.md) | Taillard, Benlic & Hao, Stützle | Robust tabu, BLS, BMA memetic |
| [dellamico-single-finger-qap.md](dellamico-single-finger-qap.md) | Dell'Amico et al. 2009 | QAP form, delta formulas, empty slots, ant system wins |
| [integer-programming-bounds.md](integer-programming-bounds.md) | Karrenbauer, Oulasvirta, Feit | Exact bounds to certify optima; AZERTY standard |
| [search-methods-general.md](search-methods-general.md) | CMA-ES, NSGA-II, MAP-Elites, PT | Geometry search and diversity |
| [ml-approaches.md](ml-approaches.md) | Surrogates, preference learning, simulated typists | Learn cost tables, keep search classical |

## Geometry generators

| File | Source | Take |
|---|---|---|
| [cosmos-keyboards.md](cosmos-keyboards.md) | Cosmos | Closed-form finger IK; hand scans; hand-fit stagger |
| [ergogen-dactyl.md](ergogen-dactyl.md) | ergogen, Dactyl | Parameter set for parametric geometry search |
