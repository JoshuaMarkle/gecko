# GECKO promo

The promo video, built in Remotion (React) with React Three Fiber for the 3D board: about 84 s, in
the product's light theme, almost no text. Storyboard: [STORYBOARD.md](STORYBOARD.md). Every number on
screen comes from real GECKO output via `src/generated/promo-data.json`.

```
npm install
npm run fonts                  # Inter + JetBrains Mono (OFL) → public/fonts
npm run data                   # CLI + API + WASM → src/generated/promo-data.json, STLs → public/stl, hands → public/hands
npm run studio                 # Remotion Studio: preview compositions
npm run components             # component sheet: http://localhost:7795/components.html
npm run stills -- promo 0 400  # render frames to out/ (system Chromium, ANGLE)
npx remotion still src/index.ts banner out/banner.png --scale=2 --gl=angle   # README banner (transparent PNG)
npm run render                 # out/promo.mp4 (full size); npm run preview for half size
npm run typecheck
```

`npm run data` needs `make` first (release CLI, `web/src/wasm/pkg`). It starts its own
`gecko serve` on port 7793.

## Layout

| Path | What |
|---|---|
| `media/` | The finished video (`promo.mp4`) and the README banner (`banner.png`), committed; `out/` holds review renders and is ignored |
| `src/generated/` | `promo-data.json`, committed: it comes from runs that are not in git |
| `src/lib/` | `tokens` (the product's design tokens at video scale), `motion` (easings, `progress`, `interp`, spring), `data`, `font`, `rng` |
| `src/three/` | `Corne3D` (both halves: plate stack, PCB silkscreen, nice!nano under glass, TRRS, reset, MX switches, dished caps; any GECKO key poses), `Switch`, `NiceNano`, `STLModel` (the run's real STLs), `Hands3D` + `HandMesh` + `hands` (the product's skinned hands and skeleton, typing via `web/src/hand.ts` IK), `Scene3D` (light rig, fog, floor shadow, post), `CameraRig` (keyframed shots, ortho blend by dolly zoom, `project` for 2D anchors) |
| `src/ui/` | `Board2D` (legend swaps, pose labels, finger wipe, heat, arcs), `numbers` (`MetricCounter`, `StatRing`), `trace` (`Trace`: neutral ink leaders, 45° turns), `Document` (a word processor's title bar, tabs, ribbon, ruler), `LayoutPage` (the Optimize › Layout tab), `charts` (`Card`, `LineChart`, `Bars`, `StackedBar`, `RangeBars`, `Heatmap`, `Scatter`, `EliteGrid`, `DataLine`), `panels` (`DataTable`, `MonoList`, `SegmentedTabs`, `StatusBadge`, `FileCard`, `TerminalWindow`, `Page`, `Navbar`, `AppFrame`, `FlowChart`, `EndCard`), `type` (`Eyebrow`, `KineticHeadline`, `GlitchText`), `space` (`Space3D`, `FloatingPanel`, `Tunnel`, `StatCallout`), `icons` |
| `src/previews/` | One composition per component group (Studio folder "components"): `corne`, `board-opt`, `layout-ring`, `stl`, `gallery`, `hands` |
| `src/sheet/` | The component sheet (`components.html`, Vite): the 2D components as still frames, and a 3D viewer to orbit the models by hand (`#3d`) |
| `src/storyboard.ts` | Scene start frames and lengths (overlaps are the transitions) |
| `src/Promo.tsx` | The `promo` composition: the scenes on one timeline |
| `src/scenes/` | `Opening`, `Optimize`, `Tunnel`, `Language`, `Physical`, `App`, `End`; `common.tsx` (`Stage`, `At`, `useLayout`) |

3D units are millimetres; key poses map to the scene exactly as `web/src/three/Keyboard3D.tsx`
does (gecko-core `pose.rs` convention). 2D panels that move in depth use CSS 3D (`ui/space.tsx`)
so their text stays sharp.

## Look

The product's light theme (`lib/tokens.ts` holds its token values at video scale): white stage,
near-black ink, one blue. The 3D stage (`three/Scene3D.tsx`) is a white studio: white Lambert floor
and fog, a soft key-light shadow, no bloom (the switches have no LEDs).
