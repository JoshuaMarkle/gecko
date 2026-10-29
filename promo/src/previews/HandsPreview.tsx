// Preview: scene 8. The product's hands type the novel's opening on the final layout, the camera
// orbits the right hand, stat callouts follow each press, and the skin dissolves to the skeleton.

import { AbsoluteFill } from "remotion";
import { data } from "../lib/data";
import { EASE, interp, progress, sec } from "../lib/motion";
import { C, H, MARGIN, W } from "../lib/tokens";
import { Stage, useLayout } from "../scenes/common";
import { CameraRig, project, type Shot } from "../three/CameraRig";
import { Corne3D } from "../three/Corne3D";
import { Hands3D } from "../three/Hands3D";
import { pressAngle, travelMm, typing } from "../three/hands";
import { Scene3D } from "../three/Scene3D";
import { StatCallout } from "../ui/space";
import { Eyebrow, KineticHeadline } from "../ui/type";

export const HANDS_FRAMES = sec(6.5);
const TYPE_AT = sec(0.5);
const TEXT = data.language.opening.split(",")[0];
const TYPING = typing(TEXT, data.layout.final, { wpm: 80 });

const r = TYPING.rigs[1].home.index;
const PATH: Shot[] = [
  { f: 0, pos: [r[0] + 120, 400, r[2] + 600], target: [r[0] - 60, 40, r[2] + 50], fov: 30 },
  { f: HANDS_FRAMES, pos: [r[0] + 520, 360, r[2] + 260], target: [r[0] - 40, 40, r[2] + 50], fov: 30, ease: EASE.linear },
];

export function HandsPreview() {
  const { frame } = useLayout();
  const t = (frame - TYPE_AT) / 30;
  const press = TYPING.keys(t);
  const pr = TYPING.pressAt(t);
  const frames = TYPING.frames(t);
  const skin = 1 - progress(frame, sec(5.5), sec(0.5));
  const bones = interp(frame, [[sec(5.4), 0], [sec(5.7), 1], [sec(6.1), 1], [sec(6.5), 0]]);
  // Callouts: the pressing fingertip (travel, press angle from the IK pose), the scorer's reach and
  // wrist costs for that key, and the hand's resting pronation (WASM hand_costs). The resting
  // deviation and extension are 0° for the average hand on this board, so they are not shown.
  const hand = frames.find((f) => f.side === (pr?.side ?? "R"))!;
  const fi = ["thumb", "index", "middle", "ring", "pinky"].indexOf(pr?.finger ?? "index");
  const tip = hand.pose.fingers[fi][hand.pose.fingers[fi].length - 1];
  const posture = data.hand.posture[pr?.side === "L" ? 0 : 1];
  const key = pr?.key ?? -1;
  const atTip = project(PATH, frame, tip), atWrist = project(PATH, frame, hand.pose.wrist);
  const x = W - MARGIN.x - 300;
  const stats: { at: [number, number]; label: string; value: string }[] = [
    { at: atTip, label: `reach to ${pr ? pr.ch.toUpperCase() : "key"} (mm)`, value: pr ? travelMm(pr).toFixed(0) : "0" },
    { at: atTip, label: "press angle (°)", value: pressAngle(hand.pose, pr?.finger ?? "index", pr?.n).toFixed(0) },
    { at: atTip, label: "reach cost", value: key >= 0 ? data.hand.reach[key].toFixed(2) : "–" },
    { at: atWrist, label: "wrist cost", value: key >= 0 ? data.hand.wrist[key].toFixed(2) : "–" },
    { at: atWrist, label: "pronation (°)", value: posture.pronation.toFixed(0) },
  ];
  const out = 1 - progress(frame, sec(5.2), sec(0.4));
  return (
    <Stage>
      <Scene3D>
        <CameraRig path={PATH} />
        <Corne3D chars={data.layout.final} press={press} accent={pr && t < pr.up ? pr.key ?? null : null} />
        <Hands3D typing={TYPING} t={t} skin={skin} skeleton={bones} />
      </Scene3D>
      <AbsoluteFill>
        {stats.map((s, i) => (
          <StatCallout key={s.label} at={s.at} to={[x, MARGIN.y + 140 + i * 150]} label={s.label} value={s.value} lead={0.15 + 0.17 * i}
            reveal={progress(frame, sec(1.5) + i * 4, sec(0.6), EASE.out) * out} />
        ))}
        <div style={{ position: "absolute", left: MARGIN.x, top: MARGIN.y }}><Eyebrow text="Hand model." start={sec(0.2)} /></div>
        <div style={{ position: "absolute", left: MARGIN.x, top: H - MARGIN.y - 120, color: C.ink }}>
          <KineticHeadline text="Built around your hand." start={sec(4.5)} />
        </div>
      </AbsoluteFill>
    </Stage>
  );
}
