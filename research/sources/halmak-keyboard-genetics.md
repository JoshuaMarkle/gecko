# keyboard-genetics / Halmak (MadRabbit, 2016)

JavaScript GA, MIT. https://github.com/MadRabbit/keyboard-genetics, result
https://github.com/MadRabbit/halmak. A cautionary example.

## Key ideas

- **Simulation fitness:** "type" the corpus key by key, summing per-key effort and distance
  (tables from the author's measurements, `src/config.js`), plus overheads:
  same finger `(prev effort + 1)·10`, same hand `(prev effort + 1)·1`, shift retraction.
  Fitness = how much text is typed before an effort budget (3,000,000) runs out.
- GA over full layouts including shifted symbols.

## Why it matters

- Halmak scores well on its own model but has high SFB under modern analyzers. A coarse
  same-finger penalty that ignores distance and skipgrams yields layouts the community rejects.
- Shows the failure mode GECKO must test for: **optimizing a thin objective produces
  confident nonsense.** Calibrate against several analyzers and the community ranking.

## How to use in GECKO

- Keep Halmak in `data/layouts/` as a ranking check (already present).
- The "budget" formulation is equivalent to minimizing average effort; nothing to port.
