# The best English layout under the evidence-fitted default

| | |
|---|---|
| Date | 2026-10-24 / 25 |
| Question | Which arrangement of 26 letters and four punctuation keys on ansi-30 is best for English under `default`, how robust is it, and how far ahead of the community layouts? |
| Answer | One layout from every seed (20/20 in three punctuation sets): `cwdlx jyou. / gstnv pheia / fbmrq zk'/,`, 18 % below Gallium and 23 % below Graphite in score, certified optimal over its 16 most frequent letters, stable under weight noise (84 of 104 perturbed optima in its family, median 2 keys moved). But the fitted time model puts it only 0.6 % ahead of Gallium per keystroke, and the two held-out timing judges disagree, so no speed claim. The text source moves the optimum more than seeds or weights. |
| Command | `bench/english-layout-2026-10-24/README.md` (`grid.sh`, `post.sh`, `gather.py`) |
| Data | `prose` (web 50, chat 30, books 20) and its parts; 17 named layouts; profiles `default`, `balanced`, `sfb`, `engram` |
| Artifacts | `bench/english-layout-2026-10-24/` (`data.json`, `runs/`), paper `docs/papers/english-layout/` |

## Findings

- **Optimum:** vowels and `h` on the right home row, consonants left; 70 % hand alternation;
  outward rolls 31 % vs inward 12 %; Engram measured discomfort 3.98 (best community 5.2–5.4).
  It pays SFB 1.18 % (Gallium 0.92 %).
- **Robustness:** `certify` proves it optimal with 14 and 16 letters free; 18 did not finish in
  10 minutes. The weights that move it most: `sfb`, `outroll`, `engram`, `inroll`.
- **Text:** `prose-web` = `shai`, `prose-chat` = `reddit`, `prose-books` = `english` optima
  (punctuation and case do not move the letters); single-source optima cost 4–6 % on the mix.
- **Negative:** the timing judges J_B and J_B+letters give −2.5 % vs +2.2 % against QWERTY for
  the same layouts (a practice confound); under `sfb` the low-SFB community layouts win.
