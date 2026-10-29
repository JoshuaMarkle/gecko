// Preview: scene 10 "stl": the real bottom / case STLs rise in exploded and close around the final board.

import { data } from "../lib/data";
import { EASE, interp, progress, sec } from "../lib/motion";
import { Stage, useLayout } from "../scenes/common";
import { CameraRig, type Shot } from "../three/CameraRig";
import { boardPosesAt, Corne3D } from "../three/Corne3D";
import { Scene3D } from "../three/Scene3D";
import { STL_KEY_LIFT, STLModel } from "../three/STLModel";

const PATH: Shot[] = [
  { f: 0, pos: [-300, 380, 420], target: [0, 40, 0], fov: 30 },
  { f: sec(3), pos: [-120, 300, 470], target: [0, 40, 0], fov: 30, ease: EASE.inOut },
];
const FINAL = boardPosesAt(1);

export function StlPreview() {
  const { frame } = useLayout();
  const rise = progress(frame, 0, sec(1), EASE.out);
  const explode = interp(frame, [[0, 1], [sec(0.8), 1], [sec(2), 0]]);
  return (
    <Stage>
      <Scene3D>
        <CameraRig path={PATH} />
        <STLModel rise={rise} explode={explode} />
        <Corne3D chars={data.layout.final} poses={FINAL} fill="white" stack={0} spread={0} lift={STL_KEY_LIFT} />
      </Scene3D>
    </Stage>
  );
}
