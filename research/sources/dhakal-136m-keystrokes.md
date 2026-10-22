---
group: human
kind: paper
read: 2026-10-20
status: secondary
feeds: [E01]
take: IKI by hand/alternation; rollover ≈ 50 % for fast typists (data site unreachable)
---

# Dhakal, Feit, Kristensson, Oulasvirta — "Observations on Typing from 136 Million Keystrokes" (CHI 2018)

doi:10.1145/3173574.3174220. PDF: https://www.pokristensson.com/pubs/DhakalEtAlCHI2018.pdf.
Data: https://userinterfaces.aalto.fi/136Mkeystrokes/ (blocked from our env; also Cambridge
repository https://www.repository.cam.ac.uk/handle/1810/277516). Free for research.

## What it is

168,000 volunteers transcribing sentences online, 136M keystrokes, mostly QWERTY on their
own keyboards. Logged: key-down/key-up timestamps per character, sentences, demographics.

## Numbers (means; σ in parentheses)

| IKI type | All | Fast group | Slow group |
|---|---|---|---|
| Left hand (same hand) | 215 ms (97) | 124 | 385 |
| Right hand (same hand) | 204 ms (99) | 117 | 380 |
| Alternating hands | 198 ms (104) | 109 | 409 |
| Letter repetition | 176 ms (70) | 145 | 231 |

- Rollover (next key down before previous up): 25 % of keystrokes on average, **≈ 50 % for
  fast typists**, 7.6 % for slow. Fastest typists use rollover on 40–70 % of keystrokes.
- 8 clusters by normalized IKI. Fastest ("fast rollovers", 68 WPM): IKI 162 ms, alternation
  IKI 138 vs same-hand 150–160 ms. Hand-alternation benefit ranges 9–46 ms by cluster.
- Bigrams typed by different hands or fingers predict speed better than repetitions.

## How to use in GECKO

- Fit an **IKI model** (metric B12/E01): `IKI(i→j) = f(same hand, same finger, finger pair,
  dx, dy, row change)` by regression on QWERTY bigrams. Because the features are geometric,
  the fitted model transfers to any layout on the same board (this is the "fit geometry, not
  letters" rule in [corpora](corpora.md)).
- Rollover makes **alternation and inward rolls fast**; a pure "time per bigram" model
  undervalues rolls if it ignores overlap. Use key-down to key-down intervals.
- Weak signal for comfort: Engram found speed ≠ comfort (`engram-preference-study.md`). Use
  speed as one objective, not the objective.
- Data access needs the Aalto download (MySQL dump/CSV); fetch from the user's machine if the
  site is reachable there.

## Access (checked 2026-10-21)

- `userinterfaces.aalto.fi` refuses connections on 443 from the user's machine and from Firecrawl's
  proxy, so the host looks down, not blocked locally. The Cambridge record holds only the 2.8 MB paper.
- Original: `.../136Mkeystrokes/data/Keystrokes.zip`, 1.4 GB zipped, 16 GB unzipped, one tab-separated
  file per participant, with `readme.txt` and a metadata file. Free for research and non-commercial use with attribution.
- Kaggle mirror `noelsmathew/keystrokes`: 169k files, 18.85 GB, CC BY-NC-SA 4.0. Columns:
  PARTICIPANT_ID, TEST_SECTION_ID, SENTENCE, USER_INPUT, KEYSTROKE_ID, PRESS_TIME, RELEASE_TIME,
  LETTER, KEYCODE. Third-party upload; check it against the paper's numbers before trusting it.

