---
group: analyzers
kind: tool
read: 2026-10-20
status: secondary
feeds: [T11]
take: Triad effort model with default parameters (values via a mirror)
---

# carpalx (Krzywinski)

Perl optimizer, 2000s. The classic "typing effort" model. Site: https://mk.bcgsc.ca/carpalx/
(reachable 2026-10-21). Model parameters below were checked against the site's
[model parameters](https://mk.bcgsc.ca/carpalx/?model_parameters) page (set `mod_01`); the SA
settings and the base-effort grid still come from the antibracket mirror,
https://github.com/pianohacker/antibracket/tree/master/generation/carpalx (`effort/*.conf`).
Site content is CC BY-NC-SA 4.0.

## Key ideas

- Scores **triads** (every 3-char window). Effort per triad = `kb·base + kp·penalty + ks·path`.
- Base and penalty compose geometrically over the triad: `k1·x1·(1 + k2·x2·(1 + k3·x3))`.
  Later keys in the triad count less, and a cheap first key damps the rest.
- Penalty per key = `w0 + wh·P_hand + wr·P_row + wf·P_finger`.
- Stroke path: 3-digit code (hand, row, finger pattern) per triad, cost ≈ `1·h + 0.3·r + 0.3·f`.
  Hand: alternation good, all-same-hand bad. Row: monotonic good, jumps bad. Finger: monotonic
  rolls good, repeats/reversals bad.
- Search: SA, `T(i) = T0·exp(-k·i/N)`, accept worse with `p0·exp(-ΔE/T)`.

## Numbers (defaults in `effort/k/01.conf`, `effort/weight/01.conf`)

| Param | Value |
|---|---|
| kb, kp, ks | 0.3555, 0.6423, 0.4268 |
| k1, k2, k3 | 1, 0.367, 0.235 |
| w0, wh, wr, wf | 0, 1, 1.3088, 2.5948 |
| row penalty (num, top, home, bottom) | 1.5, 0.5, 0, 1 |
| finger penalty L (P R M I T) / R | 1 0.5 0 0 0 / 0 0 0 0.5 1 |
| base effort home row | 0 0 0 0 2 2 0 0 0 0 2 (center columns = 2) |
| base effort top / bottom | ~2 (2.5–3 at index stretch) / ~2 (3.5 at B) |
| stroke path weights (hand, row, finger) | 1, 0.3, 0.3 |
| SA | iterations 4096 per stage, t0 = 10, p0 = 0.7, k = 7.5 |

QWERTY ≈ 9 effort units, optimized ≈ 4.5–5 (author's figures).

## How to use in GECKO

- Keep as a separate **profile** (`carpalx`) for comparison, metric T11. Triad composition is
  not a sum over n-gram classes, so fold it as a dense `[N×N×N]` table at scorer build: the
  formula only depends on the three slots, so it is linear in trigram frequencies.
- Do not use as the default: no explicit SFB/SFS term, and its outputs (QGMLWY) lose to modern
  layouts on SFB and rolls (see `keyboard-layouts-doc.md`).
- Its SA advice still holds: scale T0 to the score range (GECKO already samples T0).
