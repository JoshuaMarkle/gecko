---
group: human
kind: paper
read: 2026-10-21
status: secondary
feeds: [G03, wrist posture cost, M02 targets]
take: Carpal-tunnel limits ext 33° / flex 49° / radial 22° / ulnar 15°; typing sits at ext ~20°, ulnar ~15°
---

# Wrist posture risk limits and finger load while typing

Abstracts read through Europe PMC (2026-10-21). Full texts were not read, so numbers below are
the abstract's. Closes the "wrist posture risk thresholds" gap in
[OPEN-QUESTIONS](../OPEN-QUESTIONS.md) for the wrist; per-finger strength is still thin.

## Risk limits (carpal tunnel pressure)

Keir, Bach, Hudes, Rempel, Hum Factors 2007, doi:10.1518/001872007779598127. 37 healthy people
moved the wrist slowly while pressure was logged. The "25th percentile angle" protects 75 % of
people from reaching the pressure threshold.

| Motion | 30 mmHg | 25 mmHg |
|---|---|---|
| Extension | 32.7° (CI 27.2–38.1) | 26.6° |
| Flexion | 48.6° (37.7–59.4) | 37.7° |
| Radial deviation | 21.8° (14.7–29.0) | 17.8° |
| Ulnar deviation | 14.5° (9.6–19.4) | 12.1° |

- Ulnar deviation has the narrowest margin, then extension. Flexion has the widest.
- Straight fingers (MCP 0°) give higher pressure than flexed fingers at the same wrist angle;
  pressure passed 30 mmHg in some extended or deviated postures (Keir 1998,
  doi:10.1016/S0363-5023(98)80007-5). So finger curl and wrist angle interact.
- Typing raises pressure beyond the same static posture, and extension or radial deviation
  raise it further (Rempel, Keir, Bach 2008, doi:10.1002/jor.20599, n = 20).
- Gap: the 30 mmHg limit is a physiological proxy, not an injury threshold. The paper itself
  says finger posture and pinch force are not yet in the model.

## What typing actually demands

- Conventional keyboards: ulnar deviation 10–15°, extension ~20°, forearm nearly fully pronated
  (Marklin & Simoneau review, doi:10.2519/jospt.2004.34.10.638). Population study: mean
  extension 18.8° ± 10.0°; most common 2-D posture 20° extension with 20° ulnar deviation
  (10.7 %) (Donoghue 2013, doi:10.1016/j.apergo.2012.04.009).
- So a flat board already uses about 70–100 % of the ulnar limit (10–15° of 14.5°) and about 60 % of the
  extension limit (20° of 32.7°), before any layout effect.
- Fixes (same review): slant (half the opening angle) 10–12.5° brings ulnar deviation near 0°;
  −7.5° slope brings extension near 0° if the rest slopes too; 20–30° tilt cuts pronation to ~45°;
  typists adapt within ~10 minutes. Best of six tested in 100 people: opening 12°, gable 14°,
  slope 0° (Rempel 2007, doi:10.1016/j.apergo.2006.05.001).

## Finger load

- Peak reaction force per keystroke: 3.33 N thumb down to 1.84 N little finger, mean 2.54 N ≈ 10 %
  MVC, 5.4× the 0.47 N make force (Martin 1996, doi:10.1518/001872096778827288).
  Peaks were 2.5–3.9× activation force, so people bottom out keys (Armstrong 1994).
- Flexor tendon force in one finger: 8.3–16.6 N per keystroke, 4–7× fingertip force, n = 5
  surgery patients, long finger only (Dennerlein 1999, doi:10.1002/jor.1100170205).
- Load per keystroke is geometry-independent, so fatigue scales with keystrokes per finger,
  which is what finger-load metrics already count. No source here gives per-finger fatigue limits.

## Key size and hand size

Pereira et al. (Hum Factors 2013, 2014): 17–19 mm horizontal and 16–18 mm vertical pitch had
no speed or usability loss, for large-fingered men (middle finger ≥ 8.7 cm) and small-fingered
women (≤ 7.71 cm). Horizontal 16 mm and vertical 15.5 mm were worse.
Lower bound for GECKO pitch: 17 mm horizontal, 16 mm vertical.

## How to use in GECKO

1. **Posture cost (G03):** penalise wrist angle as a fraction of the limit, convex in the fraction.
   Starting points: ulnar 14.5°, extension 32.7°, radial 21.8°, flexion 48.6° (30 mmHg).
   Use 12.1° / 26.6° for the conservative profile.
2. **Typing offsets:** the limits are for static or slow motion. Add the typing offset
   (+~15° extension, +10–15° ulnar on a flat board) before comparing a geometry to them.
3. **Finger curl:** add MCP angle as a modifier, as a first step "curled fingers lower the cost".
   Needs the hand model's MCP output ([hand-model](../../context/hand-model.md)).
4. **Pitch bounds:** clamp geometry search to ≥ 17 mm horizontal, ≥ 16 mm vertical.
