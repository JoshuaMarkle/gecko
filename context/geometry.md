# Parametric 2D geometry

`gecko_core::parametric::ParametricGeometry` generates an ordinary `Geometry` from
`data/geometries/*.param.toml`. The scorer, layout fitting, comfort tables, run hashes,
CLI and server use the resulting keys exactly as they use explicit `[[key]]` boards.
Existing geometry files and JSON shapes are unchanged.

Load a parameter file directly, e.g.

```
gecko compare -g data/geometries/corne-36.param.toml --json
gecko compare -g corne-36.param --json
```

`Store::geometry("name")` prefers `name.toml`, then `name.param.toml`. Listing geometries
returns the base name once, so a parameter-only board appears in the web picker and adding
a parameterized equivalent never changes an existing board selection. A direct path or
`name.param` selects parameters explicitly. Store resolves `[comfort]` CSVs after generation.
For custom saved runs, use a new model name matching its file's base name; `verify` reloads
geometry by the recorded name, as it does for explicit board files.

## Shape and metadata

A board has named `[[zone]]`s, ordered logical `rows`, and `[[zone.column]]`s. Separate
finger and thumb zones let the thumb cluster move without moving the finger matrix.
Units are u (19.05 mm); angles are degrees, positive clockwise in the x-right/y-down plane.
All generated keys have z = 0 and x/y rotation = 0.

| Field | Meaning / default |
|---|---|
| Zone `anchor` | First column's first-row **center**, before stagger; `[0,0]` |
| Zone `rotate` | Rotate the whole zone about its anchor; 0 |
| Zone `rows` | Strictly increasing logical row labels, e.g. `[0,1,2]` or `[3]` for thumbs |
| Zone `row_pitch` | Distance between successive rows, independent of labels; 1 |
| Zone `home_row` | Logical home-row label; 1 (no home keys if absent from `rows`) |
| Zone `home_stretch` | Tag home-row stretch keys as home too; false |
| Zone `col_start` | First logical column label; 0 |
| Zone `mirror` | Generate a right-hand copy using `[split]`; false |
| Column `spread` | Baseline distance from previous column; 1; ignored for first column |
| Column `stagger` | Shift the column toward the user; 0 |
| Column `splay` | Rotate about this column's home-key center; 0 |
| Zone `arc_radius` | Bend the baseline into a circular arc; omitted = straight |
| Column `finger`, `effort` | Finger and one nonnegative base effort per logical row |
| Column `size`, `stretch` | Key footprint `[1,1]`, stretch flag false |
| Column `fill`, `mirror_fill` | Optional fill on the original / mirrored key |

The columns' total baseline distance is arc length: for distance `d` and signed radius
`r`, the baseline is `(r sin(d/r), r(1−cos(d/r)))`, with tangent angle `d/r`. Stagger and
row offsets follow that tangent; column splay adds rotation around the home key. Negative
radius curves away from the user. This is a real circular thumb arc, with independent
cluster position and rotation. A zone without a home row uses its first row as the splay
pivot while leaving every key's `home` flag false.

`[split]` has required `axis` (mirror plane x), plus `spread = 0`, `angle = 0`,
`pivot = [0,0]`. After zone transforms, the left hand rotates `−angle/2` about `pivot`;
the right copy reflects its centers as `x_right = 2*axis + spread − x_left`. Thus the
included hand angle is `angle`, and spread adds separation to the right half. Column
fingers in mirrored zones must be left-hand fingers; `LP↔RP`, `LT↔RT`, etc. are automatic.
An unmirrored zone can use either hand and receives only its own zone transforms.

For example, a two-thumb arc (add it beside a finger zone) is:

```toml
[[zone]]
name = "thumbs"
anchor = [3.5, 4.2]
rows = [3]
mirror = true
arc_radius = -5.0
rotate = 10.0
[[zone.column]]
finger = "LT"
effort = [1.0]
fill = "space"
mirror_fill = "enter"
[[zone.column]]
finger = "LT"
spread = 1.0
effort = [1.3]
fill = "tab"
mirror_fill = "bspc"
```

Column labels increase left-to-right in the unrotated topology: original columns first,
then mirrored columns in reverse. Keys sort by `(row,col)` across zones, preserving layout
row order even after splay or split rotation. Zones sharing a row need distinct `col_start`
ranges. Poses rotate key centers; the stored `pos` subtracts half of each key's size to match
GECKO's top-left convention. Computation uses f64, casts once to f32 and canonicalizes zero.

Unknown fields, non-finite values, invalid sizes/efforts, duplicate logical slots, invalid
fills and excessive key counts fail. Generation does not repair physical key collisions;
collision constraints belong to the geometry search (#39).

## Bundled models and search

`ortho-30.param.toml` and `corne-36.param.toml` reproduce their explicit files exactly,
including metadata, content hashes and all reference scores under all bundled profiles.
Corne targets the corrected QMK flat projection, with a taller inner thumb and one thumb
column stagger. Its baseline omits `arc_radius`: QMK display coordinates do not establish
a measured circular thumb arc. A physical thumb arc can be selected explicitly.

The useful symmetric search vector is about 16 numbers: five column staggers, five splays,
split spread/angle, thumb x/y/rotation/radius. Row/column pitches can also be freed. Finger
assignments, efforts, logical rows, fills and footprints are metadata, not independent key
poses. `ParametricGeometry` is serializable for future CMA-ES candidates; no optimizer or
3D curvature is implemented here. The resolved `Geometry::hash()` identifies generated
input content, so equivalent explicit and parametric boards remain interchangeable in runs.
