# genkey and Semimak (semilin)

Go analyzer/generator (GPL-3) that produced **Semimak** (2021). Repo:
https://github.com/semilin/genkey (mirror https://github.com/kuuuube/genkey). Blog posts:
https://semilin.github.io/blog/ (source: https://github.com/semilin/semilin.github.io,
`src/blog/semimak.org`, `reflection-on-semimak.org`, `final-reflection-on-semimak.org`,
`layout-quality.org`).

## Key ideas

- **Finger speed (fspeed):** per finger, `Σ (w_sfb·SFB + w_dsfb·DSFB) · dist / KPS[finger]`,
  scaled `800·x / total`. Pioneered weighting SFBs by distance and finger dexterity, and
  counting **same-finger skipgrams**.
- **Skipgram decay:** skipgram at gap g (0 = bigram) weighted `1/2^g`, up to 10 chars back.
- **Distance:** `lateral·dx² + dy²` (squared, lateral 1.4×), plus `2·KeyTravel` (0.01) per press.
  Squared distance = "2 units is much worse than 1".
- **KPS per finger** (keys/s from a tapping test, e.g. `eded…`): pinky 1.5, ring 3.6,
  middle 4.8, index 5.5. Personal dexterity → personal layout.
- **Dynamic fspeed:** removes the worst SFB per finger, assuming the user alt-fingers it.
- Score = 3·fspeed + 0.3·index balance + 1·LSB (+ optional trigram terms over the top 100).
- **Search:** 1000 random layouts → greedy random-swap improvement until 500 failed swaps →
  keep top 100 → "full improve" with multi-swap moves → post-process: swap top/bottom rows
  per column so the more frequent letter is on top.

## Lessons from the author

- Semimak minimized movement but had **very high redirects**; columns were swapped by hand to
  fix that. Metrics you do not optimize get worse.
- Ring-finger load was the biggest complaint: dexterity differs between people. Pinky load was
  learnable; ring load was not for many.
- Specific frequent words matter (`you're`, `I'll`): check worst words, not just aggregates.
- "What matters is not how well the layout fits the metrics, it's how well the metrics fit you."
- Purpose of alt layouts is comfort, not speed.

## How to use in GECKO

- B03 already covers distance-weighted SFB. Add: skipgram decay `1/2^g` (S02), per-finger
  KPS divisors, optional squared distance. All fold into C2 / skipgram C2k tables.
- Add a **personal dexterity calibration**: a 30-second tapping test per finger (web UI) →
  KPS table → profile. Cheap and directly useful.
- Report **worst words** per metric, not only worst n-grams.
