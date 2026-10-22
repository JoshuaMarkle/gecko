# Decisions: Geometry and hand model

Boards (2D, parametric, 3D), KLE import, collision, derived costs and the hand model. Newest last. Format: date — decision — why.

- 2026-10-20 — Geometry, layout and corpus are separate; positions are always 3D. — 2D → 3D without rewriting the core.
- 2026-10-20 — Empty slots (`·`) may repeat and are movable. — Lets the optimizer put a letter on a spare thumb key; `--pin '{none}'` keeps them empty.
- 2026-10-20 — Geometry keys have an optional `fill`; `Layout::fit` puts 30-key layouts onto bigger boards. — Every reference layout works on every board without extra layout files.
- 2026-10-21 — KLE import infers rows, thumbs and fingers from positions, not labels; it writes no thumb fills. — KLE labels vary too much to trust; which thumb gets space is the user's call.
- 2026-10-21 — KLE row inference aligns adjacent equal-height columns by ordinal row first. — Sweep's 0.62u pinky stagger otherwise shifts a column by a row.
- 2026-10-21 — Hand proportions: Buchholz 1992 phalanx ratios, Becker & Thakor active ROM as limits, Bain 2015 functional ROM as the low-cost zone; DIP–PIP is a soft coupling. — The metacarpal ratios and the "⅔" rule could not be sourced.
- 2026-10-21 — `flat2d` uses one plane per hand only when the halves differ by > 5°; overlaps are pushed apart along the SAT MTV. — Split halves stay put and rows horizontal; keys never pass each other.
- 2026-10-21 — Lily58, Kyria, ErgoDox, Dactyl and Corne/Sweep come from pinned QMK-derived KLE projections, keeping every physical slot. — Reproducible sources; limits in `data/geometries/sources/README.md`.
- 2026-10-21 — Parametric boards are mirrored zones/rows/columns, computed in f64 and cast once; hashes use resolved content. — ~16 parameters replace per-key poses and keep run formats.
- 2026-10-21 — The store prefers `name.toml` over `name.param.toml`. — Existing selections keep their geometry.
- 2026-10-21 — `Key::rot` means `Rx·Rz·Ry` in board axes, as the viewer already drew it. — Every existing file stays valid; one convention for core, viewer and hand.
- 2026-10-21 — Geometric metrics measure between keycap tops in each hand's best-fit plane on non-flat boards. — A rigid tent changes no metric while curvature does; flat boards stay bit-identical.
- 2026-10-21 — Parametric 3D uses the Dactyl generator's parameters (`row_curve`, `col_curve`, tent, tilt, lift). — CMA-ES can search them; flat zones keep the old code path.
- 2026-10-21 — `dactyl-manuform-5x6` is an explicit 3D TOML from the generator, not a `.param.toml`. — Its thumb fan is six individually posed keys; slot order stays valid.
- 2026-10-21 — The scorer's hand model is a Rust port of the web's skeleton, placement and IK (G01 reach, G05 press angle). — One model for tab and scorer; mono tables fold for free.
- 2026-10-21 — G05 uses the distal phalanx as the press direction. — The MCP-tangent model scored keywells as worse, the opposite of their design.
- 2026-10-21 — The hand is measured in each half's best-fit plane frame (`Geometry::hand_frame`). — Keeps "a rigid tent changes no metric"; tent belongs to G03.
- 2026-10-21 — Profile `[hand]` is optional; only the `hand` profile weights G01/G05. — Existing profiles, hashes and snapshots stay unchanged.
- 2026-10-21 — `hand.toml` = default + G01 1.0 + G05 0.5 with M01 halved. — M01's key efforts already encode reach; halving avoids double-counting.
- 2026-10-21 — The thumb is a 4-DoF chain solved by damped least squares, in Rust and `hand.ts` alike. — A planar chain can't swing sideways, so thumb clusters were costed as reach.
- 2026-10-21 — Thumb G05 = misalignment with the CMC→tip line only. — The CMC's two axes push the tip anywhere across that line; other models called every thumb key 65–90° off.
- 2026-10-21 — G03 works in board space; default `elbow_width` 465 mm is calibrated to ≈ 12° ulnar deviation on flat boards. — Calibrating to a measured outcome beats guessing a sensitive shoulder width.
- 2026-10-21 — G03 treats ≥ 1 u between the hands as split, with each half aligned to the forearm. — QMK split spacing is illustrative; scoring it would penalize a gap nobody types at.
- 2026-10-21 — G03 weighs 1.0 in natural units, not spread units. — Its spread over layouts is ~0.004; spread units would amplify it ~250×.
- 2026-10-21 — `Scorer::set_geometry` refolds touched entries in build order instead of folding deltas. — Bit-identical to a fresh build, so runs stay reproducible.
- 2026-10-21 — `Geometry::validate` rejects overlapping keycaps; `check_keys` keeps only structural checks; 2D flat boards only. — Search candidates that collide must stay scorable; 3D collision waits for H8.
- 2026-10-21 — Missing `effort` is a NaN sentinel resolved on load; derived effort is linear in distance to home. — Every consumer sees plain numbers; linear fits the bundled efforts as well as quadratic.
