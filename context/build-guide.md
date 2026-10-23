# Build guide: hand → optimized board → STL

One reproducible path from your hand measurements to a printable split keyboard (#77). It
optimizes a Corne-style 36-key board (`corne-36.param`) and its layout together for your hand,
then writes the plate, case and bottom as STL. Everything runs from a clean checkout; numbers
below are from the reference run (2026-10-23, 8 threads).

```
cargo build --release                                       # ./target/release/gecko
```

## 1. Measure your hand → a profile

The hand model ([hand-model.md](hand-model.md)) reads a profile's `[hand]` section. Start from
the example, which is the bundled `hand` profile (layout metrics + G01 reach, G03 wrist
posture, G05 press angle) with one measured hand and a case check:

```
cp data/examples/my-hand.toml my-hand.toml                  # then edit [hand.dims]
```

Measure in mm, hand flat on a table unless noted:

| Field | How |
|---|---|
| hand length *L* | wrist crease to the tip of the middle finger |
| `palm_length` | wrist crease to the middle finger's base knuckle (the joint, about 2 cm into the palm from the crease where the finger starts) |
| `palm_width` | across the knuckles of index to pinky |
| `span` | thumb tip to pinky tip, fingers spread wide |
| finger segments `[a, b, c]` | knuckle → middle joint → end joint → tip; bend the finger to measure joint to joint on its back |
| `thumb` `[a, b, c]` | wrist end of the thumb's long bone (base of the thumb at the wrist) → knuckle → joint → tip |

Quick version: measure only *L* and `palm_width`, then scale every value of the average hand by
*L* / 196 and set `palm_width` to yours. Average hand (*L* = 196): thumb `[46, 32, 23]`, index
`[40, 23, 18]`, middle `[45, 27, 19]`, ring `[42, 26, 19]`, pinky `[33, 18, 17]`, palm_length
105, palm_width 84, span 200, reach 40. The example is *L* = 205 mm, palm 88 mm. Optionally
add `[hand.body]` `elbow_width` and `forearm` (mm) for the wrist-posture term.

The web UI's Hand tab (`gecko serve`, then the Hand tab) edits the same measurements and shows
the posed hand on a board; check that it looks like your hand before searching.

The example also sets `[build] case = "mx"` ([geometry.md](geometry.md#manufacturability-g07)):
the search builds each 3D candidate's case and rejects boards whose keycaps would hit it.
Use `"choc"` for Choc switches. Without it, 5 of 17 earlier geometry runs produced boards whose
inner columns or thumbs hit the case.

## 2. Optimize board and layout

```
./target/release/gecko geom optimize -g corne-36.param \
  --free '3d,fingers.row_curve=0:20,split.tent=0:30' \
  --pin '{space}{enter}{bspc}{tab}{esc}' \
  -p my-hand.toml -c english --evals 4000 --seed 1 --threads 8
```

- `--free 3d`: column stagger, splay and spread, row and column curvature, column lift, split
  opening, tent and tilt, and the thumb cluster's pose ([algorithms/geometry-search.md](algorithms/geometry-search.md)).
  The bounds keep curvature and tent within what Dactyl builds use: the hand model is
  uncalibrated and pushes both to their limits (30° rows, ~38° tent) when free.
- `--pin`: keep space, enter, backspace, tab and escape on the thumbs. Without it the layout
  search moves them onto letter keys.
- Optional `--thumb-keys auto` sizes each thumb cluster to the keys that fit along your thumb's
  natural arc (3 for an average hand, 4 for a large one), or `--thumb-keys N`. Add `{none}` to
  `--pin` to keep letters off the empty keys it adds. Add `G06 = 1.0` under `[weights]` in your
  profile to pull the thumb keys onto the arc ([hand-model.md](hand-model.md)).
- `--evals 4000 --seed 1 --threads 8`: a fixed budget, so the run is reproducible (≈ 40 s on
  20 cores). `--time 5m` searches longer but cannot be re-run exactly.

It writes `runs/<id>/` (the id is printed) with `geometry.toml`, `geometry.param.toml` and
`best.json`. Reference run `20261023-121212-geom-e91bdfbb`: score 10.47 → 6.62 (−37%) against
the flat Corne with the same layout effort; G03 wrist 1.68 → 0.12, G05 press angle 0.26 → 0.08,
G01 reach 0.24 → 0.18, SFB 0.52% → 0.32%. Shape: rows curved 20° (bound), columns 6.8°, tent
27°, opening 10°, thumbs tilted −21°. Layout:

```
zlmpb jfuo,
nrtdg kheai
xqvcw ·y/;.
  ⎋␣⇥ ⏎⌫s
```

Check and inspect it:

```
./target/release/gecko verify run:<id>                      # re-runs the search: "reproduced"
./target/release/gecko score run:<id> -g run:<id> -p my-hand.toml
./target/release/gecko analyze run:<id> -g run:<id> -p my-hand.toml
```

In the web UI the board appears under geometries as `run:<id>`; the Hand tab poses your hand
on it and the Export tab shows the case in 3D.

## 3. Write the STL

```
./target/release/gecko build stl run:<id> -p my-hand.toml   # --switch choc for Choc
```

Writes `runs/<id>/stl/`: `plate.stl` (switch plate alone, for a sandwich or hand-wired build),
`case.stl` (plate + walls), `bottom.stl`, and `build.json` (parameters, geometry hash, case
report, build-limit violations). The reference board: two pieces of 230 × 122 × 79 mm, lifted
53 mm by the tent, no blocked switches, no keycap hits. Any `warning:` line means the case is
not buildable as is; rerun step 2 with the case check on.

Print `case.stl` and `bottom.stl` per half (the slicer unions the overlapping shells). Not yet
generated: screw bosses, controller / USB / TRRS openings, stabilizer cutouts ([geometry.md](geometry.md#plate-and-case-stl-j4)).

## 4. Firmware layout

The layout is in `best.json` (`best.chars`, slot order) and loads anywhere as `run:<id>`.
`gecko layouts export run:<id> --to cmini` writes the 30 alpha keys; the web Export tab writes an
ergogen config (one zone per key, plate outline) and KLE with legends. Keymap firmware (QMK /
ZMK) is not generated.

## Caveats

The 3D shape is what the model prefers, not a validated design: the hand model has no
finger-vs-keycap clearance and G03's optimum is uncalibrated ([hand-model.md](hand-model.md)).
Seeds agree on the score (≈ 1%) but not on the shape: curvature, tent and thumb pose trade
off. Print a test half before a full build.
