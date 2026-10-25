# Inverse fit: what layout designers value, and inward rolls

| | |
|---|---|
| Date | 2026-10-25 |
| Question | Under which weights is each well-known layout optimal (is it optimal under any weights near `default`)? What do the designers' choices imply they value? Is there a measured reason for the community's preference for inward rolls? |
| Answer | No community layout is optimal under nearby weights; the best modern ones come within 0.5–1.2 %, classics stay 6–300 % away. Designers imply Engram comfort ×0.14, hand alternation ×0.28, pinky–ring ×0.25, SFS ×5, rolls in ×1.42 vs rolls out ×1.08 (11 modern layouts, stable under σ 0.5 and 1.0). Inward rolls are not faster (pair +4.7 %, CI +0.2 to +10.1 %; all from finger-skipping and pinky rolls) nor preferred (Engram P = 0.51); only adjacent-finger inward rolls lean comfortable (P 0.54, n.s.). Weighting them up costs held-out speed and comfort. |
| Command | `bench/inverse-fit-2026-10-25/README.md` (`grid.sh`, `roll_timing.py`, `inroll_price.sh`, `analyze.py`, `judges.py`) |
| Data | ansi-30, `prose`, `default`; Aalto `timing30.npz` (fast QWERTY typists); `data/comfort/engram-bt.csv` source choices |
| Artifacts | `bench/inverse-fit-2026-10-25/`, paper `docs/papers/inverse-fit/` |

## Findings

- **Method:** `gecko fit-layout` (`context/objective.md` Inverse fit). A planted ×4 rolls-in
  preference is recovered in direction; GECKO's optima keep ×1.00 on every weight.
- **Consensus:** Gallium is within 1.2–3 % of the best under five of six other leading modern
  layouts' fitted weights.
- **Rolls:** the community preference seen through GECKO's metrics is "less outward-tilted than
  `default`" (fitted in/out ratio ×1.2–×2.4 in 16 of 17 layouts), not inward over outward.
  Likely sources: analyzer conventions (Oxeylyzer weights, Engram's formula scores inward 1.0 vs
  outward 0.78 while its own measured choices tie).
- **Negative / open:** timing and comfort data come from QWERTY typists; neither measures rolls on
  a learned layout. Most community fits were still adding competitors after 12 rounds (gaps are
  upper bounds).
