---
group: human
kind: paper
read: 2026-10-21
status: secondary
feeds: [M02, M07, finger_target, switch force]
take: Typing is low-force (≈ 10 % MVC) and shows potentiation, not fatigue, over 6 h; key force and pace matter more than which finger
---

# Finger force and fatigue while typing

Abstracts read through Europe PMC (2026-10-21); full texts not read. Extends the "Finger load"
section of [wrist posture](wrist-posture-and-finger-load.md). Closes most of the "per-finger
strength, speed, fatigue" gap: there is no per-finger fatigue limit in the literature found,
and the evidence says one is probably not needed at typing forces.

## Findings

| Finding | Number | Source |
|---|---|---|
| Typing force vs max | mean peak 2.54 N ≈ 10 % MVC (thumb 3.33 N, little 1.84 N); EMG-based force estimates 1.2–3.2× the measured force | Martin 1996, doi:10.1518/001872096778827288, n = 10 |
| Force above need | people press 4.1–7.0× harder than needed on a 0.28 N key, 2.2–3.5× on a 0.83 N key | Gerard 1996, doi:10.1080/15428119691014549, n = 6, 2 h |
| Make force vs load | 0.34 → 0.47 N: no change; 0.47 → 1.02 N: applied force +40 %, flexor EMG +20 % | Rempel 1997, doi:10.1080/001401397187793, n = 10 |
| Stiffer key | 0.83 vs 0.28 N: 54 % more peak force, 34 % more flexor EMG; fatigue "mild" over 2 h | Gerard 1996 |
| Pace | force and flexor / extensor EMG rise linearly with typing rate; discomfort jumps sharply between self-selected pace and maximum | Gerard 2002, doi:10.1518/0018720024494757 |
| Fast typists | no more flexor EMG, less extensor EMG, not more discomfort than slow typists | Gerard 2002 |
| 6 h keyboard use | middle-finger FDS twitch force **+30 %** (potentiation), twitch time −3 %; mouse use added true fatigue | Kim & Johnson, 2014, doi:10.1007/s00421-014-2974-y, n = 18 |
| After submaximal isometric work | ring finger flexor force −17–26 %; keystroke duration −5 % (6 ms) | Chang 2009, doi:10.1007/s00421-008-0878-4, n = 14 |
| Finger posture | extended finger posture raises joint torque, energy and stiffness at the same fingertip force | Jindrich 2004, doi:10.1016/j.clinbiomech.2004.03.003, n = 16, index only |
| Idle fingers | lifting idle fingers raised the master fingers' maximal flexion force +12 % on average (−8 to +30 %) | Li & Yue 2002, doi:10.1080/00222890209601951, n = 10 |

## What this means

- Typing force is about a tenth of maximum, so strength differences between fingers decide
  little; **keystrokes per finger and key force** decide more. That matches GECKO's finger-load
  metrics (M02, M07) and says a per-finger strength weight would add little. Pinky and ring
  limits come from independence ([Hager-Ross](hager-ross-finger-independence.md)) and reach,
  not from weakness.
- Key force is a design variable GECKO does not model: halving make force from 1.02 to 0.47 N
  cut applied force by ~30 %. Relevant to J/I tracks and to `finger_target` only if the user
  sets a switch weight.
- Fast typists are not more fatigued, and the pace discomfort threshold is about the user's
  own preferred speed. Layout speed claims should not be turned into a fatigue claim.
- The "finger rests on the key" and "extended finger" effects point to home-row curl and the MCP
  posture in the hand model, not to a layout term.

## How to use in GECKO

1. **Keep M02/M07 as keystroke shares** with `finger_target`; do not add per-finger fatigue
   curves.
2. **Optional switch-force parameter** for geometry scoring: load ∝ make force above 0.47 N
   (≈ +40 % per +0.55 N, from Rempel 1997). One number, user-set; defer.
3. **Do not calibrate on %MVC.** If personal calibration (K2, #46) measures finger speed,
   report it as speed, not strength.

Gaps: no data on thumbs for fatigue, none on ortho or split boards, small samples, all
studies on QWERTY typists at office pace; nothing on 8+ h of mixed keyboard and layer use.
