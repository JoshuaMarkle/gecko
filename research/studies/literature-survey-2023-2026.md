# Literature survey 2023–2026: computational keyboard design

Written 2026-10-21. Method: Europe PMC and arXiv title/abstract searches restricted to 2023–2026,
Semantic Scholar (rate-limited, one query returned), Firecrawl paper and web search. Titles and
abstracts only; no full texts read. This is a first pass, not an exhaustive review, and venues
that none of these index (CHI, UIST, ASSETS proceedings) were reached only through web search.

## Main finding

Peer-reviewed work on **computational keyboard layout optimization** in 2023–2026 is close to
empty in the indexes searched. The field's active work is the community tools already in
[tool-comparison](tool-comparison.md). Nothing found contradicts the model in
[SYNTHESIS](../SYNTHESIS.md) §2, and nothing replaces it. Coverage gaps below are real gaps in
the literature, not only in our search.

## What the search found

| Item | Year | What it is | Use for GECKO |
|---|---|---|---|
| Engram, IJHCI (doi:10.1080/10447318.2026.2665409, as cited by its README) | 2026 | Peer-reviewed form of the Engram study: multi-objective layout optimization with crowdsourced typing preferences | Check against [engram](../sources/engram-preference-study.md); the DOI is unverified |
| Motor automaticity in natural keyboard typing (bioRxiv, doi:10.64898/2026.06.04.730281) | 2026 | Inter-keypress intervals are faster and less variable for higher-frequency words and bigrams | **E01 must control for bigram frequency**, or the geometry costs absorb a frequency effect |
| emg2qwerty (NeurIPS 2024, arXiv:2410.20081) | 2024 | 108 users, 1,135 sessions, 346 h of wrist sEMG during touch typing on QWERTY, with key labels | Second timing source beside Aalto, and the only public muscle-activity data found. Size not checked; needs a permission ask before any download |
| Computing the optimal keyboard… (arXiv:2310.10956) | 2023 | Course project, integer programming, English only | Low weight: its own abstract says little about typing assumptions |
| Bespoke layouts for Twitter users (arXiv:2310.01858); ability-based personalised keyboards (arXiv:2201.04593, doi:10.3390/mti6080067) | 2022–23 | Single-finger or virtual keyboards | Only the personalisation idea carries over (phase 9 calibration) |
| Wrist-pressure and keyboard-geometry papers | 2023–25 | Keyboard inclination on sitting and standing workstations (doi:10.1080/00140139.2023.2284682); a 2025 paper on blood flow, force, wrist posture and finger speed (doi:10.3390/biomechanics5010015, abstract not read) | Read the second one for G03; see [wrist posture](../sources/wrist-posture-and-finger-load.md) |

## Layout learning cost (L02): evidence is weak

Peer-reviewed: Analysis of alternative keyboards using learning curves (Hum Factors 2009,
doi:10.1177/0018720808329844) and Turkish F vs E layouts (arXiv:1905.11791, 12 students, no
difference in learning rate). Everything else found is forum or blog anecdote: about 30 WPM after
1–2 weeks, full speed after about 4–6 weeks (Colemak) and 8–12 weeks (Dvorak). Treat as
unverified. The familiarity metric should stay a user-set weight, not a fitted one.

## Gaps confirmed

- No recent work found on thumb clusters, per-finger fatigue limits or hand-size effects on layout.
- No peer-reviewed comparison of layout scoring models against measured comfort beyond Engram.
- Non-English multilingual layout work was not searched beyond one Latvian paper
  (arXiv:1707.03753, 2017, carpalx-based).

## Next steps

1. Read the biomechanics 2025 and Engram IJHCI full texts. 2. Add the bigram-frequency covariate
to the E01 plan ([dhakal](../sources/dhakal-136m-keystrokes.md)). 3. Decide on emg2qwerty after a
size check. 4. Search CHI/UIST proceedings directly (ACM DL is not indexed by the tools used).
