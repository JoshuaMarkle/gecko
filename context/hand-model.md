# Hand model

`gecko_core::hand` turns a hand (preset size or measurements), arms and a board into per-key
costs the scorer folds like any mono metric: **G01 reach**, **G05 press angle** and **G03 wrist
posture**. The hand also scales B14's relaxed finger spread. It is a port of the web's `hand.ts` (same skeleton,
constants, placement and IK), so the Hand tab shows the hand the scorer measures.
Background: `research/hand-model-and-geometry.md`, `research/sources/hand-models.md`.

## Inputs

Profile `[hand]`: `size = "small" | "average" | "large"` (average × 0.92 / 1 / 1.08, ≈ 8th /
50th / 92nd percentile of hand length), or `[hand.dims]` in mm: `thumb`, `index`, `middle`,
`ring`, `pinky` (three segments each, base → tip), `palm_width`, `palm_length`, `span`, `reach`
(`reach` is shown in the web, not used in costs). `[hand.body]` (optional, mm): `elbow_width`
(elbow to elbow while typing, 100–1500) and `forearm` (elbow to wrist crease, 100–600); default
465 / 255 scaled with hand length. Without `[hand]`, G01/G03/G05 use the average hand and arms
and B14 is unchanged. The web's **Use hand in optimization** sends the Hand tab's hand (and arms,
when edited under Advanced → Arms) as this block (`hand.ts` `withHand`) and adds G01 1.0 / G03 1.0
/ G05 0.5 (with scales) when the profile has no weight for them. Bundled profile:
`data/profiles/hand.toml` (default + G01 1.0, G05 0.5, G03 1.0 in natural units, M01 halved
because its hand-entered key efforts also encode reach).

## Model

1. **Skeleton** (right hand; the left is its mirror): knuckles on an arch across 0.76 × palm
   width, the thumb's base and rest point from palm width/length (`hand.ts` `handLayout`).
2. **Placement:** per hand, a pattern search over wrist x, z, height, yaw and roll puts each
   finger's natural curl (PIP 54°, DIP 0.7 × PIP, proximal phalanx 11° down) on its home key
   (non-stretch home key, else any home key, else its first key), pulled weakly toward 10°
   inward yaw and no roll. A hand without all four finger keys is not placed (its keys cost 0).
3. **Fingertip targets:** keycap top centres (`pose::contact`). On 3D boards they are taken in
   each hand's frame (`Geometry::hand_frame`, the best-fit plane of its finger keys), like the
   geometric metrics, so a rigid tent changes nothing: the forearm's roll belongs to G03.
4. **IK** (`hand::solve`): planar 3-segment chain in the vertical plane through knuckle and
   target; joint limits MCP −20°…86°, PIP −3°…109°, DIP 0…80°; among MCP angles, the pose that
   misses least, then sits nearest DIP ≈ 0.7·PIP with the MCP near rest. Out of reach: the finger
   points straight at the target and the shortfall is the miss.
5. **Thumb** (`hand::Thumb`, 4 DoF): CMC flexion (tip sweeps across toward the fingers) and
   palmar abduction (tip moves away from the palm) about fixed axes at the CMC, then MCP and IP
   flexion toward the pad (the pad faces the fingers and a little down: the thumb is pronated).
   Rest: the tip on the rest point (beside the index, 0.35 × palm length below the knuckles), MCP
   and IP near 17°. IK is damped least squares on the tip error plus each joint's deviation from
   rest, within limits (CMC flexion ±34°, abduction −29°…+40° about rest, MCP −11°…60°, IP
   −17°…80°; `research/sources/hand-models.md`).
6. **Wrist** (G03): the placed hand's forward and up axes and wrist point are turned back from
   the hand frame into board space (so a tent shows), the left hand mirrored about the board's
   middle. The elbow sits `elbow_width / 2` outside the middle, level with the wrist, `forearm`
   behind it; the forearm runs elbow → wrist. On a **split** board (≥ 1 u between the hands'
   finger keys) the halves' drawn spacing is illustrative, so each half is assumed turned to line
   up with the forearm: no resting deviation. Angles: deviation in the palm's plane between the
   forearm and the hand (+ ulnar), extension = the hand's pitch above the forearm's, pronation =
   90° + the back of the hand's roll about the forearm (90° palm down, 0° thumb up).

## Costs (per key; layer copies use their physical key, plus the layer holder like M01)

- **G01 reach** = mean over MCP/PIP/DIP of ((angle − natural) / 29°)² + (splay / comfortable
  abduction)² + 2·(miss / 10 mm) + (miss / 10 mm)². Splay is the finger plane's turn from its
  home key (thumb: from its rest point); comfortable abduction pinky 25°, ring/middle/index 21°,
  thumb 30°. Misses under 0.5 mm are IK rounding. ≈ 0 on home keys, ≈ 1 for a comfortable
  off-home key, 3–5 for keys the planted hand cannot reach.
  Thumbs: mean over the 4 joints of ((angle − rest) / scale)² (CMC 26°, MCP 34°, IP 46°) plus
  the same miss terms; no splay term (the CMC's two axes are the sideways motion).
- **G05 press angle** = (angle / 45°)² between the distal phalanx (the fingertip presses along
  it) and the inward keycap normal, the normal projected into the finger's plane (sideways tilt is
  wrist roll). On flat boards home ≈ 0–0.4, top/bottom rows 0.5–2; keywells tilt keys toward the
  fingertip and score lower. Thumbs: the CMC pushes the tip anywhere across its CMC → tip line, so
  only the push along that line is misaligned: angle = asin |n̂ · CMC→tip|. Flat thumb keys ≈ 0.3
  (25°); the Dactyl's cluster, tilted toward the thumb, 0.04–0.12.
- **G03 wrist posture** = (deviation / 15°)² ulnar or (/ 10°)² radial + (extension / 20°)² or
  (flexion / 25°)² + ((pronation − 30°) / 45°)², from the resting hand (comfortable ranges from
  `research/sources/hand-models.md` and the posture studies in
  `research/sources/split-keyboard-ergonomics.md`). Per key, a key the planted hand falls short of
  adds the deviation of turning the hand toward it: atan(sideways part of the shortfall /
  wrist-to-key distance). Mostly a board metric: on one board it varies only through those
  sideways reaches (spread over reference layouts on ansi-30 ≈ 0.004), so the hand profile
  weights it in natural units (1.0, no scale): ≈ 1 point between a flat board and a tented split.
- **B14 spread:** the relaxed spread per finger is 1.35 u × `span` / 200 mm with a `[hand]`
  (exactly 1.35 u for the average hand).

Costs are computed once per scorer (n keys × ~100 IK evaluations, well under a millisecond), so
search speed is unchanged. WASM `hand_costs` returns the per-key tables for the web.

## Validation (`tests/hand.rs`)

Natural curl recovered by the IK; far targets flagged with the exact shortfall; home keys the
cheapest on every finger (within 0.2: the short pinky rests slightly below its home key);
mirrored boards give mirrored costs; small > average > large mean reach on ANSI; a key moved 2 u
away is unreachable and costly; a rigid tent changes no metric but G03, which it lowers
(`tests/three_d.rs`). Thumb: a cluster key near rest costs < 0.3, a key moved 3 u away is out of
reach, Dactyl thumb keys press at < half Corne's angle. Wrist: ANSI 9–15° ulnar with palms flat,
splits ≈ 0° deviation, the Dactyl's tent lowers pronation by > 10°, G03 Dactyl < Corne < ANSI,
wider elbows deviate more, a sideways out-of-reach key adds G03, bad `[hand.body]` rejected. Web
(`hand.test.ts`): the TS thumb rests on its rest point, reaches near targets, keeps its segment
lengths; `toLocal` inverts `toWorld`. QWERTY, english, average hand and arms:

| board | G01 | G05 | G03 | resting wrist (R): deviation, pronation |
|---|---|---|---|---|
| ortho-30 | 1.196 | 0.522 | 2.34 | 12° ulnar, 88° |
| ansi-30 | 1.153 | 0.534 | 2.34 | 13° ulnar, 88° |
| ergodox | 1.286 | 0.541 | 1.67 | 0°, 88° |
| corne-36 | 0.914 | 0.468 | 1.67 | 0°, 88° |
| lily58 | 0.885 | 0.467 | 1.67 | 0°, 88° |
| kyria | 0.854 | 0.476 | 1.67 | 0°, 88° |
| sweep-34 | 0.861 | 0.489 | 1.67 | 0°, 88° |
| dactyl-manuform-5x6 | 0.623 | 0.175 | 1.04 | −2°, 73° |

The 4-DoF thumb reaches Corne/Kyria clusters more cheaply than the old planar chain (Corne G01
0.99 → 0.91) and finds Ergodox's outer thumb keys 4–6 mm out of reach (1.15 → 1.29), the cluster's
known complaint; G05 now counts thumbs. The test pins Dactyl < Corne < ortho, ANSI on G01 and
Dactyl < 0.6 × Corne on G05. On ANSI the
left bottom row costs 2–2.4 for ring/middle vs 0.5–0.6 on the right (the stagger's known
asymmetry); index top and centre-column keys, Q and P are out of reach for the planted average
hand by 3–10 mm, matching the Hand tab. Reference layouts on ansi-30 (G01): Halmak 0.45,
Canary/Graphite 0.50, Workman 0.51, Semimak 0.52, Sturdy 0.53, Colemak-DH 0.56, Colemak 0.62,
Dvorak 0.75, QWERTY 1.15.

## Results (english, SA, seed 1, 15 s, 2026-10-21)

ANSI rows predate the thumb model and G03: ansi-30 has no thumb keys and G03 is constant there
up to sideways misses, so those optima stand. Corne was re-run with them:

| board · profile | G01 | G05 | SFB % | SFS % | B14 | layout (rows) |
|---|---|---|---|---|---|---|
| ansi-30 · default | 0.708 | 0.409 | 0.74 | 5.92 | 1.41 | `jqmpbzguo. rhtcfxseai lndwvyk/;,` |
| ansi-30 · hand | 0.465 | 0.278 | 0.62 | 6.12 | 2.17 | `qhtvxzpuo. rndsfgceai lbkmjyw/;,` |
| ansi-30 · hand, small | 0.610 | 0.312 | 0.63 | 6.24 | 2.56 | `jhtwvzquo. rndcfxseai lmkpbyg/;,` |
| ansi-30 · hand, large | 0.471 | 0.242 | 0.51 | 6.27 | 1.79 | `blfkjzpea. hrstmwcuoi nxvdqyg/;,` |
| corne-36 · default | 0.320 | 0.220 | 0.22 | 3.34 | 0.65 | runs `…-a2cfeb3a` |
| corne-36 · hand (old planar thumb) | 0.308 | 0.181 | 0.34 | 3.28 | 0.65 | runs `…-83c23983` |
| corne-36 · hand (4-DoF thumb, G03) | 0.247 | 0.231 | 0.22 | 3.32 | 0.89 | run `20261021-225227-sa-72390ea7` |

Runs `20261021-215352-sa-{dfcd41ae, 0458415f, 6182f076, 28b4c861, a2cfeb3a, 83c23983, b4af222d,
50cb1ce4}` (8 searches in parallel, so the 15 s were contended). Reading: on ANSI the hand profile
cuts reach by a third (0.71 → 0.47) and press angle by a third; its optimum scores only 1.9%
worse under the default profile (18.19 vs 17.85), while the default optimum is 9% worse under
the hand profile (23.02 vs 21.10). It keeps the index off the out-of-reach top-row keys and trades some stretch
(B14 1.4 → 2.2). Hand size changes the optimum: the small hand gives up more reach (0.61), the
large hand moves frequent letters to the middle/ring top row it reaches easily. On Corne the
default optimum is already fairly reach-efficient; with the 4-DoF thumb it scores G01 0.280 /
G05 0.257 (G05 now including thumbs) and the hand optimum 0.247 / 0.231, which costs 0.7% under
the default profile (8.29 vs 8.24) while the default optimum costs 2.6% under the hand profile
(11.91 vs 11.61). G03 is 1.666 for both: it ranks boards, not layouts.

## Limits and next steps

- Planar finger IK: abduction is free within the cost, not a joint; no MCP abduction/flexion
  coupling (abduction shrinks at full flexion). The thumb's CMC axes are fixed (no CMC axial
  rotation/opposition coupling); G06 (thumb-arc fit) is separate, unimplemented.
- G03: elbows level with the keys, one elbow width for every board; split halves assumed ideally
  turned (a real user may not); no wrist-height or keyboard-slope input yet (extension comes from
  the board's shape only). Elbow width is calibrated to the ≈ 12° conventional-keyboard deviation
  and is sensitive (±10 mm ≈ ±1.3°).
- The hand is planted on the home keys; real typing moves the wrist, which the miss term
  stands in for. The typing simulation (wrist follow, parallel motion) is view-only.
- No per-pair hand cost yet (finger-to-finger transitions with the hand pose, #52's travel
  part); B03/B14/B11 cover pairs heuristically.
- Constants (curl, coupling, limits, cost scales) are heuristics from the sources above;
  population data and a measured hand would replace the presets (#103 optimizes over sizes).
