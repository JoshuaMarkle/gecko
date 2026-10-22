---
group: human
kind: paper
read: 2026-10-20
status: secondary
feeds: [G03, geometry priors]
take: Slope 0 to −4°, opening ≈ 15°
---

# Split, tented, and sloped keyboards: posture studies

- Rempel, Nathan-Roberts, Chen, Odell, "The effects of split keyboard geometry on upper body
  postures", *Ergonomics* 52(1), 2009, doi:10.1080/00140130802481040 (title, authors and year
  checked on Europe PMC, PMID 19308823; NIOSH record: https://stacks.cdc.gov/view/cdc/191592).
- Marklin, Simoneau, Monroe, "Wrist and forearm posture from typing on split and vertically
  inclined computer keyboards", *Human Factors* 1999 (https://epublications.marquette.edu/phys_therapy_fac/31).
- Meta-analysis of split/tented designs, *Am. J. Occup. Ther.* 60(1), 2006 (AOTA).

## Numbers

- Gabled (14° tent) keyboard, 7 conditions: **slope 0° to −4°** (negative slope = back edge lower
  than front edge) gave the least wrist extension, forearm pronation and elbow height.
- **Opening (splay) angle ≈ 15°** balances less ulnar deviation against more pronation and
  elbow separation.
- Lower keyboard height of the two tested was better.
- Split keyboards set up correctly cut mean ulnar deviation from ~12° to within 5° of neutral
  (90 office workers).
- Adjustable open-tented designs: large effect on pronation (r = 0.85) and ulnar deviation;
  fixed-angle split: large effect on ulnar deviation only (r = 0.79).

## How to use in GECKO

- Bounds and priors for geometry search (phases 6–8): split angle per half around 7–8° (15°
  total opening), tent angle as a free parameter with a pronation cost, negative slope.
- Metric G03 (wrist posture): cost = weighted squared deviation of ulnar deviation, extension,
  pronation from neutral, computed from the half's pose and the forearm position.
- These are posture studies, not typing-speed or comfort studies; combine with the hand model.
