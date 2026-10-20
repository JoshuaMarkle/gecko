# Feit, Weir, Oulasvirta — "How We Type: Movement Strategies and Performance in Everyday Typing" (CHI 2016)

doi:10.1145/2858036.2858233. Project: https://userinterfaces.aalto.fi/how-we-type/
Dataset (motion capture, keylogs, eye tracking, 30 participants, non-commercial):
https://zenodo.org/record/4034268

## Key findings

- Self-taught typists using fewer fingers can match touch typists' speed.
- Finger-to-key mappings cluster into 4 left-hand and 6 right-hand strategies; the left hand
  stays put, the right hand moves widely.
- Predictors of speed: **consistent key→finger mapping**, **preparing upcoming keystrokes**
  (fingers move toward the next key early), **little global hand motion**.

## How to use in GECKO

- Supports modeling **fingering as a variable** (several fingerings per geometry, already in
  `parameters.md`), and penalizing hand displacement rather than only finger travel.
- "Preparation" is why skipgrams matter: a finger has a key or two of time to pre-move. Matches
  the SFS weighting (skip-1 ≈ half an SFB).
- The motion-capture data could validate a hand model's key reach postures (phase 7).
