// Scene 1–4: the Corne in a white studio. Three slow macro shots joined by dips to white (the camera
// pivots around the top-left corner key; drifts over the nice!nano and pulls focus onto its
// silkscreen; passes along the thumbs), then a cut to a low, flat shot from the front that rises to
// the top view and narrows to orthographic, where the 2D drawing (Optimize scene) takes over.

import { AbsoluteFill } from "remotion";
import * as THREE from "three";
import { data } from "../lib/data";
import { clamp01, EASE, interp, lerp, progress, sec } from "../lib/motion";
import { C } from "../lib/tokens";
import { along, CameraRig, sample, type Shot } from "../three/CameraRig";
import { Corne3D, keyTop, nanoLabel } from "../three/Corne3D";
import { Scene3D } from "../three/Scene3D";
import { useLayout } from "./common";

type V = [number, number, number];
const corner = keyTop(0), lower = keyTop(37), space = keyTop(38), g = keyTop(16), nano = nanoLabel();
const v = (p: THREE.Vector3, dx = 0, dy = 0, dz = 0): V => [p.x + dx, p.y + dy, p.z + dz];

/** Cuts (frames) between the shots; each is hidden by a dip to white. */
export const CUTS = [sec(4.2), sec(7.2), sec(10.6)] as const;
/** The rise reaches the top view at TOP, the board is orthographic at ORTHO. */
export const TOP = sec(12.4), ORTHO = sec(12.8);

/** Camera on a sphere around `c`: azimuth from the front (+z) toward the left (−x), elevation, radius (deg, deg, mm). */
const orbit = (c: V, az: number, el: number, r: number): V => {
  const a = THREE.MathUtils.degToRad(az), e = THREE.MathUtils.degToRad(el);
  return [c[0] - r * Math.sin(a) * Math.cos(e), c[1] + r * Math.sin(e), c[2] + r * Math.cos(a) * Math.cos(e)];
};
/** The rise: exponential out, so it leaves the low angle at once and slows into the top view. */
const riseEase = (t: number) => (1 - Math.exp(-4 * t)) / (1 - Math.exp(-4));

const C0 = v(corner);
const RISE_C: V = [0, 0, 0];

export const OPENING_PATH: Shot[] = [
  // 1. Pivot on the corner key: move in while swinging left around the corner, slowly.
  ...along(0, CUTS[0], (t) => ({ pos: orbit(C0, lerp(-28, 48, t), lerp(30, 20, t), lerp(92, 68, t)), target: C0, fov: 34 })),
  // 2. Over the nice!nano, a slow drift toward the typist.
  { f: CUTS[0] + 0.01, pos: v(nano, -8, 80, 12), target: v(nano, 0, 0, -20), fov: 30 },
  { f: CUTS[1], pos: v(nano, -6, 78, 40), target: v(nano, 0, 0, 8), fov: 30, ease: EASE.linear },
  // 3. Low along the thumb keys.
  { f: CUTS[1] + 0.01, pos: v(lower, -56, 6, 42), target: v(lower, 4, -6, -2), fov: 34 },
  { f: CUTS[2], pos: v(space, -26, 8, 50), target: v(space, 8, -6, -4), fov: 34, ease: EASE.linear },
  // 4. Low and flat from the front, rising over the board to the top view.
  ...along(CUTS[2] + 0.01, TOP, (t) => ({ pos: orbit(RISE_C, lerp(0, 0, t), lerp(3, 89.91, t), lerp(560, 640, t)), target: RISE_C, fov: 30 }), riseEase),
  { f: ORTHO, pos: [0, 640, 1], target: [0, 0, 0], fov: 30, ortho: 1, ease: EASE.inOut },
];

/** Focus point: the shot's target, except over the nano, where focus pulls from the caps onto the silkscreen. */
function focus(frame: number, s: ReturnType<typeof sample>): V {
  if (frame < CUTS[0] || frame >= CUTS[1]) return [s.target.x, s.target.y, s.target.z];
  const pull = progress(frame, CUTS[0] + sec(0.6), sec(1.1), EASE.inOut);
  const from = v(g), to = v(nano);
  return [lerp(from[0], to[0], pull), lerp(from[1], to[1], pull), lerp(from[2], to[2], pull)];
}

export function Opening() {
  const { frame } = useLayout();
  const s = sample(OPENING_PATH, frame);
  const bokeh = frame < CUTS[2] ? 5 : interp(frame, [[CUTS[2], 2.5], [CUTS[2] + sec(1.5), 0]]);
  const dip = Math.max(...CUTS.map((c) => clamp01(1 - Math.abs(frame - c) / 8)));
  const fadeIn = 1 - progress(frame, 0, 18, EASE.linear);
  return (
    <AbsoluteFill style={{ background: C.canvas }}>
      {/* The depth-of-field pass stays mounted at zero blur: swapping effect passes mid-render freezes the canvas. */}
      <Scene3D dof={{ target: focus(frame, s), bokeh, range: 20 }}>
        <CameraRig path={OPENING_PATH} />
        <Corne3D chars={data.layout.start} />
      </Scene3D>
      <AbsoluteFill style={{ background: C.canvas, opacity: Math.max(dip, fadeIn) }} />
    </AbsoluteFill>
  );
}
