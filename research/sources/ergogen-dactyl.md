---
group: geometry
kind: tool
read: 2026-10-20
status: read
feeds: [G1, H6, J2]
take: Parameter set for parametric geometry search
---

# ergogen and Dactyl / Dactyl-Manuform: parametric keyboard geometry

- ergogen (Bán Dénes, MIT): https://github.com/ergogen/ergogen, docs https://docs.ergogen.xyz.
- Dactyl (Matt Adereth) and Dactyl-Manuform (Tom Short), Clojure → OpenSCAD:
  https://github.com/adereth/dactyl-keyboard, https://github.com/tshort/dactyl-keyboard
  (many forks, e.g. the Python port in joshreve/dactyl-keyboard).

## ergogen model

- YAML **points**: `zones` → `columns` → `rows`. Per column: `stagger` (y shift), `spread`
  (x gap), `splay` (rotation) with an `origin`; per zone: `anchor`, `rotate`, `mirror`
  (for the other half). Keys get `width/height`, `padding`, `orient`, `shift`, `tags`.
- Outputs: point list → SVG/DXF outlines, KiCad PCB, OpenJSCAD cases.
- A small set of numbers describes a whole column-staggered split board.

## Dactyl model

- Keys placed on a curved "keywell": per-column curvature α (around the row axis), row
  curvature β (around the column axis), per-column offsets (finger lengths), tenting angle,
  thumb cluster as a separate transform chain.

## How to use in GECKO

- **Parametric geometry for phase 6–7 should mirror these parameters**, so CMA-ES works over
  ~10–20 numbers: per-column stagger and splay, column spread, row curvature, column
  curvature, tent, split angle, thumb cluster pose. Generate GECKO `[[key]]` poses from them.
- Write `ergogen points → geometry TOML` and back, so results can become real PCBs.
- Hard limits from these tools: switch spacing ≥ 18–19 mm for MX/Choc keycaps, collision of
  keycaps in curved wells (check in 3D).
