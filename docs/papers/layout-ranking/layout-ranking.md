# Ranking the modern layouts: what the evidence-fitted profile says about 33 keyboard layouts

*Experiment report · 24 October 2026 · GECKO commit e4a88c4 · profile `default` (the evidence-fitted weights of 24 October) · ansi-30 · `english` corpus. Data and scripts: `bench/layout-ranking-2026-10-24/`.*

## Abstract

We rank 31 named layouts under GECKO's new default profile. They are the community's popular cmini layouts plus the classic references, scored with their own symbols. Its weights were fitted to 136M keystrokes of timing and Engram's comfort study. The GECKO optimum for that profile is included as a yardstick. **The Gallium/Graphite family wins:** Gallium, Galite, Maya and Graphite score 11.8–12.1, and the robust ranking cannot separate them. Dhorf, CTGAP, Nerps, Focal and Noctum follow at 12.9–13.8. The profile change matters less than the family. Layouts move by at most a few places, except Hands Down (20th → 10th) and Pine v4 (13th → 20th), because the new weights pay for hand alternation and measured comfort and barely charge for same-finger skipgrams. Two design styles lose ground: layouts built for inward rolls (APTv3, Rain, Whirl, Panke) and roll-heavy, one-hand-loaded layouts (Canary, Sturdy, Stand). A micro experiment polishes each layout by steepest-descent swaps. Every layout improves. The best single swap is worth 4–20 %, and three swaps are worth 10–45 %. Polishing lowers the top four's scores by 21–32 %, and the GECKO optimum for their symbol set is 29–32 % lower. Most of that headroom is a handful of moves on the vowel block and on the rare consonants. The common first fix is a vowel swap (`a↔e` or `e↔i`) that trades pinky–ring pairs and SFB for comfort. All of this is a model's opinion: typing time barely separates the top 20 (E01 6.8–7.8 %), so comfort, as one 61-person study measures it, decides most of the ranking.

## Setup

**Scoring.** Score is GECKO's weighted cost under `data/profiles/default.toml` (lower is better). The profile was found on 24 October by a bilevel search. It costs SFB (+17 % time per pair), pinky–ring pairs and bad redirects. It pays for hand alternation (−8 % per alternating bigram) and for rolls, outward slightly more than inward. Measured Engram discomfort (E05) is its second-largest term. Same-finger skipgrams, trigram alternation and plain redirects are nearly free (`research/experiments/2026-10-24-cost-function-optimization.md`). The previous default, now `balanced`, gives the "old rank" column.

**Layouts.** Seven classics (QWERTY, Colemak, Colemak-DH, Dvorak, Workman, Halmak, Canary) and the 27 most-liked modern layouts in `data/community/cmini.json`. Graphite uses the reference rows: cmini's 30-key copy drops the comma, which would flatter it. Vylet (a `*` repeat key) and Curtsy (no comma) are left out of the ranking, because a layout missing a frequent symbol looks cheaper than it is. Five layouts are designed for the angle mod. They are marked † and scored on `ansi-30-angle`, the fingering their authors intended. Canary appears twice, ortho and angle.

**Symbol sets.** The four punctuation keys change the score: `'` is frequent, `;` and `/` are rare. Each layout is also compared with the GECKO optimum over its own set: `qwerty` (`, . / ;`), `apos` (`' , . /`) or `semi` (`' , . ;`). These optima are 8.07, 8.21 and 8.40 on ansi-30, and 7.92, 8.08 and 8.28 on ansi-30-angle (BLS, 25 M evaluations, seed 1). The letters are identical across the three sets on each board.

**Judges.** E01 is the timing model GECKO never optimizes (predicted keystroke time, % of a median interval). E05 is Engram's measured discomfort. It is in the objective, so it is not an independent check. Corpus robustness is each layout's mean rank over `english`, `shai`, `reddit` and `mt-450k`. Rank confidence comes from `gecko compare --robust` (2,000 draws, weight noise σ 0.5, ansi-30 layouts only).

**Micro experiment (polish).** From each layout, take the best of all 435 swaps, repeat until no swap helps, and record the path. The first swap, the first three swaps and the endpoint answer "could it be better, and how cheaply?"

## The ranking

| # | layout | set | score | old # | E01 time | E05 discomfort | SFB % | alt. % | rolls in/out % | corpus rank |
|---:|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| – | *GECKO optimum* | qwerty | *8.07* | – | *6.54* | *4.04* | *0.95* | *70* | *15/30* | – |
| 1 | Gallium | semi | 11.81 | 2 | 6.82 | 5.21 | 0.88 | 67 | 19/27 | 2.5 |
| 2 | Galite | qwerty | 11.85 | 3 | 6.82 | 5.19 | 1.12 | 67 | 20/26 | 3.0 |
| 3 | Maya † | apos | 11.93 | 1 | 6.85 | 5.46 | 0.83 | 67 | 19/27 | 5.0 |
| 4 | Graphite | apos | 12.13 | 4 | 6.84 | 5.25 | 1.16 | 67 | 20/26 | 5.2 |
| 5 | Dhorf | semi | 12.90 | 5 | 7.11 | 5.32 | 0.67 | 67 | 20/23 | 3.8 |
| 6 | CTGAP | apos | 13.02 | 7 | 7.08 | 5.18 | 1.04 | 66 | 17/27 | 7.2 |
| 7 | Nerps | semi | 13.09 | 8 | 7.18 | 5.54 | 1.17 | 67 | 18/28 | 7.0 |
| 8 | Focal | semi | 13.19 | 10 | 7.31 | 5.36 | 0.73 | 66 | 22/23 | 6.5 |
| 9 | Noctum | semi | 13.81 | 6 | 7.18 | 5.71 | 0.82 | 68 | 26/18 | 6.5 |
| 10 | Hands Down (ref) | apos | 14.68 | 20 | 7.43 | 5.26 | 0.87 | 67 | 24/21 | 10.8 |
| 11 | Panke † | qwerty | 14.82 | 11 | 7.57 | 5.62 | 0.98 | 63 | 33/14 | 10.8 |
| 12 | HRT | semi | 15.36 | 9 | 7.05 | 6.37 | 0.95 | 65 | 23/21 | 14.8 |
| 13 | Whorf | semi | 15.73 | 18 | 7.48 | 6.02 | 0.59 | 66 | 22/24 | 12.5 |
| 14 | Semimak-JQ | apos | 16.97 | 17 | 7.36 | 5.81 | 0.94 | 66 | 20/23 | 13.2 |
| 15 | MTGAP | semi | 17.16 | 22 | 7.45 | 5.11 | 1.17 | 65 | 30/16 | 20.2 |
| 16 | Sturdy | semi | 17.40 | 12 | 7.60 | 6.54 | 0.89 | 62 | 22/27 | 15.0 |
| 17 | Canary (angle) † | qwerty | 17.50 | 14 | 7.71 | 7.21 | 0.74 | 60 | 23/27 | 15.2 |
| 18 | Stand † | semi | 17.96 | 16 | 7.78 | 6.95 | 0.78 | 62 | 23/29 | 16.5 |
| 19 | APTv3 | apos | 18.19 | 15 | 7.49 | 6.37 | 1.24 | 63 | 34/14 | 21.5 |
| 20 | Pine v4 | semi | 18.47 | 13 | 7.31 | 6.72 | 0.95 | 64 | 19/25 | 18.0 |
| 21 | Canary | qwerty | 18.65 | 23 | 7.94 | 7.21 | 0.97 | 60 | 22/27 | 18.8 |
| 22 | ISRT | apos | 18.90 | 21 | 7.60 | 7.49 | 0.75 | 59 | 22/27 | 24.0 |
| 23 | Stronk | semi | 19.09 | 24 | 7.58 | 7.08 | 1.07 | 65 | 22/24 | 20.2 |
| 24 | Recurva | semi | 20.26 | 19 | 7.80 | 7.50 | 0.93 | 63 | 22/28 | 23.2 |
| 25 | Rain | semi | 20.78 | 25 | 8.08 | 7.50 | 0.89 | 59 | 34/18 | 25.2 |
| 26 | Dvorak | semi | 20.85 | 26 | 7.29 | 6.13 | 2.58 | 70 | 26/14 | 26.2 |
| 27 | Whirl | semi | 22.46 | 27 | 8.14 | 6.68 | 1.57 | 62 | 34/16 | 26.0 |
| 28 | Halmak | qwerty | 25.24 | 30 | 8.06 | 6.30 | 2.75 | 68 | 20/19 | 28.8 |
| 29 | Colemak-DH | qwerty | 27.15 | 28 | 8.17 | 8.52 | 1.54 | 57 | 24/22 | 27.8 |
| 30 | Colemak | qwerty | 28.69 | 29 | 8.20 | 8.82 | 1.54 | 57 | 24/22 | 29.5 |
| 31 | Workman | qwerty | 30.99 | 31 | 9.11 | 8.33 | 3.00 | 55 | 23/21 | 31.0 |
| 32 | QWERTY | qwerty | 64.61 | 32 | 13.06 | 12.23 | 6.63 | 53 | 21/17 | 32.0 |

Ranks exclude the GECKO optimum, Vylet and Curtsy; old # is the rank under `balanced`. Lower is better for every column except alternation and rolls. † = angle-mod layout, scored on ansi-30-angle.

![Speed and comfort judges. E01 (time) is unweighted in the profile; E05 (comfort) is weighted. The modern layouts lie on one diagonal: on this data, comfortable layouts are also fast ones.](assets/judges.pdf)

**Reading the ranking.**

- **Top cluster.** Under weight noise, the rank intervals (counting the optimum as 1; `robust.json`) are 2–5 for Gallium and Galite and 3–7 for Graphite. Dhorf ranges 2–8, and CTGAP, Nerps, Focal and Noctum 4–10. Treat 1–4 as a tie and 5–9 as a tie.
- **Speed hardly separates layouts; comfort does.** The 20 best layouts span 6.8–7.8 % on E01. E05 spans 5.1–7.2. Most of the score gap between the top and the middle of the table is comfort, alternation and pinky–ring pairs, not SFB. All of the top 20 have SFB under 1.25 %.
- **Corpus robustness matches the English rank.** Gallium, Galite and Dhorf are top 4 on average across corpora. Dhorf is 1st on `mt-450k` (rare vocabulary), the only corpus where the top cluster is reshuffled. MTGAP, APTv3 and HRT do clearly worse off `english`.
- **The angle mod is worth about one point for Canary** (18.65 → 17.50). Its left bottom-row keys stop being stretched.

## What changed with the new weights

The new profile scales up three terms: hand alternation (B08, new), comfort (E05, ×3.3) and finger load. It nearly zeroes out three others: same-finger skipgrams, trigram alternation and plain redirects. The winners and losers follow from that:

- **Up:** Hands Down (+10 places), MTGAP (+7), Whorf (+5) and Focal (+2). All have high alternation or low E05, and they paid under `balanced` for SFS or redirects that no longer cost anything.
- **Down:** Pine v4 (−7), Recurva (−5), Sturdy (−4), APTv3 (−4), Noctum (−3), HRT (−3) and Canary-angle (−3). They are roll-first designs with a lopsided hand split or a poor E05. Their strength (rolls, low SFS) is now cheaper than their weakness (comfort).
- **Not moved:** the top four, the classics and QWERTY. The family that wins under both profiles is the one that is good on everything at once.

![Fingerprint: each layout's weighted cost per metric minus the median modern layout (blue = better than median, orange = worse). It shows what each layout optimizes for and what it pays.](assets/fingerprint.pdf)

## Layout by layout

Per layout: what it optimizes for (from its metrics and design), what makes it good, the first fix the polish finds, and why it turned out as it did. Design history is kept to what is well documented; otherwise the description comes from the numbers.

**GECKO optimum** (`wgncvzuo,q / dtrsfyheia / pblmjxk/;.`). *Optimizes:* exactly this profile. *Good:* best E05 (4.04) and E01 (6.54) here, 70 % alternation, the most outward rolls (30 %). *Pays:* SFS 8.5 % and the fewest inward rolls (15 %). *Why:* every vowel and `y h` sit on the right hand and the consonant columns on the left. That shape follows directly from paying for alternation and comfort while ignoring SFS. Its win is partly circular, since it is scored by the function it was searched under. The fair comparison is E01, where it is 0.3 points ahead.

**Gallium (1), Galite (2), Graphite (4), Maya (3 †): the gallium/graphite family.** *Optimizes:* low SFB with a balanced profile everywhere: `nrts` or `rnts` on the left home row, `haei` on the right. *Good:* the best E05 of the human layouts (5.2), pinky–ring and alternation better than the median, and top 5 on every corpus. *Could be better:* the first swap is in the vowel block. Gallium and Maya swap `a↔e` (−1.35 points: fewer pinky–ring pairs and stretches, more outward rolls). Graphite and Galite swap `e↔i` (−1.1: SFB 1.16 → 0.92 %, better comfort, more pinky–ring pairs). Polished, Maya lands exactly on the GECKO angle optimum, and Gallium reaches 8.58. *Why:* Graphite (stronglytyped, 2023) and Gallium (GalileoBlues, 2023) were tuned in community analyzers (cmini, oxeylyzer and similar) to be good on SFB, SFS, rolls and redirects together. "Good at everything, extreme at nothing" is what any reasonable weighting rewards.

**Dhorf (5), Noctum (9) and Stronk (23), by oxey; Focal (8); Whorf (13).** *Optimizes:* very low SFB (Whorf 0.59 %, Dhorf 0.67 %, Focal 0.73 %, the lowest here). *Good:* SFB, and Dhorf has a strong E05. *Could be better:* Dhorf `a↔e`; Noctum `r↔h` (it is out-roll poor: 26 % in, 18 % out); Whorf `v↔z`, then `h↔n` (stretch and key effort). *Why:* minimizing SFB first pushes common letters onto awkward keys. The new profile values SFB only as much as the timing data does (+17 % per pair). Below about 1 %, the remaining SFB is worth less than the comfort it costs.

**CTGAP (6), Nerps (7).** *Optimizes:* comfort and pinky–ring avoidance. Nerps has one of the best pinky–ring terms. *Good:* E05 ≈ 5.2–5.5. *Could be better:* CTGAP's hand balance (`j↔f`); Nerps's SFB 1.17 % (`p↔f`, `x↔b`). Polished, Nerps reaches 8.58, as good as polished Gallium.

**Hands Down reference (10).** *Optimizes:* a thumb-letter board. This 30-key variant has no thumb letter, so it is scored outside its design point. *Good:* E05 5.26, among the best comfort scores, which is why it jumped 10 places. *Could be better:* stretch (the worst B14 in the top 20); first fix `g↔f`.

**Panke (11 †), APTv3 (19), Rain (25), Whirl (27): the inward-roll school.** *Optimizes:* inward rolls (33–34 % in vs 14–18 % out, the most of any layout here). *Good:* the fingerprint's best "rolls in" column. *Could be better:* `h↔n` is the first or second fix for Panke, APTv3 and Rain. That one swap converts about 17 points of inward rolls into outward ones and improves comfort (APTv3 −3.3 points). *Why:* the community tradition holds that inward rolls feel best. The Aalto timing data shows outward rolls speed the next keystroke more (−1.4 % vs −0.3 %), so the fitted profile pays for both but slightly prefers outward ones. If inward rolls really feel better, that is a comfort fact Engram's bigram study did not measure. This is the clearest case where GECKO's evidence and community taste disagree.

**Sturdy (16), Canary (21 / 17 †), Stand (18 †), Recurva (24), ISRT (22), Pine v4 (20).** *Optimizes:* rolls and low SFB with a left-heavy or vowel-split arrangement. Canary is 42 % left hand, and ISRT puts `i` and `o` on opposite pinkies. *Good:* SFB 0.74–0.97 %, many rolls (most of them 49–51 %). *Could be better:* the polish fixes hand balance and comfort: Canary `j↔m` (hand balance 5.6 → 2.6, E05 −0.8), Sturdy `f↔z`, ISRT `u↔i`. Recurva, ISRT and Pine v4 are among the moderns with the most headroom (45–51 % by full polish). *Why:* these are 2022-era roll optimizers. Their analyzers did not weight measured comfort, and E05 now costs them 1–2.5 points against the median.

**Semimak-JQ (14).** *Optimizes:* the lowest SFS here (5.6 %, tied with Recurva and Whirl) with low SFB. *Good:* an SFB/SFS pair that was state of the art in 2021. *Could be better:* pinky–ring (+1.2 vs the median); first fix `y↔,`. *Why:* built in semi's genkey analyzer, where SFS mattered. The new profile finds no measurable SFS cost, so its main advantage no longer counts.

**MTGAP (15).** *Optimizes:* Michael Dickens's own typing-effort model, an early (around 2010) computer search. *Good:* the best E05 of any human layout (5.11) and Engram formula E04 4.48. *Why it sits mid-table:* pinky–ring (+2.3) and bad redirects (+1.5). It is the layout closest to a local optimum here: only 4 improving swaps (17.2 → 14.8). It is a clean optimum of a different cost function.

**Dvorak (26), Halmak (28).** *Optimize:* hand alternation, the most of any human layout (68–70 %). *Good:* alternation and home-row use. Dvorak has a better E05 than Canary, Sturdy or Rain. *Could be better:* SFB 2.6–2.8 %, which polish removes fast (Dvorak `h↔s`; Halmak `d↔,`). *Why:* Dvorak (1936) put the vowels on the left home row to force alternation, which the new profile likes. Halmak (2016) was generated by an evolutionary search. Both predate SFB as the community's first metric.

**Colemak-DH (29), Colemak (30), Workman (31), QWERTY (32).** *Optimize:* Colemak keeps QWERTY's shortcut keys and most positions (familiarity), and the DH mod pulls D and H off the centre column. Workman follows a key-effort map that avoids lateral index reaches. QWERTY descends from the 1870s typewriter. *Good:* Colemak's 71 % home row; Workman's effort and finger load. *Could be better:* everything. The first fix for both Colemaks is `a↔i`. Workman's 3.0 % SFB is the price of its effort map. *Why:* none of them optimize SFB, alternation or comfort jointly.

## Micro experiment: how far is each layout from a local optimum?

![Each layout as published (filled) and after steepest-descent polish to a local optimum (open). The dashed line is the GECKO optimum. QWERTY (64.6 → 9.75) is omitted for scale.](assets/polish.pdf)

| layout | best 1 swap | best 3 swaps | full polish (swaps, keys moved) | first three swaps |
|---|---:|---:|---|---|
| Gallium | −11 % | −16 % | −27 % (11, 14) | `a↔e` `b↔v` `a↔i` |
| Galite | −8 % | −14 % | −21 % (12, 14) | `e↔i` `z↔f` `y↔p` |
| Maya † | −9 % | −16 % | −32 % (13, 17) → GECKO angle optimum | `a↔e` `g↔v` `q↔f` |
| Graphite | −9 % | −15 % | −23 % (13, 15) | `e↔i` `z↔f` `y↔p` |
| Dhorf | −7 % | −13 % | −16 % (7, 12) | `a↔e` `k↔w` `q↔b` |
| Nerps | −13 % | −22 % | −34 % (11, 15) | `p↔f` `x↔b` `;↔,` |
| Semimak-JQ | −6 % | −22 % | −35 % (8, 10) | `y↔,` `'↔y` `z↔w` |
| MTGAP | −7 % | −13 % | −14 % (4, 8) | `o↔u` `j↔q` `l↔w` |
| Sturdy | −7 % | −11 % | −23 % (12, 17) | `f↔z` `c↔y` `.↔b` |
| APTv3 | −18 % | −27 % | −42 % (15, 19) | `h↔n` `l↔j` `z↔.` |
| Canary | −16 % | −26 % | −32 % (9, 15) | `j↔m` `y↔f` `d↔g` |
| Colemak-DH | −6 % | −25 % | −32 % (10, 13) | `a↔i` `q↔y` `,↔/` |
| Dvorak | −10 % | −22 % | −42 % (21, 23) | `h↔s` `u↔i` `s↔t` |
| QWERTY | −20 % | −39 % | −85 % (27, 26) | `e↔l` `r↔d` `j↔n` |

Findings:

1. **No published layout is a local optimum of this profile.** Every one improves by at least 4 % with one swap. That is expected: none was designed under these weights. The size of the gap is the interesting part.
2. **The headroom is concentrated.** For the top 10, three swaps recover more than half of the full-polish gain. The moves touch the vowel block (`a e i`) and rare consonants (`z q j v b`).
3. **Local polish does not reach the global optimum.** Maya is the exception: it lands on the angle optimum exactly. Gallium and Nerps stop at 8.58 and Graphite at 9.34, against 8.07. Endpoints range from 8.1 to 14.8 whatever the starting quality: QWERTY polishes to 9.75, better than polished Canary (12.66). The landscape has many funnels, and the start decides which one a descent falls into.
4. **The best family is close to GECKO's answer.** Gallium's polish needs 11 swaps and ends 0.5 points above the optimum. The structure (`nrts`-style consonant home, vowels on the right) is already there; the remaining differences are local.

## Limits

- **One profile, one corpus, one board.** The ranks are `default` on ansi-30 with `english`. Angle-mod layouts are on ansi-30-angle, so their scores are close to, but not strictly comparable with, the ortho ones. The corpus audit (`reports/English corpus fitness.md`) notes that `english` is ten novels; the cross-corpus column partly addresses this.
- **E05 is both objective and judge.** Comfort rests on Engram's 61-person, 12-key study. Inward-roll and SFS preferences that study did not measure could reverse several rankings in Sections 3 and 4.
- **Timing differences are small.** E01 separates the top 20 by under 1 point (% of an interval). A ranking driven by comfort is only as good as the comfort data.
- **Design history is brief and partly inferred.** For less-documented layouts, "what it optimizes for" is read from the metrics, not from the author's notes.
- **The polish is greedy steepest descent with swaps only.** Larger moves (cycles, column swaps) would find more.

## Reproduce

```
python3 bench/layout-ranking-2026-10-24/gather.py bench/layout-ranking-2026-10-24/data.json
python3 bench/layout-ranking-2026-10-24/analyze.py
gecko optimize --algo bls --evals 25000000 --seed 1 --from <canary|graphite|sturdy> [-g ansi-30-angle]
gecko compare <ansi-30 layouts as raw:...> --robust --draws 2000 --seed 1 --json
MPLCONFIGDIR=/tmp/gecko-mpl python3 -I docs/papers/layout-ranking/figures.py
python3 -I docs/papers/layout-ranking/build.py
```

The set-optimum runs are `runs/20261024-2321{44,49,54,59}-bls-*` and `runs/20261024-2326{01,07}-bls-*`. Their best layouts are copied to `opt-*.json`.
