# research/sources/

One file per source (or tight group of sources): what it is, key ideas, numbers worth
keeping, how GECKO uses it, where the code or data is. Start from
[`../SYNTHESIS.md`](../SYNTHESIS.md); open a source file only when you implement its idea.

To add one: copy [`../_template-source.md`](../_template-source.md), fill it in, run
`python3 -I research/scripts/research.py index`. The table below is generated from each file's
frontmatter; do not edit it by hand.

**Status:** `read` = primary source (paper or repository) read directly · `secondary` = some
details come from abstracts, mirrors or a blocked site, marked **(verify)** in the file ·
`verified` = numbers cross-checked against more than one source or reproduced. Open items from
all files are collected in [`../OPEN-QUESTIONS.md`](../OPEN-QUESTIONS.md).

<!-- index:start -->

### Analyzers and optimizers

| File | Take | Status |
|---|---|---|
| [carpalx.md](carpalx.md) | Triad effort model with default parameters (values via a mirror) | secondary |
| [cmini.md](cmini.md) | ~4100 community layouts + many corpora; finger-combo trigram table | read |
| [engram-preference-study.md](engram-preference-study.md) | Crowdsourced comfort → Bradley–Terry → reusable pair-score CSV | verified |
| [genkey-semimak.md](genkey-semimak.md) | Finger speed with per-finger KPS; skipgram decay 1/2^g; personal dexterity | read |
| [halmak-keyboard-genetics.md](halmak-keyboard-genetics.md) | What a thin objective produces | read |
| [klo-dariogoetz.md](klo-dariogoetz.md) | Finger-pair movement table; finger-length scissor rule; modifier n-gram expansion; key cost matrices | read |
| [kuehlmak.md](kuehlmak.md) | Full same-hand bigram classes; disjoint bigrams; threshold annealing with adaptive fidelity | read |
| [oxeylyzer.md](oxeylyzer.md) | Geometric stretch formula for any board; trigram weights; depth-N greedy | read |

### Community knowledge and data

| File | Take | Status |
|---|---|---|
| [community-design-notes.md](community-design-notes.md) | No single stat; corpus dependence; alt-fingering; thumb letters | read |
| [corpora.md](corpora.md) | Which corpora and timing data exist; which to add for comparable numbers | read |
| [keyboard-layouts-doc.md](keyboard-layouts-doc.md) | Shared metric definitions; distance examples; design rules | read |

### Human data and biomechanics

| File | Take | Status |
|---|---|---|
| [dhakal-136m-keystrokes.md](dhakal-136m-keystrokes.md) | IKI by hand/alternation; rollover ≈ 50 % for fast typists (data site unreachable) | secondary |
| [feit-how-we-type.md](feit-how-we-type.md) | Consistent finger mapping, preparation, low hand motion predict speed | read |
| [finger-force-and-fatigue.md](finger-force-and-fatigue.md) | Typing is low-force (≈ 10 % MVC) and shows potentiation, not fatigue, over 6 h; key force and pace matter more than which finger | secondary |
| [fitts-and-movement-models.md](fitts-and-movement-models.md) | Fitts only for same-finger travel and thumbs | secondary |
| [hager-ross-finger-independence.md](hager-ross-finger-independence.md) | Ring least independent, then middle | read |
| [hand-models.md](hand-models.md) | Segment ratios, joint ROM, DIP–PIP coupling; Buchholz, MediaPipe, MANO, MyoSuite | verified |
| [split-keyboard-ergonomics.md](split-keyboard-ergonomics.md) | Slope 0 to −4°, opening ≈ 15° | secondary |
| [thumb-biomechanics.md](thumb-biomechanics.md) | Thumb uses < 65 % of its joint range in normal use and performs best near rest; far-flexed keys near the thumb base are worst | secondary |
| [wrist-posture-and-finger-load.md](wrist-posture-and-finger-load.md) | Carpal-tunnel limits ext 33° / flex 49° / radial 22° / ulnar 15°; typing sits at ext ~20°, ulnar ~15° | secondary |

### Optimization and ML

| File | Take | Status |
|---|---|---|
| [dellamico-single-finger-qap.md](dellamico-single-finger-qap.md) | QAP form, delta formulas, empty slots, ant system wins | read |
| [integer-programming-bounds.md](integer-programming-bounds.md) | Exact bounds to certify optima; AZERTY standard (MPI pages unreachable) | secondary |
| [ml-approaches.md](ml-approaches.md) | Surrogates, preference learning, simulated typists; keep search classical | secondary |
| [qap-metaheuristics.md](qap-metaheuristics.md) | Robust tabu, BLS, BMA memetic | read |
| [search-methods-general.md](search-methods-general.md) | CMA-ES, NSGA-II, MAP-Elites, parallel tempering | secondary |

### Geometry generators

| File | Take | Status |
|---|---|---|
| [cosmos-keyboards.md](cosmos-keyboards.md) | Closed-form finger IK; hand scans; hand-fit stagger | read |
| [ergogen-dactyl.md](ergogen-dactyl.md) | Parameter set for parametric geometry search | read |

<!-- index:end -->
