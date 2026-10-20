# Hand models: anthropometry, kinematics, landmarks

## Sources

- Buchholz, Armstrong, Goldstein, "Anthropometric data for describing the kinematics of the
  human hand", *Ergonomics* 35(3), 1992. Segment lengths and joint centres as fractions of hand
  length (and breadth). Full text paywalled; ratios not verified here.
- MediaPipe Hands: 21 landmarks per hand from a single image
  (https://github.com/google-ai-edge/mediapipe, `hands.md`). Used by Cosmos.
- MANO (Romero, Tzionas, Black, SIGGRAPH Asia 2017, https://mano.is.tue.mpg.de): learned
  parametric hand mesh, 16 joints, shape PCA; research license.
- MyoSuite MyoHand (arXiv:2205.13600): musculoskeletal model in MuJoCo.
- Häger-Ross & Schieber 2000 for finger coupling (`hager-ross-finger-independence.md`).
- Cosmos IK scheme (`cosmos-keyboards.md`).

## What GECKO needs (phase 7), simplest first

1. **Kinematic chain per finger:** MCP (flex + abduct), PIP, DIP with DIP ≈ ⅔·PIP coupling
   (classic approximation). Thumb: CMC 2 DoF + MCP + IP.
2. **Segment lengths:** from a hand scan (MediaPipe) or from hand length + breadth with
   Buchholz ratios **(verify ratios from the paper before hard-coding)**.
3. **Joint limits** (typical ROM, verify per source): MCP flex ~0–90°, abduction ~±20–30°;
   PIP 0–100°; DIP 0–80°.
4. **Reach cost:** IK to the key top (Cosmos's closed-form method), cost = weighted joint
   deviation from neutral curl + press-direction misalignment + wrist offset if out of reach.
5. **Coupling:** penalize postures where adjacent fingers must be far apart in flexion
   (ring/middle), using the independence indices.

Validation checks are in `../hand-model-and-geometry.md` (home row cheapest; col-stagger beats
row-stagger; Corne/Dactyl beat flat ANSI).
