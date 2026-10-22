# Open questions and research backlog

The one place for research debt. Two lists: claims to **verify**, and topics **not yet
researched**. When an item is closed, fix the source file, delete the marker there, and add one
line to "Closed" with the date. `python3 -I research/scripts/research.py lint` counts the
`(verify)` markers still in the files. Last pass: 2026-10-21.

## To verify

| Item | Where | Why it is open | To close |
|---|---|---|---|
| Aalto 136M data | [dhakal](sources/dhakal-136m-keystrokes.md) | Aalto host refuses connections; Kaggle mirror (18.85 GB) needs a user API token and a size-approved download | Token in `~/.kaggle/`, sample first, compare with the paper's means |
| Buchholz phalanx ratios, third decimals; finger metacarpal lengths | [hand models](sources/hand-models.md) | Read through secondary tables; metacarpals only in the 1989 dissertation | Buchholz 1992 full text. Low value: reach costs do not need them |
| MCP abduction ROM per finger | [hand models](sources/hand-models.md) | One small study (n = 41) | A larger study |
| Engram protocol on column-stagger and ortho boards | [Engram](sources/engram-preference-study.md) | Study used row-stagger boards, left hand only | Re-run on a split board (#50) |
| Taillard 1991 tenure range, AZERTY amélioré text | [QAP](sources/qap-metaheuristics.md), [integer programming](sources/integer-programming-bounds.md) | Citations now checked; the papers themselves are unread | Read them if the numbers start to matter |

## Not yet researched

Candidates, not findings. The 2023–2026 survey ([study](studies/literature-survey-2023-2026.md))
found little recent work, so most gaps below are gaps in the literature too.

| Topic | Needed by | State |
|---|---|---|
| Hand-size population data (hand length, breadth, finger ratios) | H11 (#103), hand presets | Only small regional studies found. Source: ANSUR II public CSVs (a few MB each); needs a download OK |
| Per-finger strength, speed, fatigue; RSI risk factors | M02 targets, `finger_target` | Peak key force per finger known ([wrist posture](sources/wrist-posture-and-finger-load.md)); no fatigue limits |
| Forearm pronation and other posture limits beyond the wrist | G03 | Carpal-tunnel angles done; pronation limit missing |
| Thumb biomechanics and thumb-cluster ergonomics | I2 (#71), G06 | ROM only |
| Layout learning cost | L02 familiarity | Two peer-reviewed studies, rest anecdote; keep as a user-set weight |
| Symbol and code frequency, layer design | Y-track, D9 (#91) | The French-standard CACM paper is unread |
| Key switch force, travel, actuation | J, I-tracks | Armstrong 1994 and Martin 1996 give typing forces; no switch-design source |
| Manufacturability: plate, case, PCB, printing | I4 (#73), J4 (#75) | Only ergogen spacing limits |
| Non-English n-gram sources, multilingual layouts | E3 corpora | cmini and Leipzig listed, not evaluated |
| Timing data beyond QWERTY habits | E01 (#31) | 136M data is QWERTY-biased; emg2qwerty is a second source (size unchecked) |
| Bigram-frequency control in the IKI model | E01 (#31) | 2026 preprint says frequency speeds keypresses; the fit must include it |
| CHI/UIST proceedings 2023–2026, Engram IJHCI full text, 2025 Biomechanics wrist paper | all tracks | Not indexed by the tools used; abstracts only |

## Closed

- 2026-10-21 — carpalx `k`, `w`, row/finger penalties, stroke weights checked on the primary site (site is reachable now).
- 2026-10-21 — Citations checked via Crossref / Europe PMC / arXiv: Rempel 2009, John & Karrenbauer MOTOR 2019, Feit CACM 2021, CVT-MAP-Elites, FUTO Swipe (arXiv:2606.25247, and its role corrected), Engram IJHCI 2026 DOI, Hume 1990.
- 2026-10-21 — RoTS tenure 0.9n–1.1n confirmed in a secondary source (arXiv:1010.0157).
- 2026-10-21 — Wrist posture risk thresholds ([wrist posture](sources/wrist-posture-and-finger-load.md)); first-pass 2023–2026 survey.
- 2026-10-21 — Buchholz segment ratios and joint ROM (#6); skipgram definition; trigram classes and `rg` / `my` / `vt` distances tested.
- Dropped: "neutral curl 5° / 4° / 9°" is a Cosmos design choice, not a claim to verify; it stays documented as such in [Cosmos](sources/cosmos-keyboards.md).
