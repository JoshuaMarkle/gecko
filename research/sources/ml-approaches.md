---
group: optimization
kind: paper
read: 2026-10-20
status: secondary
feeds: [K-track, C7]
take: Surrogates, preference learning, simulated typists; keep search classical
---

# Machine learning for keyboard design

Where learning (not just search) helps. Ordered by how directly GECKO can use it.

## 1. Learn the cost tables from people (most useful)

- **Pairwise comfort → Bradley–Terry / Bayesian model** (Engram study,
  `engram-preference-study.md`). Features of a key pair (row separation, column separation,
  finger pair, direction) → latent comfort. Output is a C2 table. Active selection of the next
  pairs to ask (max–min distance in feature space) keeps the study short.
- **Timing regression** on keystroke logs (`dhakal-136m-keystrokes.md`): IKI ~ geometric
  bigram features. Gradient-boosted trees or a small GLM; export as a table, so the hot loop
  stays a lookup.
- **Learned profile weights (E03):** fit metric weights so the weighted sum predicts
  comfort/speed labels, or so it reproduces the community's pairwise layout preferences
  (learning-to-rank over cmini's layouts and likes is a cheap, noisy label source).

## 2. Surrogates for expensive objectives

- Nivasch & Azaria, "Keyboard Layout Optimization and Adaptation", *IJAIT* 32(5), 2023,
  doi:10.1142/S0218213023600023: a **deep network approximates an expensive evaluation
  function** inside a GA; better crossover choice also mattered a lot; 300-participant
  adaptation study.
- GECKO's scorer is cheap, so surrogates matter only for: geometry candidates (each needs an
  inner layout search), hand-model IK costs, and simulated typists. Gaussian-process or
  small-MLP surrogates + Bayesian optimization fit there.
- FUTO Swipe (arXiv:2606.25247, **verify**): a neural swipe decoder used directly as the
  objective for a swipe layout. Same pattern: a learned model as the cost function.

## 3. Simulated users

- **CRTypist** (Shi, Zhu, Jokinen, Acharya, Putkonen, Zhai, Oulasvirta, CHI 2024,
  doi:10.1145/3613904.3642918, open at Aalto ACRIS): RL agent with eyes, fingers and working
  memory, trained under human limits; reproduces touchscreen typing across layouts.
  Physical-keyboard version does not exist yet; a long-term option for E01.
- **Computational Input Design** (Yi-Chi Liao, arXiv:2101.10032): multi-objective Bayesian
  optimization + physical simulator + meta-RL user model, applied to button force curves.
  Template for user-in-the-loop geometry tuning.
- **MyoSuite** (arXiv:2205.13600, https://github.com/MyoHub/myosuite): MuJoCo musculoskeletal
  hand (29 bones, 23 joints, 39 muscles), Gym API. Could give muscle-effort costs for key
  poses; heavy, research-grade (phase 7+).

## 4. Learned search (low priority)

- Neural combinatorial optimization (Bello et al., arXiv:1611.09940) and RL for QAP exist, but
  for n = 30 with fast deltas, SA/tabu beat them. Not worth it unless the problem grows a lot.

## Recommendation

Spend ML effort on **cost tables and weights** (sections 1), keep search classical.
The project's current limit is the profile, not the search.
