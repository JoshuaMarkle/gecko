// Preview: the 3D Corne through the opening shots (corner macro → nice!nano → thumbs → full board → top).

import { data } from "../lib/data";
import { EASE, sec } from "../lib/motion";
import { Stage, useLayout } from "../scenes/common";
import { CameraRig, sample, type Shot } from "../three/CameraRig";
import { Corne3D, keyTop } from "../three/Corne3D";
import { Scene3D } from "../three/Scene3D";

const tab = keyTop(0), inner = keyTop(5), space = keyTop(38), lower = keyTop(37);
const v = (p: { x: number; y: number; z: number }, dx = 0, dy = 0, dz = 0): [number, number, number] => [p.x + dx, p.y + dy, p.z + dz];

export const CORNE_PATH: Shot[] = [
  { f: 0, pos: v(tab, -46, 10, 44), target: v(tab, 6, -4, 4), fov: 34 },
  { f: sec(1.8), pos: v(inner, -40, 14, 48), target: v(inner, 10, -4, 2), fov: 34, ease: EASE.linear },
  { f: sec(1.81), pos: [-44, 92, 2], target: [-36, 9, -32], fov: 30 },
  { f: sec(3.4), pos: [-34, 88, 38], target: [-32, 9, 4], fov: 30, ease: EASE.linear },
  { f: sec(3.41), pos: v(lower, -50, 6, 40), target: v(lower, 4, -6, -2), fov: 34 },
  { f: sec(5.0), pos: v(space, -24, 8, 52), target: v(space, 10, -6, -4), fov: 34, ease: EASE.linear },
  { f: sec(6.2), pos: [-260, 320, 420], target: [0, 0, 0], fov: 30, ease: EASE.inOut },
  { f: sec(7.5), pos: [0, 640, 1], target: [0, 0, 0], fov: 30, ease: EASE.inOut },
  { f: sec(9.5), pos: [0, 640, 1], target: [0, 0, 0], fov: 30, ortho: 1, ease: EASE.inOut },
];

export function CornePreview() {
  const { frame } = useLayout();
  const s = sample(CORNE_PATH, frame);
  const macro = frame < sec(5.0);
  return (
    <Stage>
      <Scene3D dof={macro ? { target: [s.target.x, s.target.y, s.target.z], bokeh: 6, range: 30 } : null}>
        <CameraRig path={CORNE_PATH} />
        <Corne3D chars={data.layout.start} />
      </Scene3D>
    </Stage>
  );
}
