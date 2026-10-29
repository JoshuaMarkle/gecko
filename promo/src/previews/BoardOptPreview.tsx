// Preview: the board run playing back (flat corne-42 → tented, curved, splayed) with the final layout.

import { data } from "../lib/data";
import { EASE, interp, progress, sec } from "../lib/motion";
import { Stage, useLayout } from "../scenes/common";
import { CameraRig, type Shot } from "../three/CameraRig";
import { BOARD_LIFT, boardPosesAt, Corne3D, FLAT_POSES } from "../three/Corne3D";
import { Scene3D } from "../three/Scene3D";

const PATH: Shot[] = [
  { f: 0, pos: [-380, 420, 520], target: [0, 20, 0], fov: 30 },
  { f: sec(5.9), pos: [0, 260, 330], target: [-70, 30, 20], fov: 30 },
  { f: sec(6), pos: [380, 360, 520], target: [0, 30, 0], fov: 30, ease: EASE.inOut },
];

export function BoardOptPreview() {
  const { frame } = useLayout();
  const lift = interp(frame, [[0, 0], [sec(1), BOARD_LIFT]]);
  const t = progress(frame, sec(1), sec(4), EASE.inOut);
  const poses = frame < sec(1) ? FLAT_POSES : boardPosesAt(t);
  const stack = 1 - progress(frame, sec(0.3), sec(0.7));
  return (
    <Stage>
      <Scene3D>
        <CameraRig path={PATH} />
        <Corne3D chars={data.layout.final} poses={poses} fill="fingers" stack={stack} tiles={1 - stack} lift={lift} />
      </Scene3D>
    </Stage>
  );
}
