# Hand models: anthropometry, kinematics, landmarks

## Sources

- Buchholz, Armstrong, Goldstein, "Anthropometric data for describing the kinematics of the
  human hand", *Ergonomics* 35(3):261–273, 1992 (PubMed 1572336). Kinematic segment lengths
  (joint centre to joint centre) as fractions of hand length. Full text paywalled; values
  below are cross-checked through secondary sources.
- Buchholz & Armstrong, "A kinematic model of the human hand to evaluate its prehensile
  capabilities", *J Biomech* 25(2):149–162, 1992, doi:10.1016/0021-9290(92)90272-3
  (open: deepblue.lib.umich.edu). Model form `r_ij = C_ij · HL`, MCP x-position `F · HL`,
  MCP z-position `G · HB`; coefficients are deferred to Buchholz's 1989 PhD dissertation.
  Trapezium axes vs 3rd metacarpal (from Cooney 1981): flexed 48°, abducted 38°, pronated 80°.
- Buchholz & Armstrong, "An ellipsoidal representation of human hand anthropometry",
  *Human Factors* 33(4):429–441, 1991. Ellipsoid radii per segment.
- MediaPipe Hands: 21 landmarks per hand from a single image
  (https://github.com/google-ai-edge/mediapipe, `hands.md`). Used by Cosmos.
- MANO (Romero, Tzionas, Black, SIGGRAPH Asia 2017, https://mano.is.tue.mpg.de): learned
  parametric hand mesh, 16 joints, shape PCA; research license.
- MyoSuite MyoHand (arXiv:2205.13600): musculoskeletal model in MuJoCo.
- Häger-Ross & Schieber 2000 for finger coupling (`hager-ross-finger-independence.md`).
- Cosmos IK scheme (`cosmos-keyboards.md`).

## Verified parameters (issue #6, H1)

Checked 2026-10-21. **Status:** ✓ = number read in a readable source; ≈ = consistent with a
rounded source but the exact digit is not confirmed; **(verify)** = not found.

### Segment lengths as a fraction of hand length (HL)

HL = distal wrist crease to tip of the middle finger. Buchholz values are **kinematic**
lengths (joint centre to joint centre; distal = DIP centre to tip). Bone lengths from
X-rays (Melzner) are shorter: they leave out the joint gaps and soft tissue at the tip.

| Digit | Segment | Buchholz 1992 | Status | Melzner 2022 bone length (X-ray, n=71) |
|---|---|---|---|---|
| Thumb | CMC (trapezium) | 0.118 | ≈ [B] | — |
| Thumb | Metacarpal | 0.251 | ≈ [B] | 0.234 |
| Thumb | Proximal | 0.196 | ≈ [B] | 0.158 |
| Thumb | Distal | 0.158 | ≈ [B] | 0.115 |
| Index | Metacarpal | 0.374 | (verify) | 0.348 |
| Index | Proximal | 0.245 | ✓ [A] ≈ [B] | 0.203 |
| Index | Middle | 0.143 | ✓ [A] ≈ [B] | 0.118 |
| Index | Distal | 0.097 | ✓ [A] ≈ [B] | 0.088 |
| Middle | Metacarpal | 0.373 | (verify) | 0.334 |
| Middle | Proximal | 0.266 | ≈ [B] | 0.225 |
| Middle | Middle | 0.170 | ≈ [B] | 0.143 |
| Middle | Distal | 0.108 | ≈ [B] | 0.093 |
| Ring | Metacarpal | 0.336 | (verify) | 0.295 |
| Ring | Proximal | 0.244 | ≈ [B] | 0.211 |
| Ring | Middle | 0.165 | ≈ [B] | 0.136 |
| Ring | Distal | 0.107 | ≈ [B] | 0.095 |
| Little | Metacarpal | 0.295 | (verify) | 0.278 |
| Little | Proximal | 0.204 | ≈ [B] | 0.167 |
| Little | Middle | 0.117 | ≈ [B] | 0.096 |
| Little | Distal | 0.093 | ≈ [B] | 0.084 |

- **[A]** Bundhoo et al., *Robotica* 2008, doi:10.1017/S026357470800458X, Table II: index
  phalanx coefficients 0.245 / 0.143 / 0.097 citing Buchholz 1992 (HL 182.9 mm used).
- **[B]** Peerdeman, PhD thesis, Univ. Twente 2014, Table 6.1 (citing Buchholz 1992 and
  Buchholz & Armstrong 1991): ellipsoid half-lengths `c`/HL to two decimals. For every phalanx
  and the whole thumb, `c` = Buchholz value / 2 after rounding (e.g. index 0.12 / 0.07 / 0.05;
  thumb 0.06 / 0.13 / 0.10 / 0.08). The 3rd decimals come from the commonly reproduced
  Buchholz table and are not confirmed against the paper.
- **Finger metacarpals (verify):** Peerdeman's `c` (0.21–0.23) is not half of 0.30–0.37, so the
  ellipsoid paper uses a different metacarpal segment. GECKO places MCP joints directly
  (from a scan or a geometry parameter), so this value is not needed for reach costs.
- Melzner, *Development and validation of a new musculoskeletal hand and elbow model*,
  Dr. sc. hum. dissertation, Univ. Regensburg 2022 (epub.uni-regensburg.de/53644), Table 4.1
  (percent, ±SD 0.8–2.4 points; mean HL 190.5 ± 12.7 mm). Says Buchholz measured only six hands.
  Use as a **range check**: kinematic ≥ bone length for every segment above, as expected.
- Buryanov & Kotiuk, *Int J Morphol* 28(3):755–758, 2010 (X-rays, 66 adults), absolute bone
  lengths in mm. Index 39.8 / 22.4 / 15.8, middle 44.6 / 26.3 / 17.4, ring 41.4 / 25.7 / 17.3,
  little 32.7 / 18.1 / 16.0; thumb 31.6 / 21.7; fingertip soft tissue ≈ 3.7–5.7 mm.

### Joint range of motion (degrees, flexion positive)

| Joint | Active ROM | Functional range (daily tasks) | Source |
|---|---|---|---|
| Finger MCP flex | Index 71 ± 11, middle 85 ± 10, ring 85 ± 14, little 86 ± 18 | 19–71 | [C], [E] |
| Finger MCP ext | 0 typical (hyperextension varies) | — | [D] |
| Finger PIP flex | Index 104 ± 8, middle 104 ± 9, ring 107 ± 13, little 99 ± 11 | 23–87 | [C], [E] |
| Finger DIP flex | Index 61 ± 13, middle 74 ± 16, ring 67 ± 16, little 71 ± 16 | 10–64 | [C], [E] |
| Typical (clinical) | MCP 0/90, PIP 0/120, DIP 0/40–50 | — | [D] |
| Finger MCP abd/add | Index ≈ 42, middle ≈ 81 (radial + ulnar), ring ≈ 42, little ≈ 49 total | — | [F] **(verify)** |
| Thumb CMC flex | 49 ± 6 (Hume: 53) | — | [G] |
| Thumb MCP flex | 61 ± 14 (Hume: 56) | — | [G] |
| Thumb IP flex | 81 | — | [G] |
| Wrist | Ext 30–70, flex 40–80, radial dev 10–30, ulnar dev 20–40 | — | [D] |

- **[C]** Becker & Thakor, "A study of the range of motion of human fingers with application
  to anthropomorphic designs", *IEEE Trans Biomed Eng* 35:110–118, 1988 (mean (SD), video
  analysis); table read in Sarac Stroppa's PhD thesis (Sant'Anna 2020), arXiv:2003.11598, Table 2.3.
- **[D]** handsurgeryresource.net "Range of motion, active", citing Adams et al. *J Hand Surg
  Am* 2003 and Rayan & Akelman 2012.
- **[E]** Bain, Polites, Higgs, Heptinstall, McGrath, "The functional range of motion of the
  finger joints", *J Hand Surg Eur* 40(4):406–411, 2015: the range needed for 90% of 20
  Sollerman tasks, 48% / 59% / 60% of active MCP / PIP / DIP motion.
- **[F]** Electronic-inclinometer study of 41 Turkish students (acikerisim.trakya.edu.tr);
  total MCP abduction arc per finger. Small sample, full text not read. Abduction shrinks
  to near zero at full MCP flexion. Medicine Science 2023 (doaj 0ca76c6c…) gives angles
  *between* fingers instead: thumb–index 54°, index–middle 20°, middle–ring 17°,
  ring–little 28° (right hand, males).
- **[G]** Lee & Jung, "Ergonomic evaluation of biomechanical hand function", *Saf Health Work*
  6(1):9–17, 2015 (PMC4372184), Table 3; Hume et al., *J Hand Surg Am* 1990 values via the same
  table. Hume's finger values were not found **(verify)**.

### DIP–PIP coupling

The "DIP ≈ ⅔·PIP" rule has no source we could find. Measured linear fits `θDIP = k·θPIP`
(Roda-Sales, Sancho-Bru, Vergara, *PeerJ* 10:e14051, 2022, Tables 1, 3, 4):

| Study | Index | Middle | Ring | Little |
|---|---|---|---|---|
| Mentzel 2011, fist (n=10) | 0.77 | 0.75 | 0.75 | 0.57 |
| Roda-Sales 2022, free motion | 0.52 | 0.75 | 0.52 | 0.80 |
| Roda-Sales 2022, 26 daily tasks (+ offset) | 0.44 − 2.5° | 0.81 − 14.0° | 0.58 − 12.3° | 0.87 − 10.5° |

Coupling fails when the DIP is passively extended while the PIP flexes (pinch, pressing on
a fingertip), which is the keypress posture. Treat DIP as a free joint with a soft coupling
cost, not as a hard ratio.

### What is still open

- Exact finger-metacarpal kinematic lengths and MCP joint-centre positions (`F`, `G`
  coefficients). Only Buchholz's 1989 dissertation has them. Use scans or geometry parameters.
- Third decimals of the Buchholz phalanx ratios (≈ rows).
- MCP abduction ROM per finger from a larger study; Hume 1990 finger values.
- Neutral (relaxed) curl: Cosmos's 5° / 4° / 9° (MCP / PIP / DIP) is a design choice, not a
  measurement (`cosmos-keyboards.md`).

## What GECKO needs (phase 7), simplest first

1. **Kinematic chain per finger:** MCP (flex + abduct), PIP, DIP with a soft DIP–PIP coupling
   (per-finger slope from the table above). Thumb: CMC 2 DoF + MCP + IP.
2. **Segment lengths:** from a hand scan (MediaPipe) or from hand length with the Buchholz
   ratios above (kinematic lengths; Melzner bone lengths as a lower bound).
3. **Joint limits:** active ROM above as hard limits; functional range as the low-cost zone.
4. **Reach cost:** IK to the key top (Cosmos's closed-form method), cost = weighted joint
   deviation from neutral curl + press-direction misalignment + wrist offset if out of reach.
5. **Coupling:** penalize postures where adjacent fingers must be far apart in flexion
   (ring/middle), using the independence indices.

Validation checks are in `../hand-model-and-geometry.md` (home row cheapest; col-stagger beats
row-stagger; Corne/Dactyl beat flat ANSI).
