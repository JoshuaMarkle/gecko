# Roadmap

The feature backlog in the order to build it. Source of truth: GitHub issues #1–103 (#58 and #65 closed as superseded) and the project [GECKO](https://github.com/users/JoshuaMarkle/projects/5). Each issue has a goal, steps, done-when and blocked-by links.

**Picking work:** take the lowest-numbered open item whose blockers are closed. No dates: items are done one after another. Size (S/M/L) is relative effort only.

When an item closes, tick it here. When scope changes, edit the issue first.

## Tracks

| Track | Scope |
|---|---|
| A · Foundations | Testing, CI, speed. Makes unattended agent work safe. |
| B · Search | New optimizers, diversity, certificates. |
| C · Objective v2 | Better cost models and calibration (phase 3). |
| D · Symbols & layers | Finish phase 5: Shift, one-shot layers, symbol metrics. |
| E · Data & interop | Board import, more boards and corpora. |
| F · Web UI | Views for the new features. |
| G · 2D geometry | Phase 6: optimize key x, y, rotation on a plane. |
| H · Hand model & 3D | Phase 7: hand model, IK reach costs, z/tilt/tent. |
| I · Free-form 3D | Phase 8: per-key 6-DoF poses, thumb clusters. |
| J · Physical output | Turn an optimized board into something buildable. |
| K · Learned costs | Phase 9: timing models, personal calibration. |
| L · App workflow | Web app tabs follow the design process: Board, Hand, Corpus, Profile, Search, Optimize, Explore, Stats, Runs. |

## Sequence

1. [ ] [#1](https://github.com/JoshuaMarkle/gecko/issues/1) `A1` CI: GitHub Actions for Rust and web checks · S
2. [ ] [#7](https://github.com/JoshuaMarkle/gecko/issues/7) `A2` Metric unit tests with hand-computed answers · M
3. [ ] [#12](https://github.com/JoshuaMarkle/gecko/issues/12) `A3` insta snapshots of reference layout scores · S
4. [ ] [#16](https://github.com/JoshuaMarkle/gecko/issues/16) `A4` Automated WASM == CLI test · S · after [#1](https://github.com/JoshuaMarkle/gecko/issues/1)
5. [ ] [#18](https://github.com/JoshuaMarkle/gecko/issues/18) `A5` criterion benches + bench/history.jsonl · S
6. [ ] [#20](https://github.com/JoshuaMarkle/gecko/issues/20) `A6` Scorer speed: cache per-trigram cost in State · M · after [#18](https://github.com/JoshuaMarkle/gecko/issues/18)
7. [ ] [#28](https://github.com/JoshuaMarkle/gecko/issues/28) `A7` Validate trigram classes and pair distances against external data · S
8. [ ] [#78](https://github.com/JoshuaMarkle/gecko/issues/78) `A8` `gecko verify run:<id>` · S
9. [ ] [#79](https://github.com/JoshuaMarkle/gecko/issues/79) `A9` Issue and PR templates for agent work · S
10. [ ] [#80](https://github.com/JoshuaMarkle/gecko/issues/80) `A10` Claude Code GitHub Action · S · after [#1](https://github.com/JoshuaMarkle/gecko/issues/1)
11. [ ] [#81](https://github.com/JoshuaMarkle/gecko/issues/81) `A11` Release builds with the web UI embedded · S · after [#1](https://github.com/JoshuaMarkle/gecko/issues/1)
12. [ ] [#2](https://github.com/JoshuaMarkle/gecko/issues/2) `B1` Tabu search (robust tabu, RoTS) optimizer · M
13. [ ] [#3](https://github.com/JoshuaMarkle/gecko/issues/3) `C1` B11 finger-pair movement table · M
14. [ ] [#4](https://github.com/JoshuaMarkle/gecko/issues/4) `D1` Model Shift · M
15. [ ] [#5](https://github.com/JoshuaMarkle/gecko/issues/5) `E1` KLE JSON importer · M
16. [ ] [#6](https://github.com/JoshuaMarkle/gecko/issues/6) `H1` Verify hand-model parameters · S
17. [ ] [#82](https://github.com/JoshuaMarkle/gecko/issues/82) `L1` Navbar restructure: inputs become top-level tabs · M
18. [ ] [#8](https://github.com/JoshuaMarkle/gecko/issues/8) `B2` More neighbourhoods: 3-cycle, column swap, depth-2 swaps · S
19. [ ] [#9](https://github.com/JoshuaMarkle/gecko/issues/9) `C2` S04 skipgram LSB / scissor / stretch · S
20. [ ] [#10](https://github.com/JoshuaMarkle/gecko/issues/10) `D2` One-shot and toggle layers · M
21. [ ] [#11](https://github.com/JoshuaMarkle/gecko/issues/11) `E2` More boards: Lily58, Kyria, Ergodox, Dactyl-Manuform 5x6 · S · after [#5](https://github.com/JoshuaMarkle/gecko/issues/5)
22. [ ] [#83](https://github.com/JoshuaMarkle/gecko/issues/83) `L9` Flatten any board to a 2D view · M
23. [ ] [#13](https://github.com/JoshuaMarkle/gecko/issues/13) `B3` Memetic algorithm (GA + local search, islands) · M · after [#8](https://github.com/JoshuaMarkle/gecko/issues/8)
24. [ ] [#14](https://github.com/JoshuaMarkle/gecko/issues/14) `C3` Asymmetric row costs per finger · S
25. [ ] [#15](https://github.com/JoshuaMarkle/gecko/issues/15) `E3` Chat and multilingual corpora · S
26. [ ] [#84](https://github.com/JoshuaMarkle/gecko/issues/84) `L2` Board tab: board catalog, current board, start layout · M · after [#82](https://github.com/JoshuaMarkle/gecko/issues/82), [#11](https://github.com/JoshuaMarkle/gecko/issues/11)
27. [ ] [#17](https://github.com/JoshuaMarkle/gecko/issues/17) `C4` Remaining P1/P2 metrics · M
28. [ ] [#85](https://github.com/JoshuaMarkle/gecko/issues/85) `E4` Import layouts from Oxeylyzer, genkey and cmini formats · S
29. [x] [#19](https://github.com/JoshuaMarkle/gecko/issues/19) `D3` Y03 paired symbol placement · S
30. [ ] [#86](https://github.com/JoshuaMarkle/gecko/issues/86) `E5` Export layout image (SVG/PNG) · S · after [#83](https://github.com/JoshuaMarkle/gecko/issues/83)
31. [ ] [#87](https://github.com/JoshuaMarkle/gecko/issues/87) `L10` Mini keyboards everywhere · M · after [#83](https://github.com/JoshuaMarkle/gecko/issues/83)
32. [ ] [#21](https://github.com/JoshuaMarkle/gecko/issues/21) `B4` NSGA-II Pareto fronts · M
33. [ ] [#22](https://github.com/JoshuaMarkle/gecko/issues/22) `D4` Y04 home-row-mod misfire risk · S
34. [ ] [#23](https://github.com/JoshuaMarkle/gecko/issues/23) `C5` Worst words report per metric · S
35. [ ] [#24](https://github.com/JoshuaMarkle/gecko/issues/24) `D5` Thumb trigram classes + M09 thumb load · S
36. [ ] [#25](https://github.com/JoshuaMarkle/gecko/issues/25) `G1` Parametric 2D geometry model · M · after [#3](https://github.com/JoshuaMarkle/gecko/issues/3), [#17](https://github.com/JoshuaMarkle/gecko/issues/17)
37. [ ] [#26](https://github.com/JoshuaMarkle/gecko/issues/26) `C6` cmini community layout importer + ranking tests · M
38. [ ] [#27](https://github.com/JoshuaMarkle/gecko/issues/27) `D6` Count the layer key's own keystroke in effort and finger load · S
39. [ ] [#88](https://github.com/JoshuaMarkle/gecko/issues/88) `L4` Corpus tab: stats and sample text · M · after [#82](https://github.com/JoshuaMarkle/gecko/issues/82)
40. [ ] [#29](https://github.com/JoshuaMarkle/gecko/issues/29) `B5` MAP-Elites quality-diversity archive · M
41. [ ] [#30](https://github.com/JoshuaMarkle/gecko/issues/30) `D7` Number and navigation layer optimization · S
42. [ ] [#31](https://github.com/JoshuaMarkle/gecko/issues/31) `K1` E01 IKI timing model from the 136M keystrokes data · L · after [#20](https://github.com/JoshuaMarkle/gecko/issues/20)
43. [ ] [#89](https://github.com/JoshuaMarkle/gecko/issues/89) `D8` Magic / repeat (adaptive) keys · M
44. [ ] [#32](https://github.com/JoshuaMarkle/gecko/issues/32) `G2` Fast cost-table rebuild when keys move · M · after [#25](https://github.com/JoshuaMarkle/gecko/issues/25)
45. [ ] [#33](https://github.com/JoshuaMarkle/gecko/issues/33) `H2` Hand module: skeleton from measurements · M · after [#6](https://github.com/JoshuaMarkle/gecko/issues/6), [#25](https://github.com/JoshuaMarkle/gecko/issues/25)
46. [ ] [#34](https://github.com/JoshuaMarkle/gecko/issues/34) `J1` Export key poses (JSON/CSV) and KLE · S · after [#25](https://github.com/JoshuaMarkle/gecko/issues/25)
47. [ ] [#35](https://github.com/JoshuaMarkle/gecko/issues/35) `C7` Learning-to-rank profile weights · M · after [#26](https://github.com/JoshuaMarkle/gecko/issues/26)
48. [ ] [#36](https://github.com/JoshuaMarkle/gecko/issues/36) `J2` Export to ergogen points · M · after [#25](https://github.com/JoshuaMarkle/gecko/issues/25)
49. [ ] [#90](https://github.com/JoshuaMarkle/gecko/issues/90) `L5` Profile tab: organized weights with live ranking · M · after [#82](https://github.com/JoshuaMarkle/gecko/issues/82)
50. [ ] [#37](https://github.com/JoshuaMarkle/gecko/issues/37) `B6` Lower bound (Gilmore–Lawler) and optimality gap · M
51. [ ] [#38](https://github.com/JoshuaMarkle/gecko/issues/38) `F1` Web: Pareto front and MAP-Elites views · M · after [#21](https://github.com/JoshuaMarkle/gecko/issues/21), [#29](https://github.com/JoshuaMarkle/gecko/issues/29)
52. [ ] [#91](https://github.com/JoshuaMarkle/gecko/issues/91) `D9` Combos and chords · M · after [#10](https://github.com/JoshuaMarkle/gecko/issues/10)
53. [ ] [#39](https://github.com/JoshuaMarkle/gecko/issues/39) `G3` G04 2D collision (OBB, SAT) · S
54. [ ] [#40](https://github.com/JoshuaMarkle/gecko/issues/40) `H3` Finger IK with joint limits · L · after [#33](https://github.com/JoshuaMarkle/gecko/issues/33)
55. [ ] [#92](https://github.com/JoshuaMarkle/gecko/issues/92) `L7` Hand tab: hand parameters with live skeleton · M · after [#82](https://github.com/JoshuaMarkle/gecko/issues/82), [#33](https://github.com/JoshuaMarkle/gecko/issues/33)
56. [ ] [#41](https://github.com/JoshuaMarkle/gecko/issues/41) `C8` L01 shortcut placement and L02 familiarity · S
57. [ ] [#42](https://github.com/JoshuaMarkle/gecko/issues/42) `G4` Geometry-driven costs: G02 travel + 2D reach proxy · M · after [#25](https://github.com/JoshuaMarkle/gecko/issues/25)
58. [ ] [#93](https://github.com/JoshuaMarkle/gecko/issues/93) `L3` Board tab: Unity-style key editing · M · after [#84](https://github.com/JoshuaMarkle/gecko/issues/84), [#39](https://github.com/JoshuaMarkle/gecko/issues/39)
59. [ ] [#94](https://github.com/JoshuaMarkle/gecko/issues/94) `L6` Search tab: algorithm and budget settings · S · after [#82](https://github.com/JoshuaMarkle/gecko/issues/82)
60. [ ] [#43](https://github.com/JoshuaMarkle/gecko/issues/43) `B7` MIP oracle for sub-problems (HiGHS / CP-SAT) · L · after [#37](https://github.com/JoshuaMarkle/gecko/issues/37)
61. [ ] [#44](https://github.com/JoshuaMarkle/gecko/issues/44) `C9` L06 held-out corpus generalization · S
62. [ ] [#45](https://github.com/JoshuaMarkle/gecko/issues/45) `F2` Web: bench views · M
63. [ ] [#46](https://github.com/JoshuaMarkle/gecko/issues/46) `K2` Web: per-finger tapping test · M · after [#35](https://github.com/JoshuaMarkle/gecko/issues/35)
64. [ ] [#95](https://github.com/JoshuaMarkle/gecko/issues/95) `L13` Saved workspaces · S · after [#82](https://github.com/JoshuaMarkle/gecko/issues/82)
65. [ ] [#47](https://github.com/JoshuaMarkle/gecko/issues/47) `C10` Alternate fingering: angle mod geometry · S
66. [ ] [#96](https://github.com/JoshuaMarkle/gecko/issues/96) `L12` Corpus tab: build a corpus from your own files · S · after [#88](https://github.com/JoshuaMarkle/gecko/issues/88)
67. [ ] [#97](https://github.com/JoshuaMarkle/gecko/issues/97) `C11` Optimize finger assignment (alt fingering) · M
68. [ ] [#98](https://github.com/JoshuaMarkle/gecko/issues/98) `F7` Layout diff view · S · after [#83](https://github.com/JoshuaMarkle/gecko/issues/83)
69. [ ] [#48](https://github.com/JoshuaMarkle/gecko/issues/48) `G5` CMA-ES continuous optimizer · M
70. [ ] [#49](https://github.com/JoshuaMarkle/gecko/issues/49) `F3` Web: worst words with finger path animation · S · after [#23](https://github.com/JoshuaMarkle/gecko/issues/23)
71. [ ] [#50](https://github.com/JoshuaMarkle/gecko/issues/50) `K3` Web: pairwise comfort study · M · after [#46](https://github.com/JoshuaMarkle/gecko/issues/46)
72. [ ] [#99](https://github.com/JoshuaMarkle/gecko/issues/99) `L11` Runs tab: gallery and run comparison · M · after [#87](https://github.com/JoshuaMarkle/gecko/issues/87), [#98](https://github.com/JoshuaMarkle/gecko/issues/98)
73. [ ] [#51](https://github.com/JoshuaMarkle/gecko/issues/51) `F4` Web: contribution waterfall vs a reference layout · S
74. [ ] [#52](https://github.com/JoshuaMarkle/gecko/issues/52) `H4` G01 reach cost and G05 press alignment · M · after [#40](https://github.com/JoshuaMarkle/gecko/issues/40), [#32](https://github.com/JoshuaMarkle/gecko/issues/32)
75. [ ] [#53](https://github.com/JoshuaMarkle/gecko/issues/53) `F5` Web: process views (per-worker convergence, temperature/acceptance) · M
76. [ ] [#54](https://github.com/JoshuaMarkle/gecko/issues/54) `G6` Bilevel geometry + layout search · L · after [#32](https://github.com/JoshuaMarkle/gecko/issues/32), [#39](https://github.com/JoshuaMarkle/gecko/issues/39), [#42](https://github.com/JoshuaMarkle/gecko/issues/42), [#48](https://github.com/JoshuaMarkle/gecko/issues/48)
77. [ ] [#55](https://github.com/JoshuaMarkle/gecko/issues/55) `B8` Adaptive fidelity scoring · M
78. [ ] [#56](https://github.com/JoshuaMarkle/gecko/issues/56) `K4` E02 personal keystroke logs · M · after [#31](https://github.com/JoshuaMarkle/gecko/issues/31)
79. [ ] [#57](https://github.com/JoshuaMarkle/gecko/issues/57) `H5` Hand model validation suite · S · after [#52](https://github.com/JoshuaMarkle/gecko/issues/52)
80. [ ] [#100](https://github.com/JoshuaMarkle/gecko/issues/100) `F8` Remaining search-space views · L
81. [ ] [#59](https://github.com/JoshuaMarkle/gecko/issues/59) `H6` 3D parametric params: z, tilt, tent, curvature · M · after [#25](https://github.com/JoshuaMarkle/gecko/issues/25)
82. [ ] [#60](https://github.com/JoshuaMarkle/gecko/issues/60) `B9` Rank-sum selection over many runs (`gecko select`) · S
83. [ ] [#61](https://github.com/JoshuaMarkle/gecko/issues/61) `K5` E03 learned profile weights · M · after [#31](https://github.com/JoshuaMarkle/gecko/issues/31), [#35](https://github.com/JoshuaMarkle/gecko/issues/35)
84. [ ] [#62](https://github.com/JoshuaMarkle/gecko/issues/62) `G7` `gecko geom` CLI · S · after [#54](https://github.com/JoshuaMarkle/gecko/issues/54)
85. [ ] [#63](https://github.com/JoshuaMarkle/gecko/issues/63) `H7` G03 wrist posture cost · S · after [#52](https://github.com/JoshuaMarkle/gecko/issues/52), [#59](https://github.com/JoshuaMarkle/gecko/issues/59)
86. [ ] [#64](https://github.com/JoshuaMarkle/gecko/issues/64) `J3` Export to Cosmos expert config · M · after [#59](https://github.com/JoshuaMarkle/gecko/issues/59)
87. [ ] [#67](https://github.com/JoshuaMarkle/gecko/issues/67) `G9` Web: geometry runs (before/after board overlay) · S · after [#54](https://github.com/JoshuaMarkle/gecko/issues/54)
88. [ ] [#66](https://github.com/JoshuaMarkle/gecko/issues/66) `H8` 3D collision (keycap + switch, GJK) · M · after [#39](https://github.com/JoshuaMarkle/gecko/issues/39)
89. [ ] [#101](https://github.com/JoshuaMarkle/gecko/issues/101) `G10` Switch type: MX vs Choc · S · after [#39](https://github.com/JoshuaMarkle/gecko/issues/39)
90. [ ] [#68](https://github.com/JoshuaMarkle/gecko/issues/68) `H9` Web: posed hand in the 3D viewer · M · after [#40](https://github.com/JoshuaMarkle/gecko/issues/40)
91. [ ] [#69](https://github.com/JoshuaMarkle/gecko/issues/69) `H10` 3D bilevel search · M · after [#59](https://github.com/JoshuaMarkle/gecko/issues/59), [#63](https://github.com/JoshuaMarkle/gecko/issues/63), [#66](https://github.com/JoshuaMarkle/gecko/issues/66), [#54](https://github.com/JoshuaMarkle/gecko/issues/54)
92. [ ] [#102](https://github.com/JoshuaMarkle/gecko/issues/102) `L8` Typing simulation tab · L · after [#92](https://github.com/JoshuaMarkle/gecko/issues/92), [#40](https://github.com/JoshuaMarkle/gecko/issues/40), [#68](https://github.com/JoshuaMarkle/gecko/issues/68)
93. [ ] [#103](https://github.com/JoshuaMarkle/gecko/issues/103) `H11` Optimize for a range of hand sizes · M · after [#69](https://github.com/JoshuaMarkle/gecko/issues/69)
94. [ ] [#70](https://github.com/JoshuaMarkle/gecko/issues/70) `I1` Free-form per-key 6-DoF poses · L · after [#69](https://github.com/JoshuaMarkle/gecko/issues/69)
95. [ ] [#71](https://github.com/JoshuaMarkle/gecko/issues/71) `I2` Thumb cluster optimization (G06) · M · after [#52](https://github.com/JoshuaMarkle/gecko/issues/52)
96. [ ] [#72](https://github.com/JoshuaMarkle/gecko/issues/72) `I3` Collision repair step · S · after [#66](https://github.com/JoshuaMarkle/gecko/issues/66)
97. [ ] [#73](https://github.com/JoshuaMarkle/gecko/issues/73) `I4` G07 manufacturability constraints · M
98. [ ] [#74](https://github.com/JoshuaMarkle/gecko/issues/74) `I5` Differentiable reach-cost surrogate (optional) · L · after [#52](https://github.com/JoshuaMarkle/gecko/issues/52)
99. [ ] [#75](https://github.com/JoshuaMarkle/gecko/issues/75) `J4` Generate plate and case mesh (STL) · L · after [#66](https://github.com/JoshuaMarkle/gecko/issues/66), [#73](https://github.com/JoshuaMarkle/gecko/issues/73)
100. [ ] [#76](https://github.com/JoshuaMarkle/gecko/issues/76) `J5` Web: export and printable preview · S · after [#34](https://github.com/JoshuaMarkle/gecko/issues/34), [#75](https://github.com/JoshuaMarkle/gecko/issues/75)
101. [ ] [#77](https://github.com/JoshuaMarkle/gecko/issues/77) `J6` End-to-end walkthrough: hand → board → STL · M · after [#75](https://github.com/JoshuaMarkle/gecko/issues/75), [#69](https://github.com/JoshuaMarkle/gecko/issues/69)

## Path to a printable board

Objective v2 (C1, C4) → parametric 2D geometry (G1) → table rebuild, collision, geometric costs (G2–G4) → CMA-ES (G5) → bilevel search (G6) → hand model + IK (H2–H4) → 3D params, 3D collision (H6, H8) → 3D bilevel (H10) → free-form + manufacturability (I1, I4) → STL (J4) → walkthrough (J6).
