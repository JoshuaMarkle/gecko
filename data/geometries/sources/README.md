# Split-board sources

These KLE inputs preserve `x`, `y`, `w`, `h` from QMK's layout definitions at
[`180f394b489c238cb231c2bf8b080262155e03c0`](https://github.com/qmk/qmk_firmware/tree/180f394b489c238cb231c2bf8b080262155e03c0)
(GPL-2.0-or-later, [license](https://github.com/qmk/qmk_firmware/blob/180f394b489c238cb231c2bf8b080262155e03c0/LICENSE)).
Each key label is its QMK matrix `row,col`. They were imported with
`gecko geometry import-kle data/geometries/sources/<name>.kle.json --out <name>`;
finger, logical row, home, stretch, effort and fill annotations were then reviewed.

| Geometry | QMK file under `keyboards/` | Layout | Keys |
|---|---|---|---|
| `corne-36` | `crkbd/info.json` | `LAYOUT_split_3x5_3` | 36 |
| `sweep-34` | `ferris/sweep/keyboard.json` | `LAYOUT_split_3x5_2` | 34 |
| `lily58` | `lily58/rev1/keyboard.json` | `LAYOUT` | 58 |
| `kyria` | `splitkb/kyria/rev1/info.json` | `LAYOUT_split_3x6_5` | 50 |
| `ergodox` | `ergodox_ez/info.json` | `LAYOUT_ergodox` | 76 |
| `dactyl-manuform-5x6` | `handwired/dactyl_manuform/5x6/keyboard.json` | `LAYOUT_5x6` | 64 |

**Dactyl-Manuform 5x6 is 3D**: its slots, fingers and fills follow the QMK input, but
its poses are written by `dactyl_manuform.py`, a port of the key placement in
tshort/dactyl-keyboard (default 5x6 parameters: α 15°, β 5°, pinky column level, 15°
tent, per-column offsets, six-key thumb fan). Keycap tops are 18.4 mm apart within a
column. The halves are placed with the inner home keys 6u apart (illustrative, as QMK).
`tests/boards.rs` checks its slots against the KLE input but not its poses.

The other boards are **flat configurator projections**, with QMK's illustrative split spacing;
they do not specify a user's half placement or measured PCB/keycap poses. QMK supplies
no splay for these definitions, so the previously invented Corne/Sweep thumb rotations
are removed. The Dactyl's KLE projection has no bowl curvature or tenting, which is why its
poses come from the generator instead. Corne and Sweep now use
the source staggers (left alpha columns `0.30/0.10/0/0.10/0.20` and
`0.93/0.31/0/0.28/0.42` u respectively).

Every board leaves the central 3x10 alpha core unfilled in row order. Outer pinky,
number, auxiliary and unused thumb slots fill with `{none}`; the thumb clusters add
one each of Esc, Space, Tab, Enter and Backspace (Sweep omits Esc). These fills are a
comparison convention, not the board's factory keymap. Kyria's two inner keys above
each bottom thumb cluster and Lily58's high inner key are assigned to the thumbs.
Alpha efforts reuse `ortho-30`; other efforts are heuristic, not measured.

`tests/boards.rs` checks every pose against these inputs, the reviewed alpha cores,
all reference-layout fits, and score deltas across all symbols (including the 76-key
ErgoDox). Geometry TOML includes the direct pinned source URL for each board.

Validation: `gecko list geometries` lists all eight bundled boards; `gecko compare
--geometry <name> --json` scores all ten references on each of the six splits with
`english` / `default` (2026-10-21). These are within-board comparisons; the schematic
thumb distances make cross-board score differences unsuitable as physical measurements.
