// Scenes 9–11 in one 3D take. The product's hands (porcelain white) type the novel's opening on the
// final layout, shot low and level from in front of the left half; the pressed key turns blue and a
// stat block reads out that press (no leaders: they chased the fingertip). The camera orbits past
// the side as the skin dissolves to the skeleton, each bone labelled with its length, and keeps
// going the same way round the board, rising, to the board view; "Optimize for" switches Layout →
// Board and the board run plays back (tent, row curve, splay, thumb tilt) beside its score. Then the
// run's real STL case rises in slowly and closes around the board, its file card fades up and its
// download button is clicked.

import { AbsoluteFill } from "remotion";
import * as THREE from "three";
import { data } from "../lib/data";
import { clamp01, EASE, interp, lerp, progress, sec } from "../lib/motion";
import { C, MARGIN, MONO, WEIGHT } from "../lib/tokens";
import { along, CameraRig, project, type Shot } from "../three/CameraRig";
import { BOARD_LIFT, boardPosesAt, Corne3D, FLAT_POSES, isLeft, keyTop } from "../three/Corne3D";
import { Hands3D } from "../three/Hands3D";
import { pressAngle, travelMm, typing } from "../three/hands";
import { Scene3D } from "../three/Scene3D";
import { STL_KEY_LIFT, STLModel } from "../three/STLModel";
import { RangeBars } from "../ui/charts";
import { MetricCounter } from "../ui/numbers";
import { FileCard, SegmentedTabs, StatusBadge } from "../ui/panels";
import { useLayout } from "./common";

/** Phase starts (frames, scene-local). */
const P = { type: 14, stats: 40, skeleton: sec(5.6), side: sec(5.2), bones: sec(6.6), hold: sec(8.4), swing: sec(10.9), board: sec(11.9), tab: sec(12.1),
  morph: sec(12.7), morphEnd: sec(16.7), stl: sec(17.6), close: sec(18.8), card: sec(20.6), click: sec(22), end: sec(23.7) };
/** Scene length (s) for the timeline. */
export const PHYSICAL_S = P.end / 30;

/** The hands type the whole opening sentence (the skeleton is still typing when the orbit reaches the
 * board view); the read-out under them shows its first clause. */
const TYPED = data.language.opening;
const TEXT = TYPED.split(",")[0];
const TYPING = typing(TYPED, data.layout.final, { wpm: 78 });
const r = TYPING.rigs[1].home.index;
/** Characters of TYPED typed after each press (strokesFor skips characters the layout lacks). */
const PRESS_END = (() => {
  const keys = new Set([...data.layout.final]);
  return [...TYPED].flatMap((ch, i) => (keys.has(ch === " " ? "␣" : ch) || keys.has(ch.toLowerCase()) ? [i + 1] : []));
})();

/** Centre of the left half's cap tops: the low front shot aims at it. */
const LEFT = (() => {
  const ks = FLAT_POSES.map((_, i) => i).filter((i) => isLeft(i) && i < 36).map((i) => keyTop(i));
  return ks.reduce((a, p) => a.add(p), keyTop(0).multiplyScalar(0)).multiplyScalar(1 / ks.length);
})();

type V = [number, number, number];
/** An orbit pose about a target: azimuth from +z (the typist) toward +x, elevation (deg), radius (mm). */
interface Orbit { target: V; az: number; el: number; r: number; fov: number }
const orbitPos = (o: Orbit): V => {
  const a = THREE.MathUtils.degToRad(o.az), e = THREE.MathUtils.degToRad(o.el);
  return [o.target[0] + o.r * Math.sin(a) * Math.cos(e), o.target[1] + o.r * Math.sin(e), o.target[2] + o.r * Math.cos(a) * Math.cos(e)];
};
const toOrbit = (pos: V, target: V, fov: number, turns = 0): Orbit => {
  const d = [pos[0] - target[0], pos[1] - target[1], pos[2] - target[2]], r = Math.hypot(...d);
  return { target, r, fov, el: THREE.MathUtils.radToDeg(Math.asin(d[1] / r)), az: THREE.MathUtils.radToDeg(Math.atan2(d[0], d[2])) + 360 * turns };
};
const mix = (a: Orbit, b: Orbit, t: number): Orbit => ({ target: [0, 1, 2].map((k) => lerp(a.target[k], b.target[k], t)) as V,
  az: lerp(a.az, b.az, t), el: lerp(a.el, b.el, t), r: lerp(a.r, b.r, t), fov: lerp(a.fov, b.fov, t) });

/** The orbit's stops: the end of the skin shot, the skeleton (past the side, a little toward the
 * board's far edge), the end of its slow drift, and the board view, one turn on in the same direction. */
const SKIN = toOrbit([LEFT.x + 10, 32, LEFT.z + 270], [LEFT.x + 20, 37, LEFT.z], 36);
const BONES: Orbit = { target: [r[0] - 90, 35, r[2] + 10], az: 115, el: 24, r: 575, fov: 30 };
const DRIFT_DEG = 0.3; // per frame through the bone shot
const DRIFTED: Orbit = { target: [r[0] - 80, 30, r[2] + 20], az: BONES.az + DRIFT_DEG * (P.hold - P.bones), el: 31, r: 585, fov: 30 };
const BOARD = toOrbit([-60, 470, 640], [130, 10, 20], 30, 1);

/** The camera along the orbit at frame f (P.side..P.swing). Angular speed is continuous: a sin² burst
 * from rest into the drift speed, the drift, then a second burst out of the drift that comes to rest
 * on the board view. Target, elevation and radius follow the azimuth. */
function orbitAt(f: number): Orbit {
  const burst = (x: number) => x / 2 - Math.sin(2 * Math.PI * x) / (4 * Math.PI); // ∫ sin²(πx)
  if (f < P.bones) {
    const L = P.bones - P.side, x = (f - P.side) / L;
    const a = (2 * (BONES.az - SKIN.az)) / L - DRIFT_DEG;
    const az = SKIN.az + L * (a * burst(x) + (DRIFT_DEG * x * x) / 2);
    return { ...mix(SKIN, BONES, (az - SKIN.az) / (BONES.az - SKIN.az)), az };
  }
  if (f < P.hold) return mix(BONES, DRIFTED, (f - P.bones) / (P.hold - P.bones));
  const L = P.swing - P.hold, x = clamp01((f - P.hold) / L);
  const b = (2 * (BOARD.az - DRIFTED.az)) / L - DRIFT_DEG;
  const az = DRIFTED.az + L * (b * burst(x) + DRIFT_DEG * (x - (x * x) / 2));
  return { ...mix(DRIFTED, BOARD, (az - DRIFTED.az) / (BOARD.az - DRIFTED.az)), az };
}

const PATH: Shot[] = [
  // Low and level with the board, in front of the left half, facing it.
  { f: 0, pos: [LEFT.x - 30, 30, LEFT.z + 290], target: [LEFT.x + 10, 37, LEFT.z], fov: 36 },
  { f: P.side, pos: orbitPos(SKIN), target: SKIN.target, fov: SKIN.fov, ease: EASE.linear },
  // One orbit: past the side for the skeleton, then on round the board to the board view.
  ...along(P.side + 2, P.swing, (t) => { const o = orbitAt(lerp(P.side + 2, P.swing, t)); return { pos: orbitPos(o), target: o.target, fov: o.fov }; }),
  { f: P.swing, pos: orbitPos(BOARD), target: BOARD.target, fov: BOARD.fov, ease: EASE.linear },
  { f: P.morphEnd, pos: [-40, 340, 640], target: [130, 45, 10], fov: 30, ease: EASE.inOut },
  { f: P.close + sec(1.6), pos: [-300, 330, 470], target: [0, 40, 0], fov: 30, ease: EASE.inOut },
  { f: P.end, pos: [-120, 300, 520], target: [0, 45, 0], fov: 30, ease: EASE.inOut },
];

const RANGE_LABEL: Record<string, string> = { "split.tent": "Tent", "fingers.row_curve": "Row curve", "split.angle": "Split angle", "thumbs.tilt": "Thumb tilt" };
const RANGES = data.boardOpt.ranges.map((x) => ({ label: RANGE_LABEL[x.name] ?? x.name, lo: x.lo, hi: x.hi, start: x.start, value: x.value }));
const caseFile = data.stl.build.files.find((f) => f.file.includes("case")) ?? data.stl.build.files[0];
/** The STL parts shown: the case only (no bottom plate for now). */
const STL_PARTS = ["case"] as const;

/** Bone-length labels: one per finger segment of the hand nearest the side camera (mm). */
function BoneLabels({ frame, frames, reveal }: { frame: number; frames: ReturnType<typeof TYPING.frames>; reveal: number }) {
  const hand = frames.find((f) => f.side === "R");
  if (!hand || reveal <= 0) return null;
  const segs: { at: [number, number]; mm: number }[] = [];
  hand.pose.fingers.forEach((ch) => ch.slice(0, -1).forEach((p, j) => {
    const q = ch[j + 1], mid: [number, number, number] = [(p[0] + q[0]) / 2, (p[1] + q[1]) / 2 + 3, (p[2] + q[2]) / 2];
    const [a, b] = [project(PATH, frame, p), project(PATH, frame, q)];
    if (Math.hypot(b[0] - a[0], b[1] - a[1]) < 34) return; // too short on screen to label without overlap
    segs.push({ at: project(PATH, frame, mid), mm: Math.hypot(q[0] - p[0], q[1] - p[1], q[2] - p[2]) });
  }));
  return (
    <>
      {segs.map((g, i) => {
        const t = clamp01(reveal * 1.6 - (i / segs.length) * 0.6);
        return (
          <div key={i} style={{ position: "absolute", left: g.at[0], top: g.at[1], transform: "translate(-50%, -130%)", opacity: t,
            fontFamily: MONO, fontSize: 19, color: C.ink, background: "#ffffffd9", padding: "1px 5px", borderRadius: 4, whiteSpace: "nowrap" }}>
            {g.mm.toFixed(0)}
          </div>
        );
      })}
    </>
  );
}

/** The press read-out: the key, then reach, press angle and the scorer's costs for it. */
function PressStats({ rows, ch, reveal }: { rows: [string, string][]; ch: string; reveal: number }) {
  return (
    <div style={{ width: 380, opacity: reveal, transform: `translateY(${(1 - reveal) * 16}px)` }}>
      <div style={{ display: "flex", alignItems: "center", gap: 20, marginBottom: 18 }}>
        <div style={{ width: 76, height: 76, borderRadius: 12, background: C.accent, color: C.white, display: "flex", alignItems: "center", justifyContent: "center",
          fontSize: 44, fontWeight: WEIGHT.head }}>{ch.toUpperCase()}</div>
        <div style={{ fontSize: 26, color: C.muted }}>key pressed</div>
      </div>
      {rows.map(([k, v]) => (
        <div key={k} style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline", padding: "12px 0", borderBottom: `2px solid ${C.hairSoft}` }}>
          <span style={{ fontSize: 26, color: C.inkSoft }}>{k}</span>
          <span style={{ fontSize: 36, fontWeight: WEIGHT.head }}>{v}</span>
        </div>
      ))}
    </div>
  );
}

export function Physical() {
  const { frame } = useLayout();
  // Hands.
  const t = (frame - P.type) / 30;
  const typingOn = frame < P.board;
  const press = typingOn ? TYPING.keys(t) : FLAT_POSES.map(() => 0);
  const pr = TYPING.pressAt(t);
  const frames = TYPING.frames(t);
  const skin = 1 - progress(frame, P.skeleton, 24, EASE.inOut);
  const bones = interp(frame, [[P.skeleton - 4, 0], [P.skeleton + 16, 1], [P.board - 14, 1], [P.board, 0]]);
  const handsOn = frame < P.board + 2;
  // Board run.
  const morph = progress(frame, P.morph, P.morphEnd - P.morph, EASE.inOut);
  const stack = 1 - progress(frame, P.morph - 18, 18, EASE.linear);
  const stl = progress(frame, P.stl, sec(2.2), EASE.inOut);
  const close = progress(frame, P.close, sec(2.4), EASE.inOut);
  const poses = frame < P.morph ? FLAT_POSES : boardPosesAt(morph);
  const lift = lerp(lerp(0, BOARD_LIFT, progress(frame, P.morph - 12, 30, EASE.inOut)), STL_KEY_LIFT, close);
  const spread = lerp(36, 0, close);
  const accent = typingOn && pr && t < pr.up ? pr.key ?? null : null;

  // The press read-out: the latest press, its key and costs.
  const hand = frames.find((f) => f.side === (pr?.side ?? "R"))!;
  const key = pr?.key ?? -1;
  const rows: [string, string][] = [
    ["Reach", `${pr ? travelMm(pr).toFixed(0) : "0"} mm`],
    ["Press angle", `${pressAngle(hand.pose, pr?.finger ?? "index", pr?.n).toFixed(0)}°`],
    ["Reach cost", key >= 0 ? data.hand.reach[key].toFixed(2) : "–"],
    ["Wrist cost", key >= 0 ? data.hand.wrist[key].toFixed(2) : "–"],
  ];
  const statsIn = progress(frame, P.stats, 20, EASE.out) * (1 - progress(frame, P.skeleton - 10, 14, EASE.in));
  const done = TYPING.plan.presses.filter((q) => q.arrive <= t).length;
  const nTyped = Math.min(TEXT.length, done > 0 ? PRESS_END[Math.min(done, PRESS_END.length) - 1] : 0);

  // Overlays for the board run.
  const ui = progress(frame, P.tab - 10, 18, EASE.out) * (1 - progress(frame, P.stl - 10, 24, EASE.inOut));
  const tab = interp(frame, [[P.tab, 0], [P.tab + 14, 1]], EASE.inOut);
  const score = morph;
  const fadeIn = progress(frame, 0, 12, EASE.linear);
  const fadeOut = progress(frame, P.end - 12, 12, EASE.linear);
  return (
    <AbsoluteFill style={{ background: C.canvas, opacity: fadeIn * (1 - fadeOut) }}>
      <Scene3D>
        <CameraRig path={PATH} />
        <Corne3D chars={data.layout.final} poses={poses} press={press} accent={accent} stack={stack} tiles={(1 - stack) * (1 - close)}
          lift={lift} spread={spread} />
        {handsOn && <Hands3D typing={TYPING} t={t} skin={skin} skeleton={bones} />}
        {frame >= P.stl - 2 && <STLModel parts={[...STL_PARTS]} rise={stl} explode={1 - close} color="#23262e" />}
      </Scene3D>
      <AbsoluteFill>
        {statsIn > 0 && pr && (
          <div style={{ position: "absolute", right: MARGIN.x, top: MARGIN.y + 40 }}><PressStats rows={rows} ch={pr.ch} reveal={statsIn} /></div>
        )}
        {statsIn > 0 && (
          <div style={{ position: "absolute", left: MARGIN.x, bottom: MARGIN.y, fontSize: 34, opacity: statsIn, whiteSpace: "pre" }}>
            <span style={{ color: C.ink }}>{TEXT.slice(0, nTyped)}</span><span style={{ color: C.faint }}>{TEXT.slice(nTyped)}</span>
          </div>
        )}
        {frame >= P.skeleton && frame < P.board && (
          <BoneLabels frame={frame} frames={frames} reveal={progress(frame, P.bones - 6, 30, EASE.out) * (1 - progress(frame, P.board - 18, 12, EASE.in))} />
        )}
        {ui > 0 && (
          <>
            <div style={{ position: "absolute", left: 0, right: 0, top: MARGIN.y - 24, display: "flex", justifyContent: "center", alignItems: "center", gap: 24, opacity: ui,
              transform: `translateY(${(1 - ui) * -20}px)` }}>
              {/* the label sits left of the centred control without moving it */}
              <div style={{ position: "relative" }}>
                <span style={{ position: "absolute", right: "100%", top: "50%", transform: "translateY(-50%)", marginRight: 24, fontSize: 24, color: C.muted, whiteSpace: "nowrap" }}>Optimize for</span>
                <SegmentedTabs options={["Layout", "Board"]} active={tab} size={28} w={190} />
              </div>
            </div>
            <div style={{ position: "absolute", right: MARGIN.x, top: 300, width: 600, opacity: ui, display: "flex", flexDirection: "column", gap: 40 }}>
              <MetricCounter from={data.boardOpt.start} to={data.boardOpt.best} count={score} label="board score" accent={frame > P.morphEnd} size={150} align="left" />
              <RangeBars w={600} rows={RANGES} count={morph} />
              <div style={{ height: 44 }}>{frame > P.morphEnd && <StatusBadge kind="success" text="No collisions" start={P.morphEnd + 4} />}</div>
            </div>
          </>
        )}
        {frame >= P.card && (
          <div style={{ position: "absolute", right: MARGIN.x, bottom: MARGIN.y }}>
            <FileCard name="case.stl" size={`${Math.round(data.stl.case.bytes / 1024)} KB`} dims={caseFile.size_mm.map((v) => Math.round(v)).join(" × ") + " mm"}
              reveal={progress(frame, P.card, 40, EASE.inOut)} press={progress(frame, P.click, 10, EASE.linear)} width={720} />
          </div>
        )}
      </AbsoluteFill>
    </AbsoluteFill>
  );
}

