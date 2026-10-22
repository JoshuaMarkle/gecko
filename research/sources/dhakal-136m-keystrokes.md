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
Data: https://userinterfaces.aalto.fi/136Mkeystrokes/ (host down 2026-10-21; also Cambridge
repository https://www.repository.cam.ac.uk/handle/1810/277516, paper only). Free for research.
Usable copy: Kaggle mirror `noelsmathew/keystrokes`, checked against the paper below.

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

## Mirror sample check (2026-10-21)

54 participants (810 sentences, ~19k letter bigrams) pulled from `noelsmathew/keystrokes` with the
Kaggle CLI (since deleted). Letter-to-letter press-to-press IKI
under 2 s, QWERTY left hand = QWERTASDFGZXCVB:

| | Paper (all) | Sample |
|---|---|---|
| Left same-hand | 215 ms | 192 ms |
| Right same-hand | 204 ms | 176 ms |
| Alternating | 198 ms | 160 ms |
| Repetition | 176 ms | 183 ms (n = 695) |

- Same columns as documented; the mirror's per-participant files parse cleanly.
- Order agrees (alternating fastest, same-hand slower), but the sample is 25 ms faster overall and
  the alternation gap is larger (≈ 30 ms vs 6–17). 54 of 168k people is too few to call it a
  mismatch, and the paper's exact filtering is unknown. Not yet verified.
- Kaggle limits listing and downloads (HTTP 429 after ~55 single-file downloads in a minute). The
  full 18.85 GB needs a slower, resumable pull, or the first 1.4 GB zip if the Aalto host returns.

## Full mirror check (2026-10-21)

All 168,595 participant files from `noelsmathew/keystrokes` (18.85 GB, zip tests clean) unzipped and scanned; the raw files were then deleted
(re-download from Kaggle to rerun). Script and per-bigram output are kept in
`~/projects/gecko-workspace/aalto-analysis/` (`iki.py`, `out/bigrams.csv`). Same measure as above (letter-to-letter press-to-press IKI, 0 < IKI < 2 s):

| | Paper (all) | Mirror, all | n | Mirror, sentence typed exactly |
|---|---|---|---|---|
| Left same-hand | 215 ms | 213.4 | 16.5M | 207.8 |
| Right same-hand | 204 ms | 201.0 | 10.6M | 195.2 |
| Alternating | 198 ms | 191.2 | 29.2M | 184.9 |
| Repetition | 176 ms | 175.7 | 2.1M | 173.4 |

- The mirror matches the paper to within 1% on left and repetition and within 3.5% on right and
  alternating; the order and the gaps agree. The 54-person sample above was unrepresentative
  (too fast, alternation gap too large), not evidence against the mirror.
- The remaining few ms are plausibly the paper's unstated outlier filter. The mirror is usable as the
  Aalto data; still a third-party upload, so cite the paper's numbers, not these, in published results.
- `bigrams.csv` holds n and mean IKI for all 26 x 26 letter bigrams, for the all and clean variants,
  ready for fitting a bigram time model.
