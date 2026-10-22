# Co-Designing Keyboard Layout and Board Geometry with a Hand Model

*Skeleton. Topics [T6, T7, T12](../../topics.md). Status: blocked on #54 (bilevel), #59, #66, #103.*

## Abstract (draft)
Layout and physical board are usually designed separately. We couple a parametric board model, a
finger IK hand model (reach, press angle, wrist posture), and the same fast layout scorer, and
search both jointly (outer CMA-ES over board parameters, inner layout search). [TODO: ablation of
board features; joint vs sequential optimization gain; robustness across a hand-size range.]

## Research questions
RQ1 Which board features give most of the benefit (column stagger, splay, split, thumb arc, tent)?
RQ2 Does joint optimization beat optimizing layout on the best stock board, and beat sequential?
RQ3 How sensitive is the design to hand size; what does a minimax design cost?
RQ4 Does the best layout change with the board?

## Hypotheses
H1 a few features account for most of the gain. H2 joint beats sequential by a measurable margin.
H3 a minimax design costs little on the median hand.

## Method
- Cost tables rebuilt incrementally per moved key (0.64 ms on ANSI-30; `context/objective.md`).
- Bilevel search #54: CMA-ES (IPOP, box bounds) outside; SA/BLS inside with warm start.
- Ablation: ortho-30 → add features singly, re-optimizing the layout each time.
- Stock boards: ansi-30, corne-36, sweep-34, lily58, kyria, ergodox, dactyl-manuform.
- Hand: profile `[hand]` presets/dims and `[hand.body]` arms; validate with the H5 suite.
- Collision: SAT 2D now, GJK 3D (#66); manufacturability (#73) as a constraint.

## Figures
Ablation waterfall · layout × board table · before/after board overlay · regret matrix over hand
sizes · morphing board with hand size.

## Threats
Hand-model parameters come from literature (see `research/OPEN-QUESTIONS.md`); costs are
predicted effort, not measured comfort. Ideally pair with a build-and-test of one printed board.
