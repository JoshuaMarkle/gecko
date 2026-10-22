---
group: geometry
kind: tool
read: 2026-10-20
status: read
feeds: [H3, J3]
take: Closed-form finger IK; hand scans; hand-fit stagger
---

# Cosmos keyboard generator (rianadon)

TypeScript/Svelte web app for curved split keyboards, AGPL-3.
https://github.com/rianadon/Cosmos-Keyboards, app at https://ryanis.cool/cosmos.
Docs in the repo: `docs/docs/hand-scans.md`, `hand-fitting.md`, `expert.md`.

## Key ideas

- **Hand from a phone scan:** MediaPipe Hands gives 21 landmarks per hand; each finger is a
  chain of 4 bone vectors (metacarpal + 3 phalanges).
- **Kinematic assumptions (`src/lib/hand.ts`):** CMC joints fixed; MCP 2 DoF (flex + splay);
  PIP and DIP 1 DoF each, all flexion of a finger in one plane; thumb: 2nd joint 1 DoF,
  3rd joint 2 DoF. Flexion axes found by PCA over observed joint vectors.
- **IK with a unique solution:** collapse the 3 phalanges into one virtual link from the MCP to
  the target (1 DoF: length) + MCP 2 DoF = 3 DoF for a 3D target. Then lay the 3 phalanges
  in the plane as a **cyclic quadrilateral** (max area, natural-looking curl).
- **Hand fitting (primitive, by the author's own words):** rotate proximal/intermediate/distal
  phalanges by 5°/4°/9° from flat, read each fingertip position → column Y and Z stagger
  relative to the index column. Metacarpal lengths ignored.
- **Geometry ↔ CAD split:** key placement math is separate from OpenCascade solid modeling.
  Two placement models: matrix layout (columns with curvature) and sphere layout. Expert mode
  is a declarative TS config (`keys: Key[]` with transforms), importable from ergogen/KiCad.
- Flags intersecting keycaps.

## How to use in GECKO

- Reuse the **IK scheme** for the reach cost G01: it is closed-form and fast enough to precompute
  per (finger, key pose). Add joint-angle limits (Cosmos disabled its pan limits) and a cost on
  deviation from a neutral curl.
- The 5°/4°/9° curl is a cheap default neutral posture.
- Accept MediaPipe landmark JSON as the hand input format (phone photo → personal hand).
- Export GECKO geometry to Cosmos expert-mode config to get a printable case (AGPL: interop by
  file format only, no code copying).
