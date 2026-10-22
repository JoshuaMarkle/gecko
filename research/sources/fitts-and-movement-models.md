---
group: human
kind: paper
read: 2026-10-20
status: secondary
feeds: [B03]
take: Fitts only for same-finger travel and thumbs
---

# Fitts-style movement models for key-to-key time

- Zhai, Hunter, Smith, "Performance Optimization of Virtual Keyboards", *HCI* 17(2–3), 2002,
  doi:10.1207/S15327051HCI172&3_4 (earlier IBM report "The Metropolis Keyboard", 1999).
- Bi, Li, Zhai, "FFitts law: modeling finger touch with Fitts' law", CHI 2013.
- Soukoreff & MacKenzie, "Theoretical upper and lower bounds on typing speed using a stylus
  and soft keyboard", *BIT* 1995 (digraph-weighted Fitts model).
- Dell'Amico et al. 2009 used `MT = α + β·log2(D/W + 1)` with α = 0, β = 10/49
  (`dellamico-single-finger-qap.md`).

## Key ideas

- For single-pointer input, time per digraph ≈ Fitts time between key centres; layout cost =
  Σ digraph frequency × Fitts time = a QAP.
- Metropolis keyboard: Metropolis random walk on a "Fitts-digraph energy" → ~42.5 WPM predicted
  (50 % over QWERTY on a soft keyboard). Also a physics ("Hooke") spring method, 41.6 WPM.
- FFitts adds finger-touch imprecision to the target width term.

## Relevance to ten-finger physical typing

- Weak: each finger moves only short distances, many movements overlap (rollover), and
  same-finger transitions dominate the timing cost. Fitts applies to the **same-finger travel
  time** (SFB distance cost) and to **thumb reaches**, not to alternating-hand bigrams.
- Use the log form as one option for B03's distance transform (`dist`, `dist²`, `log2(dist/W + 1)`)
  and let calibration (136M, Engram) choose.
