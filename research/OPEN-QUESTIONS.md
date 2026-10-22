# Open questions and research backlog

The one place for research debt. Two lists: claims to **verify**, and topics **not yet
researched**. When an item is closed, fix the source file, delete the marker there, and move the
line to "Closed" with the date. `python3 -I research/scripts/research.py lint` counts the
`(verify)` markers still in the files.

## To verify

| Item | Where | Why it is open | To close |
|---|---|---|---|
| carpalx default `k`, `w` values | [carpalx](sources/carpalx.md) | Primary site unreachable; values come from the antibracket mirror's configs | Fetch the archived carpalx docs or compare with the Perl source |
| Aalto 136M download | [dhakal](sources/dhakal-136m-keystrokes.md) | Site blocked from the research environment; needed for #31 | Try from the user's machine; else the Cambridge repository |
| Rempel et al. title and year | [split ergonomics](sources/split-keyboard-ergonomics.md) | Cited from the NIOSH record | Open the NIOSH record |
| John & Karrenbauer MOTOR 2019 exact title; Feit et al. CACM 2021 details | [integer programming](sources/integer-programming-bounds.md) | MPI pages blocked; read from abstracts | Fetch the PDFs |
| RoTS tenure range (≈ 0.9n–1.1n) | [QAP metaheuristics](sources/qap-metaheuristics.md) | Remembered from the HeuristicLab docs | Read Taillard 1991 |
| FUTO Swipe arXiv id | [ML approaches](sources/ml-approaches.md) | Id not confirmed | Search arXiv |
| CVT-MAP-Elites citation | [search-space study](studies/search-space-visualization.md) | Citation details unchecked | Search the TEC 2018 paper |
| Finger-metacarpal lengths, MCP joint centres | [hand models](sources/hand-models.md) | Only in Buchholz's 1989 dissertation | Not needed for reach costs; use scans or geometry parameters |
| Third decimals of the Buchholz phalanx ratios | [hand models](sources/hand-models.md) | Read through secondary tables | Buchholz 1992 full text |
| MCP abduction ROM per finger; Hume 1990 finger values | [hand models](sources/hand-models.md) | One small study (n = 41) | A larger study |
| Neutral (relaxed) curl 5° / 4° / 9° | [Cosmos](sources/cosmos-keyboards.md), [hand models](sources/hand-models.md) | A design choice in Cosmos, not a measurement | A posture study of relaxed finger curl |
| Engram protocol on column-stagger and ortho boards | [Engram](sources/engram-preference-study.md) | Study used row-stagger keyboards, left hand only | Re-run the comparison on a split board (#50) |

## Not yet researched

Gaps found by reading `context/` and the roadmap against `sources/`. These are candidates, not
findings: no literature search has been done for any of them. The folder was built from
16 repositories and about 20 papers, not from a systematic search, so a structured survey of
recent (2023–2026) keyboard-design and typing-ergonomics work is itself the first item.

| Topic | Needed by | Why |
|---|---|---|
| Systematic survey of recent computational keyboard design (HCI, ergonomics, optimization) | all tracks | Coverage so far follows the tools we found, not the literature |
| Hand-size population data (percentiles of hand length, breadth, finger ratios) | H11 (#103), hand presets | Presets (× 0.92 / 1 / 1.08) are heuristic |
| Per-finger strength, speed and fatigue; RSI risk factors | M02 targets, `finger_target` | Only individuation (Häger-Ross) is sourced |
| Thumb biomechanics and thumb-cluster ergonomics | I2 (#71), G06 | Thumb data is thin: ROM only |
| Wrist posture risk thresholds (ulnar deviation, extension, pronation) | G03 | Split-keyboard studies give means, not risk limits |
| Layout learning cost and adaptation time | L02 familiarity | Only an adaptation study is cited ([ML approaches](sources/ml-approaches.md)) |
| Symbol and code frequency, layer design studies | Y-track, D9 (#91) | The French-standard CACM paper is unread |
| Key switch force, travel and actuation per finger | J, I-tracks | No physical-switch sources |
| Manufacturability: plate, case, PCB and printing constraints | I4 (#73), J4 (#75) | Only ergogen spacing limits |
| Non-English n-gram sources and multilingual layouts | E3 corpora | cmini and Leipzig listed, not evaluated |
| Typing models beyond QWERTY habits (learned fingerings after layout change) | E01 (#31) | 136M data is QWERTY-biased |

## Closed

- 2026-10-21 — Buchholz segment ratios and joint ROM checked ([hand models](sources/hand-models.md), #6).
- 2026-10-21 — Skipgram definition: GECKO counts `x _ z` with any middle character (`context/decisions.md`).
- 2026-10-21 — Trigram classes and `rg` / `my` / `vt` distances tested (`tests/validation.rs`).
