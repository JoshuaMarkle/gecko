# Hand model and physical geometry

How we get from "assign symbols to fixed keys" to "generate the keyboard itself".

## Roadmap of geometry freedom

| Phase | Variables | Kind | Constraint |
|---|---|---|---|
| G0 | Symbol → slot on fixed board | Discrete (QAP) | Pins |
| G1 | Key x, y, z-rotation (2D) | Continuous | No overlapping keycaps (2D OBB, SAT test) |
| G2 | + z height, tilt, tent | Continuous | 3D collision of keycap + switch volumes |
| G3 | Full 6-DoF per key, curved surfaces, thumb clusters | Continuous, high-dim | 3D collision, manufacturability |

From G1 on, the problem is **bilevel**: an outer continuous search over geometry and
an inner discrete search over symbol assignment. Options:

- **Alternating:** fix layout, optimize geometry; fix geometry, re-optimize layout. Repeat.
- **Nested:** outer CMA-ES / differential evolution / Bayesian optimization, each
  candidate geometry scored by a short inner SA run (warm-started from the parent's layout).
- **Parametric:** optimize a few high-level params (column splay, stagger, curvature,
  thumb arc) instead of every key. Far fewer dimensions; ergogen/Dactyl style. Start here.

## Hand model

Parameterize per finger (index, middle, ring, pinky) and thumb:

- **Base:** metacarpophalangeal (MCP) joint position relative to the wrist.
- **Segments:** proximal, middle, distal phalanx lengths. Thumb: metacarpal, proximal, distal.
- **Joints and range of motion:** MCP flex/extend + abduct/adduct (2 DoF); PIP flex
  (1 DoF); DIP flex (1 DoF, softly coupled to PIP; measured slopes 0.44–0.87 per finger,
  the common "⅔" rule is unsourced); thumb CMC 2 DoF + MCP + IP. Verified ROM table in
  `sources/hand-models.md`.
- **Neutral posture:** relaxed joint angles; cost grows with deviation.
- **Wrist:** position, extension, ulnar/radial deviation, pronation. A fixed wrist is
  the default; allowing wrist movement adds a large cost.
- **Scaling:** derive all segment lengths from two measurements (hand length, hand
  breadth) using Buchholz et al. 1992 ratios (checked, with status per value, in
  `sources/hand-models.md`), or from a hand scan (Cosmos uses scans; MediaPipe-style landmarks are an option).

### Reach cost for a key

1. Inverse kinematics: find joint angles that put the fingertip on the key top with the
   press direction close to the key normal.
2. If no solution within ROM → unreachable (hard constraint or huge cost).
3. Cost = weighted joint deviation from neutral + press-direction misalignment
   + wrist movement if required.
4. Cache per (finger, key pose). Recompute only for keys that moved.

This produces `effort(slot)` and per-pair travel/time costs, so the **same scorer**
works on any geometry. The hand model only changes the precomputed tables.

### Validation

Sanity checks before trusting it: home row should be cheapest; standard row-stagger
should cost more than column-stagger for the same layout; known ergonomic boards
(Corne, Dactyl) should beat flat ANSI on reach cost.

## Collision

- 2D: oriented bounding boxes of keycaps (plus margin); separating axis theorem.
- 3D: convex hulls of keycap + switch body; GJK. Broad phase with a uniform grid.
- In continuous optimizers, treat as a penalty with a hard-fail threshold, or repair by
  pushing keys apart.

## Visualization link

The web 3D viewer renders exactly the geometry records (pos, rot, size), so every phase
renders without special cases. Optionally show the posed hand skeleton per key press.

## Sources

- https://github.com/rianadon/Cosmos-Keyboards (TS; hand scans; geometry separate from CAD)
- https://github.com/ergogen/ergogen (declarative zones/columns/rows, splay, stagger)
- Dactyl-Manuform (Clojure → OpenSCAD) parametric curved keyboards
- FFitts law (Bi, Li, Zhai, CHI 2013) for finger pointing time models
