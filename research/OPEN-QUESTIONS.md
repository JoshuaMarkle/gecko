# Open questions and research backlog

The one place for research debt. Two lists: claims to **verify**, and topics **not yet
researched**. When an item is closed, fix the source file, delete the marker there, and add one
line to "Closed" with the date. `python3 -I research/scripts/research.py lint` counts the
`(verify)` markers still in the files. Last pass: 2026-10-21 (second pass: thumb, finger fatigue, pronation).

## To verify

| Item | Where | Why it is open | To close |
|---|---|---|---|
| Aalto 136M data | [dhakal](sources/dhakal-136m-keystrokes.md) | Aalto host refuses connections; the Kaggle mirror is a third-party upload | Mostly closed: the full mirror agrees with the paper within 3.5% on all four bigram classes. Left: the paper's exact outlier filters, and the Aalto zip if the host returns |
| Buchholz phalanx ratios, third decimals; finger metacarpal lengths | [hand models](sources/hand-models.md) | Read through secondary tables; metacarpals only in the 1989 dissertation | Buchholz 1992 full text. Low value: reach costs do not need them |
| MCP abduction ROM per finger | [hand models](sources/hand-models.md) | One small study (n = 41) | A larger study |
| Thumb workspace scaling with hand size | [thumb](sources/thumb-biomechanics.md) | Tang 2008 abstract gives 15 women, no hand-length scaling | Read the full text; check ANSUR II for thumb length |
| Engram protocol on column-stagger and ortho boards | [Engram](sources/engram-preference-study.md) | Study used row-stagger boards, left hand only | Re-run on a split board (#50) |
| Taillard 1991 tenure range, AZERTY amélioré text | [QAP](sources/qap-metaheuristics.md), [integer programming](sources/integer-programming-bounds.md) | Citations now checked; the papers themselves are unread | Read them if the numbers start to matter |

## Not yet researched

Candidates, not findings. The 2023–2026 survey ([study](studies/literature-survey-2023-2026.md))
found little recent work, so most gaps below are gaps in the literature too.

| Topic | Needed by | State |
|---|---|---|
| Hand-size population data (hand length, breadth, finger ratios) | H11 (#103), hand presets | Only small regional studies found. Source: ANSUR II public CSVs (a few MB each); needs a download OK |
| RSI risk factors; fatigue over a full workday on non-QWERTY boards | M02 targets, `finger_target` | Typing is ≈ 10 % MVC and shows potentiation, not fatigue ([finger force](sources/finger-force-and-fatigue.md)); no per-finger limit exists in the literature found, so none is needed for M02 |
| A forearm pronation *limit* (not just effect of tent) | G03 | Tent 20–30° gives ≈ 45° pronation (Marklin review, Rempel 2007: best of six had 14° gable); no injury-linked threshold found. Elbow and shoulder posture not covered |
| Keyboard-specific thumb data: key force by cluster position, space-bar use, thumb fatigue | I2 (#71), G06 | Joint range and operational workspace done ([thumb](sources/thumb-biomechanics.md)); studies are phone-based and small |
| Same-finger skipgram cost | S01/S02 weights | Timing shows none (+0.5 %, CI −1.5 to +2.6; [validity](experiments/2026-10-22-cost-function-validity.md)); no comfort data | A comfort or fatigue measurement of SFS |
| Layout learning cost | L02 familiarity | Two peer-reviewed studies, rest anecdote; keep as a user-set weight |
| Symbol and code frequency, layer design | Y-track, D9 (#91) | The French-standard CACM paper is unread |
| Key switch force, travel, actuation | J, I-tracks | Armstrong 1994 and Martin 1996 give typing forces; no switch-design source |
| Manufacturability: plate, case, PCB, printing | I4 (#73), J4 (#75) | Only ergogen spacing limits |
| Non-English n-gram sources, multilingual layouts | E3 corpora | cmini and Leipzig listed, not evaluated |
| Timing data beyond QWERTY habits | E01 (#31) | 136M data is QWERTY-biased; emg2qwerty is a second source (size unchecked) |
| Bigram-frequency control in the IKI model | E01 (#31) | 2026 preprint says frequency speeds keypresses; the fit must include it |
| CHI/UIST proceedings 2023–2026, Engram IJHCI full text, 2025 Biomechanics wrist paper | all tracks | Not indexed by the tools used; abstracts only |

## Closed

- 2026-10-21 — Thumb ROM and operational workspace ([thumb](sources/thumb-biomechanics.md)); typing force, make-force and fatigue studies ([finger force](sources/finger-force-and-fatigue.md)).
- 2026-10-21 — carpalx `k`, `w`, row/finger penalties, stroke weights checked on the primary site (site is reachable now).
- 2026-10-21 — Citations checked via Crossref / Europe PMC / arXiv: Rempel 2009, John & Karrenbauer MOTOR 2019, Feit CACM 2021, CVT-MAP-Elites, FUTO Swipe (arXiv:2606.25247, and its role corrected), Engram IJHCI 2026 DOI, Hume 1990.
- 2026-10-21 — RoTS tenure 0.9n–1.1n confirmed in a secondary source (arXiv:1010.0157).
- 2026-10-21 — Wrist posture risk thresholds ([wrist posture](sources/wrist-posture-and-finger-load.md)); first-pass 2023–2026 survey.
- 2026-10-21 — Buchholz segment ratios and joint ROM (#6); skipgram definition; trigram classes and `rg` / `my` / `vt` distances tested.
- Dropped: "neutral curl 5° / 4° / 9°" is a Cosmos design choice, not a claim to verify; it stays documented as such in [Cosmos](sources/cosmos-keyboards.md).
