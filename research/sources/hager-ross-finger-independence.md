# Häger-Ross & Schieber — "Quantifying the Independence of Human Finger Movements" (J. Neurosci. 2000)

doi:10.1523/JNEUROSCI.20-22-08542.2000. Open: https://pmc.ncbi.nlm.nih.gov/articles/PMC6773164/

## Numbers

Individuation index (1 = only the instructed finger moves), right hand mean:

| Thumb | Index | Middle | Ring | Little |
|---|---|---|---|---|
| 0.983 | 0.982 | 0.937 | 0.907 | 0.943 |

Stationarity (how still a finger stays when others move): thumb 0.994, index 0.967,
middle 0.941, ring 0.908, little 0.943. Left hand similar.

- Thumb and index are most independent; **ring is least**, then middle.
- Moving ring or middle drags neighbours most (shared tendons, muscle organization, neural control).
- Enslaving is highest for adjacent fingers (van den Noort et al. 2016, PMC5167409).

## How to use in GECKO

- Physical basis for: higher cost on ring-finger SFBs and on rolls *toward* the ring finger
  (Kühlmak's ring rule), the middle↔ring pair costs in KLO's movement table, and the community
  complaint about Semimak's ring load.
- Turn into a per-finger **independence factor** in the finger model (`parameters.md`):
  e.g. scale same-hand adjacent-finger bigram costs by `(1 − individuation)` of the pair.
  Treat as a prior to be overridden by calibration data.
