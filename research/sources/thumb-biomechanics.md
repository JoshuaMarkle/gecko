---
group: human
kind: paper
read: 2026-10-21
status: secondary
feeds: [G06, I2, M09, G05, thumb IK]
take: Thumb uses < 65 % of its joint range in normal use and performs best near rest; far-flexed keys near the thumb base are worst
---

# Thumb range, workspace and load

Abstracts read through Europe PMC / PubMed, plus the open full text of Barakat 2013 (2026-10-21).
Full texts of the others were not read, so their numbers are the abstract's. Fills the "thumb
biomechanics" row of [OPEN-QUESTIONS](../OPEN-QUESTIONS.md), which had ROM only.

## Joint range (100 people, no thumb disease, 62 female, age 18–37)

Barakat, Field, Taylor, *Hand* 8(2), 2013, doi:10.1007/s11552-013-9492-y. Right thumb, goniometer.

| Joint | Motion | Mean (range) |
|---|---|---|
| IP | flexion / extension | 88° (80–90) / 12° (0–45) |
| MCP | flexion / extension | 60° (43–70) / 8.1° (0–15) |
| CMC | palmar abduction (anteposition) | 61° (50–71) |
| CMC | radial abduction | 63° (53–71) |
| CMC | retroposition | 31° (25–38) |
| CMC | adduction | 10° (5–20); unreliable (ICC 0.02–0.04) |

- Joints trade range: IP extension vs MCP extension r = −0.60; CMC radial abduction vs MCP
  extension r = −0.50. Hand-model joint limits should be per-person, not independent maxima.
- IP extension is the most variable (0–45°), so do not rely on it for reach.
- Agrees with the values already in [hand-models](hand-models.md) (MCP 60 vs 61/56; IP 88 vs 81).

## Operational vs maximal workspace

Tang, Zhang, Li, *Ergonomics* 51(7), 2008, doi:10.1080/00140130801958667. 15 women, marker-based.

| Thumb tip range | Maximal (circumduction) | Operational (submaximal, random) |
|---|---|---|
| Abduction / adduction | 102.8 ± 9.9 mm | 67.3 ± 16.1 mm |
| Flexion / extension | 130.7 ± 14.1 mm | 73.1 ± 18.0 mm |

- CMC and MCP angles in operational use stayed **below 65 %** of their maximal ranges, and the
  CMC moved obliquely to its anatomical axes.
- So a comfortable thumb cluster is a region of about 65–75 mm across, not the 100–130 mm the
  thumb can sweep. Women only; the hand-length scaling is not given in the abstract **(verify)**.

## Where thumb keys perform best

Trudeau, Young, Jindrich, Dennerlein, *J Biomech* 45(14), 2012, doi:10.1016/j.jbiomech.2012.07.012.
10 people, Fitts index of performance between key pairs on a phone.

- Best when the thumb is near its resting posture: neither flexed nor extended, slight CMC
  abduction and supination.
- Worst with strong thumb flexion, i.e. keys nearest the thumb's own base.
- Relevance: a keyboard thumb cluster tilted toward the thumb and placed near rest (Dactyl,
  Kinesis style) follows this; a flat row under the palm edge does not. Phone data, small n.

## Load

- Typing force peaks highest in the thumb: 3.33 N vs 1.84 N little finger, mean 2.54 N
  (Martin 1996, see [wrist posture](wrist-posture-and-finger-load.md)). A thumb key costs more
  per press than any finger key, which supports charging thumb keys a higher force weight than
  M09's squared distance from a target share.
- CMC joint reaction at a phone tap: 12.5 N and 95.5 N·mm; 3× the force and 1.4× the torque
  of keyboard typing in the same study (Kim, Kim, Park, *J Orthop Res* 37, 2019,
  doi:10.1002/jor.24405; n = 19). It is a different task (screen taps), but it says the thumb's
  base joint is the loaded one when the thumb works hard.
- Thumb overuse injuries (de Quervain's, CMC arthritis) are discussed in keyboard-community
  posts [Getreuer](https://getreuer.info/posts/keyboards/thumb-ergo/index.html), not a
  controlled study; treat as a hypothesis, not a limit.
- In Baker et al. (*Clin Biomech* 22, 2007, doi:10.1016/j.clinbiomech.2006.08.008; 20 typists),
  the thumb is the one digit whose posture differs between left and right hands. Do not mirror
  thumb costs blindly.

## How to use in GECKO

1. **Thumb IK limits (H3 / hand model):** the model's joint scales (CMC 26°, MCP 34°, IP 46°)
   are about 40 % / 55 % / 52 % of the ranges above; operational use stays under 65 %, so these
   scales are plausible. No change needed; add the cross-joint trade-off only if a validation
   case needs it.
2. **G06 thumb-arc fit (#71):** target region 65–75 mm wide around the rest point, not the maximal
   sweep. Penalise keys in the strongly flexed zone near the thumb base more than keys at
   extension.
3. **M09:** consider a force weight per thumb press (≈ 1.3–1.8 × a finger press, from 3.33 N vs
   1.84–2.54 N) on top of the share target. Needs a decision and a profile parameter.
4. **Hand presets (H11):** add thumb ranges only if hand size changes them; none of these
   studies scale by hand size.

Gaps: no keyboard-specific thumb study found (space bar and cluster key forces, pointing-style
thumb keys); samples are small and mostly women; none gives a fatigue limit.
