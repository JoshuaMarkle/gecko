// Hand tab (frontend.md "Hand"): hand settings | 3D hands placed over the board's home keys
// (mirrored skeletons, finger IK), with a typing simulation whose fingers move in parallel |
// measurements, typing timeline and stats, size distribution, similarity.
// Preview only: the scorer has no hand model yet (#92).

import { useFrame } from "@react-three/fiber";
import { Html } from "@react-three/drei";
import { useEffect, useLayoutEffect, useMemo, useRef, useState, type MutableRefObject } from "react";
import * as THREE from "three";
import type { Finger, Geometry } from "../../api";
import { ACCENT, ERROR, fingerColor, HAIR, INK, MUTED } from "../../colors";
import {
  AVERAGE, dimsVector, FINGER_NAMES, fingerAt, fingerKey, keysDown, fingerLength, FOUR, handDims, handLayout, handLength, handShift,
  LENGTH_MEAN, LENGTH_SD, naturalTip, normalCdf, normalPdf, placeHand, planTyping, adjustPlacement, wristOf, WRIST0, population, poseHand, PRESET_SCALE,
  scaleDims, similarity, sizeBand, sub3, toWorld, typicality,
  type FingerName, type FingerState, type HandDims, type HandPose, type HandSettings, type Placement, type Plan, type Preset,
  type Side, type Stroke, type V3, type Wrist,
} from "../../hand";
import { boardBounds, CAP_H, Keyboard3D, place, type Focus, type ViewMode } from "../../three/Keyboard3D";
import { useSize } from "../Charts";
import { Frame, Num, PanelTabs, Sec, Seg, Slider, Toggle } from "../Form";

interface Props {
  geometry: Geometry;
  chars: string;
  hand: HandSettings;
  setHand: (h: HandSettings) => void;
}

const MM = 1 / 19.05;
const SAMPLES: [string, string][] = [
  ["pangram", "the quick brown fox jumps over the lazy dog"],
  ["prose", "it was the best of times, it was the worst of times, it was the age of wisdom"],
  ["code", "for (let i = 0; i < n; i++) { total += values[i]; }"],
];
const FINGER_OF: Record<string, FingerName> = { P: "pinky", R: "ring", M: "middle", I: "index", T: "thumb" };
const CODE_OF: Record<FingerName, string> = { pinky: "P", ring: "R", middle: "M", index: "I", thumb: "T" };
const SEG_NAMES: Record<FingerName, [string, string, string]> = {
  thumb: ["Metacarpal", "Proximal", "Distal"], index: ["Proximal", "Middle", "Distal"], middle: ["Proximal", "Middle", "Distal"],
  ring: ["Proximal", "Middle", "Distal"], pinky: ["Proximal", "Middle", "Distal"],
};
/** Pause at the end of the sample before it loops, seconds. */
const LOOP_GAP = 0.8;
const cap = (s: string) => s[0].toUpperCase() + s.slice(1);

// ---------- rigs and strokes ----------

/**
 * One hand on the board: its solved placement (`base`), the placement shown after the wrist
 * adjustments, how much the hand follows a reaching finger, and the fingertip home of each
 * finger (board-centred three coordinates).
 */
interface Rig { side: Side; base: Placement; place: Placement; follow: number; home: Record<FingerName, V3> }

/**
 * Hands for a board: the four fingers rest on their home keys (non-stretch first), the hand is
 * placed by `placeHand`, and the thumb rests on the thumb key nearest its natural rest point
 * (or on that point at key height when the board has no thumb keys), so both thumbs pick
 * mirrored keys on a symmetric board. The wrist adjustments move the shown hand only; the
 * fingertips keep their homes.
 */
function buildRigs(g: Geometry, d: HandDims, w: Wrist): { rigs: Rig[]; keyTop: (slot: number) => V3 } {
  const b = boardBounds(g.key);
  const keyTop = (i: number): V3 => { const p = place(g.key[i], b.cx, b.cy).p; return [p[0], p[1] + CAP_H, p[2]]; };
  const rigs: Rig[] = [];
  for (const side of ["L", "R"] as const) {
    const home = {} as Record<FingerName, V3>;
    for (const f of FOUR) {
      const ks = g.key.map((k, i) => ({ k, i })).filter((x) => x.k.finger === `${side}${CODE_OF[f]}`);
      const pick = ks.find((x) => x.k.home && !x.k.stretch) ?? ks.find((x) => x.k.home) ?? ks[0];
      if (pick) home[f] = keyTop(pick.i);
    }
    if (FOUR.some((f) => !home[f])) continue;
    const pl = placeHand(side, home, d, MM);
    const rest = toWorld(side, pl, MM, handLayout(d).thumbRest);
    const thumbs = g.key.map((k, i) => ({ k, i })).filter((x) => x.k.finger === `${side}T`).map((x) => keyTop(x.i));
    const near = (p: V3) => Math.hypot(p[0] - rest[0], p[2] - rest[2]);
    home.thumb = thumbs.length ? thumbs.reduce((a, p) => (near(p) < near(a) ? p : a)) : [rest[0], Math.min(...FOUR.map((f) => home[f][1])), rest[2]];
    rigs.push({ side, base: pl, place: adjustPlacement(side, pl, w, MM), follow: w.follow, home });
  }
  return { rigs, keyTop };
}

/** Keystrokes for a text on the loaded layout. Unmapped spaces go to a thumb, alternating away from the last hand. */
function strokesFor(text: string, g: Geometry, chars: string, rigs: Rig[], keyTop: (i: number) => V3): Stroke[] {
  const slotOf = new Map<string, number>();
  [...chars].forEach((c, i) => { if (i < g.key.length && !slotOf.has(c)) slotOf.set(c, i); });
  const sides = new Set(rigs.map((r) => r.side));
  const homeOf = (s: Side) => rigs.find((r) => r.side === s)!.home;
  const out: Stroke[] = [];
  for (const ch of text) {
    const slot = slotOf.get(ch === " " ? "␣" : ch) ?? slotOf.get(ch.toLowerCase());
    if (slot !== undefined) {
      const f = g.key[slot].finger;
      const side = f[0] as Side;
      if (sides.has(side)) out.push({ side, finger: FINGER_OF[f[1]], pos: keyTop(slot), ch, key: slot });
    } else if (ch === " " && sides.size) {
      const prev = out[out.length - 1]?.side;
      let side: Side = prev === "R" ? "L" : "R";
      if (!sides.has(side)) side = side === "L" ? "R" : "L";
      out.push({ side, finger: "thumb", pos: homeOf(side).thumb, ch });
    }
  }
  return out;
}

// ---------- 3D ----------

const BONE = "#3a3a3a";
const MOVE = "#5b8def";
const STATE_COLOR: Record<FingerState, string> = { rest: BONE, move: MOVE, ready: MOVE, press: ACCENT };

interface Frame3 { pose: HandPose; states: Record<FingerName, FingerState> }

/** Both hands at time `t` of the plan (`t` < 0: resting). The hand follows its fingers' excursions; resting fingers ride along. */
function framesAt(rigs: Rig[], plan: Plan | null, d: HandDims, t: number): Frame3[] {
  return rigs.map((r) => {
    const states = {} as Record<FingerName, FingerState>;
    const pos = {} as Record<FingerName, V3>;
    const ex: [FingerName, V3][] = [];
    for (const f of FINGER_NAMES) {
      const a = plan && t >= 0 ? fingerAt(plan.byFinger[fingerKey(r.side, f)], r.home[f], t, MM) : { pos: r.home[f], state: "rest" as const };
      states[f] = a.state;
      pos[f] = a.pos;
      ex.push([f, sub3(a.pos, r.home[f])]);
    }
    const off = handShift(ex, r.follow);
    const targets = {} as Record<FingerName, V3>;
    for (const f of FINGER_NAMES) targets[f] = states[f] === "rest" ? [r.home[f][0] + off[0], r.home[f][1], r.home[f][2] + off[2]] : pos[f];
    return { pose: poseHand(r.side, r.place, targets, d, MM, off), states };
  });
}

/** Hands as line segments with square joints, colored per finger state; the palm as a translucent outline. */
function Hands({ rigs, dims, plan, clock, keys, playing, loop, onEnd, labels }: {
  rigs: Rig[]; dims: HandDims; plan: Plan | null; clock: MutableRefObject<number>; keys: MutableRefObject<Float32Array>;
  playing: boolean; loop: boolean; onEnd: () => void; labels: boolean;
}) {
  const nh = rigs.length;
  const outlineN = handLayout(dims).outline.length;
  const segN = nh * (15 + outlineN) * 2, jointN = nh * 20;
  const lines = useRef<THREE.LineSegments>(null);
  const points = useRef<THREE.Points>(null);
  const palmFill = useRef<THREE.Mesh>(null);
  const color = useMemo(() => new THREE.Color(), []);
  const rest = useMemo(() => framesAt(rigs, null, dims, -1), [rigs, dims]);

  const draw = (frames: Frame3[]) => {
    const [ls, ps, pf] = [lines.current, points.current, palmFill.current];
    if (!ls || !ps || !pf) return;
    const lp = ls.geometry.attributes.position as THREE.BufferAttribute, lc = ls.geometry.attributes.color as THREE.BufferAttribute;
    const pp = ps.geometry.attributes.position as THREE.BufferAttribute, pc = ps.geometry.attributes.color as THREE.BufferAttribute;
    const fill = pf.geometry.attributes.position as THREE.BufferAttribute;
    let li = 0, ji = 0, fi = 0;
    const seg = (p: V3, q: V3) => {
      lp.setXYZ(li, p[0], p[1], p[2]); lc.setXYZ(li++, color.r, color.g, color.b);
      lp.setXYZ(li, q[0], q[1], q[2]); lc.setXYZ(li++, color.r, color.g, color.b);
    };
    for (const { pose, states } of frames) {
      pose.fingers.forEach((ch, f) => {
        color.set(STATE_COLOR[states[FINGER_NAMES[f]]]);
        ch.forEach((p, s) => {
          pp.setXYZ(ji, p[0], p[1], p[2]); pc.setXYZ(ji++, color.r, color.g, color.b);
          if (s < ch.length - 1) seg(p, ch[s + 1]);
        });
      });
      color.set(BONE);
      const ring = pose.palm;
      const cx = ring.reduce((t, p) => t + p[0], 0) / ring.length, cy = ring.reduce((t, p) => t + p[1], 0) / ring.length, cz = ring.reduce((t, p) => t + p[2], 0) / ring.length;
      ring.forEach((p, i) => {
        const q = ring[(i + 1) % ring.length];
        seg(p, q);
        fill.setXYZ(fi++, cx, cy, cz);
        fill.setXYZ(fi++, p[0], p[1], p[2]);
        fill.setXYZ(fi++, q[0], q[1], q[2]);
      });
    }
    for (const a of [lp, lc, pp, pc, fill]) a.needsUpdate = true;
    pf.geometry.computeVertexNormals();
  };

  // Rest pose whenever the inputs change or typing stops; the clock restarts from the top.
  useLayoutEffect(() => {
    if (playing) return;
    clock.current = -1;
    keys.current.fill(0);
    draw(rest);
  });
  useFrame((_, dt) => {
    if (!playing || !plan) return;
    const next = clock.current < 0 ? 0 : clock.current + Math.min(dt, 0.1);
    if (!loop && next > plan.end) { onEnd(); return; }
    clock.current = loop ? next % (plan.end + LOOP_GAP) : next;
    keysDown(plan, clock.current, keys.current);
    draw(framesAt(rigs, plan, dims, clock.current));
  });

  return (
    <group key={`${nh}|${outlineN}`}>
      <lineSegments ref={lines} renderOrder={10}>
        <bufferGeometry>
          <bufferAttribute attach="attributes-position" args={[new Float32Array(segN * 3), 3]} />
          <bufferAttribute attach="attributes-color" args={[new Float32Array(segN * 3), 3]} />
        </bufferGeometry>
        <lineBasicMaterial vertexColors depthTest={false} transparent />
      </lineSegments>
      <points ref={points} renderOrder={11}>
        <bufferGeometry>
          <bufferAttribute attach="attributes-position" args={[new Float32Array(jointN * 3), 3]} />
          <bufferAttribute attach="attributes-color" args={[new Float32Array(jointN * 3), 3]} />
        </bufferGeometry>
        <pointsMaterial vertexColors size={6} sizeAttenuation={false} depthTest={false} transparent />
      </points>
      <mesh ref={palmFill}>
        <bufferGeometry><bufferAttribute attach="attributes-position" args={[new Float32Array(nh * outlineN * 9), 3]} /></bufferGeometry>
        <meshStandardMaterial color="#9a9a9a" transparent opacity={0.22} side={THREE.DoubleSide} depthWrite={false} />
      </mesh>
      {labels && !playing && rest.map(({ pose }, h) => pose.fingers.map((ch, fi) => ch.slice(0, -1).map((a, si) => {
        const b = ch[si + 1];
        return (
          <Html key={`${h}-${fi}-${si}`} position={[(a[0] + b[0]) / 2, (a[1] + b[1]) / 2 + 0.25, (a[2] + b[2]) / 2]} center zIndexRange={[1, 0]}>
            <span className="handlabel">{dims[FINGER_NAMES[fi]][si].toFixed(0)}</span>
          </Html>
        );
      })))}
    </group>
  );
}

/** Camera region covering the board and both resting hands. */
function handFocus(g: Geometry, rigs: Rig[], d: HandDims, mode: ViewMode): Focus {
  const b = boardBounds(g.key);
  const xs = [-b.w / 2, b.w / 2], zs = [-b.h / 2, b.h / 2];
  // Frame the unadjusted hands so the camera holds still while the wrist sliders move.
  for (const { pose } of framesAt(rigs.map((r) => ({ ...r, place: r.base })), null, d, -1)) for (const p of [...pose.palm, ...pose.fingers.flat()]) { xs.push(p[0]); zs.push(p[2]); }
  const [x0, x1, z0, z1] = [Math.min(...xs), Math.max(...xs), Math.min(...zs), Math.max(...zs)];
  // The tilted camera foreshortens depth (its fit pads a shallow board's depth, see CameraRig).
  return { cx: (x0 + x1) / 2, cz: (z0 + z1) / 2, w: x1 - x0, h: (mode === "3d" ? 0.62 : 1) * (z1 - z0) };
}

// ---------- bottom panels ----------

/** One right hand, palm down, turned so the fingers point right (thumb on top), with dimension lines. */
function Measurements({ d }: { d: HandDims }) {
  const [box, { w, h }] = useSize<HTMLDivElement>();
  const lay = handLayout(d);
  // Hand-local mm → drawing mm: forward (−z) to the right, toward the pinky (+x) down.
  const P = (p: V3): [number, number] => [-p[2], p[0]];
  const fingers = FOUR.map((f, i) => {
    const k = lay.knuckles[i];
    const pts: [number, number][] = [P(k)];
    let z = k[2];
    for (const l of d[f]) { z -= l; pts.push(P([k[0], 0, z])); }
    return pts;
  });
  // Thumb: out from its base toward the index side, bending a little toward the fingers.
  const thumb: [number, number][] = [P(lay.thumb)];
  d.thumb.forEach((l, j) => {
    const a = ((48 - 10 * j) * Math.PI) / 180;
    const [x, y] = thumb[j];
    thumb.push([x + l * Math.cos(a), y - l * Math.sin(a)]);
  });
  const outline = lay.outline.map(P);
  const all = [...fingers.flat(), ...thumb, ...outline];
  const xMax = Math.max(...all.map((p) => p[0])), yMin = Math.min(...all.map((p) => p[1])), yMax = Math.max(...all.map((p) => p[1]));
  const indexTip = fingers[0][3], middleTip = fingers[1][3], pinkyTip = fingers[3][3], thumbTip = thumb[3];
  // Fit: room for the reach circle on the right, dimension lines below, labels left and top.
  const mx0 = -48, mx1 = Math.max(xMax, indexTip[0] + d.reach) + 34, my0 = Math.min(yMin, indexTip[1] - d.reach) - 10, my1 = yMax + 40;
  const s = Math.max(0.1, Math.min((w - 16) / (mx1 - mx0), (h - 12) / (my1 - my0)));
  const ox = (w - (mx1 - mx0) * s) / 2 - mx0 * s, oy = (h - (my1 - my0) * s) / 2 - my0 * s;
  const X = (x: number) => ox + x * s, Y = (y: number) => oy + y * s;
  const els: React.ReactNode[] = [];
  const chain = (pts: [number, number][], lens: number[], key: string, below: boolean) => {
    pts.forEach(([x, y], j) => {
      els.push(<circle key={`${key}j${j}`} cx={X(x)} cy={Y(y)} r={j ? 2 : 2.6} fill={INK} />);
      if (j === 0) return;
      const [px, py] = pts[j - 1];
      els.push(<line key={`${key}s${j}`} x1={X(px)} y1={Y(py)} x2={X(x)} y2={Y(y)} stroke={INK} strokeWidth={1.4} />);
      els.push(<text key={`${key}t${j}`} x={(X(px) + X(x)) / 2} y={(Y(py) + Y(y)) / 2 + (below ? 12 : -5)} textAnchor="middle">{lens[j - 1].toFixed(0)}</text>);
    });
  };
  fingers.forEach((pts, i) => chain(pts, d[FOUR[i]], FOUR[i], false));
  chain(thumb, d.thumb, "thumb", false);
  const kn = fingers.map((f) => f[0]);
  const edge = (d.palmWidth - 0.76 * d.palmWidth) / 2;
  const dimY1 = yMax + 14, dimY2 = yMax + 30;
  const pwX = kn[1][0] - 10;
  return (
    <div className="measure">
      <div ref={box} className="chartfill">
        <svg width={w} height={h}>
          <defs><marker id="arr" viewBox="0 0 6 6" refX="3" refY="3" markerWidth="6" markerHeight="6" orient="auto-start-reverse"><path d="M0,0L6,3L0,6" fill={ACCENT} /></marker></defs>
          <path d={outline.map(([x, y], i) => `${i ? "L" : "M"}${X(x)},${Y(y)}`).join(" ") + " Z"} fill="#f6f6f6" stroke={MUTED} />
          {/* reach around the index tip */}
          <circle cx={X(indexTip[0])} cy={Y(indexTip[1])} r={d.reach * s} fill="none" stroke={ACCENT} strokeDasharray="3 3" opacity={0.6} />
          <text x={X(indexTip[0] + d.reach) + 4} y={Y(indexTip[1]) - 4}>reach {d.reach.toFixed(0)}</text>
          {/* span */}
          <line x1={X(thumbTip[0])} y1={Y(thumbTip[1])} x2={X(pinkyTip[0])} y2={Y(pinkyTip[1])} stroke={HAIR} strokeDasharray="4 3" />
          <text x={X(thumbTip[0]) + 6} y={Y(thumbTip[1]) - 4}><title>Thumb tip to pinky tip, spread flat</title>span {d.span.toFixed(0)}</text>
          {els}
          {/* palm width, across the knuckles */}
          <line x1={X(pwX)} x2={X(pwX)} y1={Y(kn[0][1] - edge)} y2={Y(kn[3][1] + edge)} stroke={ACCENT} markerEnd="url(#arr)" markerStart="url(#arr)" />
          <text className="ink" x={X(pwX) - 6} y={Y((kn[0][1] + kn[3][1]) / 2) + 4} textAnchor="end">palm width {d.palmWidth.toFixed(0)}</text>
          {/* palm length and hand length, wrist crease to the middle knuckle / tip */}
          <line x1={X(0)} x2={X(0)} y1={Y(yMin)} y2={Y(dimY2) + 4} stroke={HAIR} />
          <line x1={X(kn[1][0])} x2={X(0) + (kn[1][0]) * s} y1={Y(kn[1][1])} y2={Y(dimY1) + 4} stroke={HAIR} />
          <line x1={X(middleTip[0])} x2={X(middleTip[0])} y1={Y(middleTip[1])} y2={Y(dimY2) + 4} stroke={HAIR} />
          <line x1={X(0)} x2={X(kn[1][0])} y1={Y(dimY1)} y2={Y(dimY1)} stroke={ACCENT} markerEnd="url(#arr)" markerStart="url(#arr)" />
          <text className="ink" x={X(kn[1][0] / 2)} y={Y(dimY1) - 4} textAnchor="middle">palm length {d.palmLength.toFixed(0)}</text>
          <line x1={X(0)} x2={X(middleTip[0])} y1={Y(dimY2)} y2={Y(dimY2)} stroke={ACCENT} markerEnd="url(#arr)" markerStart="url(#arr)" />
          <text className="ink" x={X(middleTip[0] / 2)} y={Y(dimY2) + 14} textAnchor="middle">hand length {handLength(d).toFixed(0)}</text>
          <text x={X(0) - 6} y={Y((yMin + yMax) / 2)} textAnchor="end">wrist</text>
        </svg>
      </div>
      <div className="measurelist">
        <div className="kv"><span>Hand length</span><span>{handLength(d).toFixed(0)} mm</span></div>
        <div className="kv"><span>Palm</span><span>{d.palmWidth.toFixed(0)} × {d.palmLength.toFixed(0)} mm</span></div>
        {FINGER_NAMES.map((f) => <div key={f} className="kv"><span>{cap(f)}</span><span>{fingerLength(d, f).toFixed(0)} mm</span></div>)}
        <div className="kv"><span title="Knuckle to fingertip, forward, in the natural typing curl (middle finger)">Curled reach</span><span>{naturalTip(d.middle)[0].toFixed(0)} mm · {(naturalTip(d.middle)[0] * MM).toFixed(2)} u</span></div>
        <div className="kv"><span title="Centre to centre, index to pinky knuckles, divided by 3">Knuckle pitch</span><span>{((0.76 * d.palmWidth) / 3).toFixed(1)} mm</span></div>
        <div className="kv"><span>Span · reach</span><span>{d.span.toFixed(0)} · {d.reach.toFixed(0)} mm</span></div>
      </div>
    </div>
  );
}

/** Lanes top → bottom: left pinky … left thumb, right thumb … right pinky. */
const LANES: [Side, FingerName][] = [
  ["L", "pinky"], ["L", "ring"], ["L", "middle"], ["L", "index"], ["L", "thumb"],
  ["R", "thumb"], ["R", "index"], ["R", "middle"], ["R", "ring"], ["R", "pinky"],
];
/** Seconds shown in the timeline; the playhead sits at PLAYHEAD of the width. */
const WINDOW = 2.4;
const PLAYHEAD = 0.65;

/** Per-finger timeline of the typing plan (moves light, presses solid) with a playhead, plus stats. */
function Typing({ plan, clock, playing }: { plan: Plan | null; clock: MutableRefObject<number>; playing: boolean }) {
  const [box, { w, h }] = useSize<HTMLDivElement>();
  const cv = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const el = cv.current;
    if (!el || !w || !h) return;
    const dpr = window.devicePixelRatio || 1;
    el.width = w * dpr;
    el.height = h * dpr;
    const g = el.getContext("2d")!;
    let raf = 0;
    const draw = () => {
      g.setTransform(dpr, 0, 0, dpr, 0, 0);
      g.clearRect(0, 0, w, h);
      g.font = "11px Inter, -apple-system, Helvetica, Arial, sans-serif";
      const lx = 70, top = 18, lane = (h - top - 16) / LANES.length;
      const now = clock.current;
      const t0 = playing && now >= 0 ? now - PLAYHEAD * WINDOW : -0.1;
      const sx = (t: number) => lx + ((t - t0) / WINDOW) * (w - lx - 4);
      g.textBaseline = "middle";
      LANES.forEach(([side, f], i) => {
        const y = top + i * lane;
        g.fillStyle = i % 2 ? "#fafafa" : "#f3f3f3";
        g.fillRect(lx, y, w - lx, lane);
        g.fillStyle = MUTED;
        g.fillText(`${side} ${f}`, 4, y + lane / 2);
      });
      g.fillStyle = HAIR;
      g.fillRect(lx, top + 5 * lane - 0.5, w - lx, 1);
      if (plan) {
        const period = plan.end + LOOP_GAP;
        for (const shift of [-period, 0, period]) {
          for (const pr of plan.presses) {
            const i = LANES.findIndex(([s, f]) => s === pr.side && f === pr.finger);
            const y = top + i * lane;
            const [a, r, b, c] = [sx(pr.depart + shift), sx(pr.ready + shift), sx(pr.arrive + shift), sx(pr.up + shift)];
            if (c < lx || a > w) continue;
            const col = fingerColor(`${pr.side}${CODE_OF[pr.finger]}` as Finger);
            g.globalAlpha = 0.35;
            g.fillStyle = col;
            g.fillRect(Math.max(a, lx), y + lane * 0.3, Math.max(0, r - Math.max(a, lx)), lane * 0.4);
            g.fillRect(Math.max(r, lx), y + lane * 0.46, Math.max(0, b - Math.max(r, lx)), lane * 0.08);
            if (Number.isFinite(pr.back)) g.fillRect(Math.max(c, lx), y + lane * 0.38, Math.max(0, sx(pr.back + shift) - Math.max(c, lx)), lane * 0.24);
            g.globalAlpha = 1;
            g.fillRect(Math.max(b, lx), y + lane * 0.15, Math.max(1.5, c - Math.max(b, lx)), lane * 0.7);
            g.fillStyle = INK;
            if (b >= lx) g.fillText(pr.ch === " " ? "␣" : pr.ch, c + 3, y + lane / 2);
            if (pr.limit !== "rhythm" && b >= lx) {
              g.fillStyle = pr.limit === "finger" ? ERROR : MUTED;
              g.beginPath();
              g.moveTo(b, y + lane * 0.15); g.lineTo(b - 4, y + 1); g.lineTo(b + 4, y + 1);
              g.fill();
            }
          }
        }
      }
      g.fillStyle = MUTED;
      g.fillText("light: moving · thin: ready over the key · solid: key down · ▼ waited for the same finger (red) or a late start (grey)", lx, 8);
      if (playing && now >= 0) {
        g.fillStyle = ACCENT;
        g.fillRect(sx(now) - 0.75, top, 1.5, h - top - 16);
      }
      g.fillStyle = MUTED;
      g.textAlign = "right";
      g.fillText(`${WINDOW.toFixed(1)} s window`, w - 4, h - 7);
      g.textAlign = "left";
      if (playing) raf = requestAnimationFrame(draw);
    };
    draw();
    return () => cancelAnimationFrame(raf);
  }, [plan, playing, w, h, clock]);
  const st = plan?.stats;
  const pct = (x: number) => `${(100 * x).toFixed(0)}%`;
  const n = plan?.presses.length ?? 0;
  return (
    <div className="typingpanel">
      <div ref={box} className="chartfill"><canvas ref={cv} style={{ width: w, height: h, display: "block" }} /></div>
      {st && (
        <div className="measurelist">
          <div className="kv"><span title="Speed reached with every finger at its natural pace and this parallelism, vs the target: a key waits until its finger can get there">Speed</span><span>{st.wpm.toFixed(0)} of {st.targetWpm} wpm</span></div>
          <div className="kv"><span title="Keys that went down later than the target rhythm because their finger could not get there in time">Late keys</span><span>{st.fingerWaits + st.serialWaits} of {n}</span></div>
          <div className="kv"><span title="Keys whose finger set off before the previous key was released">Started early</span><span>{pct(st.early)}</span></div>
          <div className="kv"><span title="Share of the time with two or more fingers moving, waiting over a key, or pressing at once">Fingers overlap</span><span>{pct(st.overlap)}</span></div>
          <div className="kv"><span title="Waited because the same finger had just pressed its previous key">· same finger</span><span>{st.fingerWaits}</span></div>
          <div className="kv"><span title="Waited because the finger could not set off before the previous key was released (raise Parallel)">· late start</span><span>{st.serialWaits}</span></div>
          <div className="kv"><span>Hand alternation</span><span>{pct(st.alternation)}</span></div>
          <div className="kv"><span title="Fingertip travel per keystroke, including returns home">Travel / key</span><span>{st.travel.toFixed(1)} mm</span></div>
          <div className="kv"><span title="Presses after which the finger goes back to its home key before its next press">Home returns</span><span>{pct(st.homeReturns)}</span></div>
        </div>
      )}
      {st && (
        <div className="measurelist">
          <div className="kv head"><span>Finger</span><span>keys · mm · busy</span></div>
          {LANES.filter(([s, f]) => st.perFinger[fingerKey(s, f)]).map(([s, f]) => {
            const x = st.perFinger[fingerKey(s, f)];
            return <div key={s + f} className="kv"><span>{s} {f}</span><span>{x.presses} · {x.travel.toFixed(0)} · {pct(x.busy)}</span></div>;
          })}
        </div>
      )}
    </div>
  );
}

function SizeDistribution({ d }: { d: HandDims }) {
  const [box, { w, h }] = useSize<HTMLDivElement>();
  const m = { l: 16, r: 16, t: 26, b: 28 };
  const lo = LENGTH_MEAN - 3.5 * LENGTH_SD, hi = LENGTH_MEAN + 3.5 * LENGTH_SD;
  const sx = (x: number) => m.l + ((x - lo) / (hi - lo)) * (w - m.l - m.r);
  const peak = normalPdf(LENGTH_MEAN, LENGTH_MEAN, LENGTH_SD);
  const sy = (y: number) => h - m.b - (y / peak) * (h - m.t - m.b);
  const pts = Array.from({ length: 121 }, (_, i) => lo + ((hi - lo) * i) / 120);
  const curve = (a: number, b: number) => {
    const xs = pts.filter((x) => x >= a && x <= b);
    return `M${sx(xs[0])},${sy(0)} ` + xs.map((x) => `L${sx(x)},${sy(normalPdf(x, LENGTH_MEAN, LENGTH_SD))}`).join(" ") + ` L${sx(xs[xs.length - 1])},${sy(0)} Z`;
  };
  const cur = handLength(d);
  const pct = 100 * normalCdf((cur - LENGTH_MEAN) / LENGTH_SD);
  const band = (a: number, b: number, label: string, fill: string) => (
    <g>
      <path d={curve(a, b)} fill={fill} />
      <text x={sx((Math.max(a, lo) + Math.min(b, hi)) / 2)} y={h - 10} textAnchor="middle">{label}</text>
    </g>
  );
  return (
    <div ref={box} className="chartfill">
      <svg width={w} height={h}>
        {band(lo, LENGTH_MEAN - LENGTH_SD, "Small", "#ececec")}
        {band(LENGTH_MEAN - LENGTH_SD, LENGTH_MEAN + LENGTH_SD, "Average", "#dcdcdc")}
        {band(LENGTH_MEAN + LENGTH_SD, hi, "Large", "#ececec")}
        <path d={pts.map((x, i) => `${i ? "L" : "M"}${sx(x)},${sy(normalPdf(x, LENGTH_MEAN, LENGTH_SD))}`).join(" ")} fill="none" stroke={INK} strokeWidth={1.2} />
        {(["small", "average", "large"] as const).map((p) => {
          const x = sx(handLength(scaleDims(AVERAGE, PRESET_SCALE[p])));
          return <line key={p} x1={x} x2={x} y1={h - m.b} y2={h - m.b + 5} stroke={MUTED} />;
        })}
        <line x1={sx(cur)} x2={sx(cur)} y1={m.t - 6} y2={h - m.b} stroke={ACCENT} strokeWidth={2} />
        <text className="ink" x={sx(cur) + 6} y={m.t + 4}>this hand · {cur.toFixed(0)} mm · {sizeBand(cur)} · {pct.toFixed(0)}th percentile</text>
        <text x={m.l} y={14}>hand length (wrist crease to middle fingertip), adult population · bands at ±1 sd</text>
      </svg>
    </div>
  );
}

function Similarity({ d }: { d: HandDims }) {
  const [box, { w, h }] = useSize<HTMLDivElement>();
  const data = useMemo(() => {
    const pop = population(600, 7);
    const ref = pop.slice(0, 150);
    return { pop, typ: pop.map((v) => typicality(v, ref)), ref };
  }, []);
  const v = dimsVector(d);
  const mine = typicality(v, data.ref);
  const nearest = Math.max(...data.pop.map((p) => similarity(v, p)));
  const bins = 30;
  const lo = Math.min(...data.typ, mine), hi = Math.max(...data.typ, mine);
  const counts = new Array(bins).fill(0);
  for (const t of data.typ) counts[Math.min(bins - 1, Math.floor(((t - lo) / (hi - lo)) * bins))]++;
  const below = data.typ.filter((t) => t < mine).length / data.typ.length;
  const m = { l: 16, r: 16, t: 26, b: 24 };
  const bw = (w - m.l - m.r) / bins;
  const max = Math.max(...counts);
  const sx = (t: number) => m.l + ((t - lo) / (hi - lo)) * (w - m.l - m.r);
  return (
    <div ref={box} className="chartfill">
      <svg width={w} height={h}>
        <text x={m.l} y={14}>typicality: mean similarity to other hands (synthetic population until reference data lands)</text>
        {counts.map((c, i) => (
          <rect key={i} x={m.l + i * bw + 1} y={h - m.b - (c / max) * (h - m.t - m.b)} width={bw - 2} height={(c / max) * (h - m.t - m.b)} fill={HAIR} rx={1} />
        ))}
        <line x1={sx(mine)} x2={sx(mine)} y1={m.t - 6} y2={h - m.b} stroke={ACCENT} strokeWidth={2} />
        <text className="ink" x={Math.min(sx(mine) + 6, w - 260)} y={m.t + 4}>this hand · more typical than {(100 * below).toFixed(0)}% · closest match {(100 * nearest).toFixed(0)}%</text>
        <text x={m.l} y={h - 6}>less typical</text>
        <text x={w - m.r} y={h - 6} textAnchor="end">more typical</text>
      </svg>
    </div>
  );
}

// ---------- view ----------

export function HandView(p: Props) {
  const h = p.hand;
  const d = handDims(h);
  const off = !h.use;
  const [tab, setTab] = useState<"measure" | "typing" | "size" | "similar">("measure");
  const [sideTab, setSideTab] = useState<"hand" | "simulation">("hand");
  const [sample, setSample] = useState("pangram");
  const [wpm, setWpm] = useState(120);
  const [parallel, setParallel] = useState(1);
  const [playing, setPlaying] = useState(false);
  const [loop, setLoop] = useState(true);
  const [labels, setLabels] = useState(false);
  const [mode, setMode] = useState<ViewMode>("3d");
  const clock = useRef(-1);
  /** Press state per board key, written by the typing animation, read by the keycaps. */
  const keys = useRef(new Float32Array(0));
  if (keys.current.length !== p.geometry.key.length) keys.current = new Float32Array(p.geometry.key.length);
  const setDims = (next: HandDims) => p.setHand({ ...h, preset: "custom", custom: next });
  const setSeg = (f: FingerName, i: number, v: number) => { const s = [...d[f]] as [number, number, number]; s[i] = Math.max(1, v); setDims({ ...d, [f]: s }); };

  // `d` is rebuilt every render; key the memos on its values.
  const dimsKey = dimsVector(d).join(",");
  const w = wristOf(h);
  const wristKey = Object.values(w).join(",");
  const setWrist = (patch: Partial<Wrist>) => p.setHand({ ...h, wrist: { ...w, ...patch } });
  const { rigs, keyTop } = useMemo(() => buildRigs(p.geometry, d, w), [p.geometry, dimsKey, wristKey]);
  const focus = useMemo(() => handFocus(p.geometry, rigs, d, mode), [p.geometry, rigs, mode]);
  // Fingers that cannot reach their home key in the resting pose (wrist adjusted too far), with the gap in mm.
  const unreachable = useMemo(() => framesAt(rigs, null, d, -1).flatMap(({ pose }, hi) => FINGER_NAMES.map((f, fi) => {
    const tip = pose.fingers[fi][3], t = rigs[hi].home[f];
    return { name: `${rigs[hi].side} ${f}`, gap: Math.hypot(tip[0] - t[0], tip[1] - t[1], tip[2] - t[2]) / MM };
  })).filter((x) => x.gap > 1), [rigs, dimsKey]);
  const text = SAMPLES.find(([k]) => k === sample)?.[1] ?? "";
  const plan = useMemo(() => {
    if (!rigs.length) return null;
    const strokes = strokesFor(text, p.geometry, p.chars, rigs, keyTop);
    const homeOf = (s: Side, f: FingerName) => rigs.find((r) => r.side === s)!.home[f];
    return planTyping(strokes, homeOf, wpm, parallel, MM);
  }, [rigs, keyTop, text, p.geometry, p.chars, wpm, parallel]);

  return (
    <Frame
      title={<Seg inline value={sideTab} onChange={setSideTab} options={[["hand", "Hand"], ["simulation", "Simulation"]]} />}
      side={
        sideTab === "hand" ? <>
          <div className="pad">
            <Toggle label="Use hand in optimization" help="Score layouts with this hand's reach (needs the hand model in the scorer, #92). Off: preview only." value={h.use} onChange={(use) => p.setHand({ ...h, use })} />
            {off && <div className="hint">Off: the scorer ignores the hand. The views on the right still preview it.</div>}
          </div>
          <fieldset className="plain" disabled={off}>
            <div className="pad">
              <div className="row">
                <label title="Preset hand sizes. Editing a value below switches to Custom.">Preset</label>
                <Seg value={h.preset} disabled={off} onChange={(preset: Preset) => p.setHand({ ...h, preset })}
                  options={[["average", "Average"], ["large", "Large"], ["small", "Small"], ...(h.custom ? [["custom", "Custom"] as [Preset, string]] : [])]} />
                <span />
              </div>
              <div className="kv"><span>Hand length</span><span>{handLength(d).toFixed(0)} mm · {sizeBand(handLength(d))}</span></div>
            </div>
            <Sec id="hand-adv" title="Advanced" note="custom values, mm" defaultOpen={false}>
              {FINGER_NAMES.map((f) => (
                <div key={f} className="subgroup">
                  <div className="subhead">{cap(f)}</div>
                  {d[f].map((v, i) => (
                    <Num key={i} label={SEG_NAMES[f][i]} help={`${cap(f)} ${SEG_NAMES[f][i].toLowerCase()} segment length.`} value={+v.toFixed(1)} step={0.5} unit="mm" disabled={off} onChange={(x) => setSeg(f, i, x)} />
                  ))}
                </div>
              ))}
              <div className="subgroup">
                <div className="subhead">Palm and reach</div>
                <Num label="Palm width" help="Across the index–pinky knuckles." value={+d.palmWidth.toFixed(1)} step={0.5} unit="mm" disabled={off} onChange={(x) => setDims({ ...d, palmWidth: Math.max(30, x) })} />
                <Num label="Palm length" help="Wrist crease to the middle knuckle." value={+d.palmLength.toFixed(1)} step={0.5} unit="mm" disabled={off} onChange={(x) => setDims({ ...d, palmLength: Math.max(40, x) })} />
                <Num label="Span" help="Thumb tip to pinky tip, spread flat." value={+d.span.toFixed(1)} step={1} unit="mm" disabled={off} onChange={(x) => setDims({ ...d, span: Math.max(80, x) })} />
                <Num label="Reach" help="Comfortable fingertip travel from the home key." value={+d.reach.toFixed(1)} step={1} unit="mm" disabled={off} onChange={(x) => setDims({ ...d, reach: Math.max(5, x) })} />
              </div>
              <div className="btns"><button className="btn soft small" disabled={off || h.preset !== "custom"} onClick={() => p.setHand({ ...h, preset: "average", custom: null })}>Reset to Average</button></div>
            </Sec>
          </fieldset>
        </> : (
          <Sec id="hand-wrist" title="Wrist" note="view only">
            <div className="wrist">
              <Slider label="Height" help="Raise (+) or lower (−) the wrist and palm from where the fingers rest naturally; the fingers still reach the home keys." value={w.height} min={-30} max={40} step={1} unit={`${w.height} mm`} onChange={(height) => setWrist({ height })} />
              <Slider label="Distance" help="Move the wrists toward you (+) or toward the keyboard (−)." value={w.distance} min={-40} max={60} step={1} unit={`${w.distance} mm`} onChange={(distance) => setWrist({ distance })} />
              <Slider label="Spread" help="Move each hand away from (+) or toward (−) the middle of the board." value={w.spread} min={-40} max={60} step={1} unit={`${w.spread} mm`} onChange={(spread) => setWrist({ spread })} />
              <Slider label="Angle" help="Turn each hand about the wrist: + points the fingers more toward the middle of the board, − straighter ahead or outward." value={w.angle} min={-30} max={30} step={1} unit={`${w.angle}°`} onChange={(angle) => setWrist({ angle })} />
              <Slider label="Tilt" help="Roll each hand about the forearm: + lifts the thumb side so the pinky side sits closer to the keyboard (the index knuckle rises toward the middle); − the reverse." value={w.tilt} min={-30} max={45} step={1} unit={`${w.tilt}°`} onChange={(tilt) => setWrist({ tilt })} />
              <Slider label="Wrist movement" help="How much the whole hand follows a reaching finger while typing: 0 keeps the wrist planted (fingers stretch), 100% moves the hand with the finger." value={w.follow} min={0} max={1} step={0.05} unit={`${(100 * w.follow).toFixed(0)}%`} onChange={(follow) => setWrist({ follow })} />
              <div className="btns"><button className="btn soft small" disabled={Object.entries(WRIST0).every(([k, v]) => w[k as keyof Wrist] === v)} onClick={() => p.setHand({ ...h, wrist: WRIST0 })}>Reset wrist</button></div>
            </div>
          </Sec>
        )
      }
      main={
        <div className="viewport hand-viewport">
          <Keyboard3D geometry={p.geometry} chars={p.chars} heat={[]} arcs={[]} overlay="none" mode={mode} selected={null} onKey={() => {}} focus={focus} pressed={keys}>
            <Hands rigs={rigs} dims={d} plan={plan} clock={clock} keys={keys} playing={playing} loop={loop} onEnd={() => setPlaying(false)} labels={labels} />
          </Keyboard3D>
          <div className="viewlabel">
            <span>Hands on {p.geometry.name}</span>
            {unreachable.length > 0 && <span className="err" title="These fingertips cannot reach their home key from this wrist position; they point at it, fully extended.">out of reach: {unreachable.map((x) => `${x.name} ${x.gap.toFixed(0)} mm`).join(", ")}</span>}
            <span className="muted">{playing && plan ? `typing · ${plan.stats.wpm.toFixed(0)} of ${wpm} wpm reached · ${(100 * plan.stats.early).toFixed(0)}% of keys started early` : "fingertips on the home keys (IK)"}</span>
          </div>
          <div className="viewctl">
            <div className="seg inline"><button className={labels ? "on" : ""} onClick={() => setLabels(!labels)} title="Segment lengths (mm) on the resting hands">Lengths</button></div>
            <div className="seg inline"><button className={mode === "3d" ? "on" : ""} onClick={() => setMode(mode === "3d" ? "top" : "3d")} title="3D view (off: flat top-down)">3D</button></div>
          </div>
          <div className="typingbar">
            <button className="btn primary small" onClick={() => setPlaying(!playing)}>{playing ? "Stop" : "Play"}</button>
            <select value={sample} onChange={(e) => setSample(e.target.value)} title="Sample text">
              {SAMPLES.map(([k, t]) => <option key={k} value={k}>{t.length > 34 ? `${t.slice(0, 34)}…` : t}</option>)}
            </select>
            <div className="seg inline"><button className={loop ? "on" : ""} onClick={() => setLoop(!loop)} title="Repeat the sample (off: type it once, then stop)">Loop</button></div>
            <input type="range" min={5} max={200} step={5} value={wpm} onChange={(e) => setWpm(+e.target.value)} title="Target words per minute" />
            <span className="muted num">{wpm} wpm</span>
            <span className="sep" />
            <label className="muted" title="How far the next finger moves while the previous key is still down. 0: one finger at a time (each starts after the previous release); 100%: the next finger travels during the previous press and waits over its key; above 100%: keys roll over (each held into the next press) and fingers get ready ever earlier; 300%: every finger heads for its next key as soon as it is free.">Parallel</label>
            <input type="range" min={0} max={3} step={0.05} value={parallel} onChange={(e) => setParallel(+e.target.value)} title="Finger overlap" />
            <span className="muted num">{(100 * parallel).toFixed(0)}%</span>
          </div>
        </div>
      }
      bottom={
        <div className="bottomtabs">
          <PanelTabs value={tab} onChange={setTab} options={[["measure", "Measurements"], ["typing", "Typing"], ["size", "Size distribution"], ["similar", "Similarity"]]}
            right={<span className="hint">{h.preset === "custom" ? "Custom" : cap(h.preset)} hand</span>} />
          <div className="bottombody">
            {tab === "measure" && <Measurements d={d} />}
            {tab === "typing" && <Typing plan={plan} clock={clock} playing={playing} />}
            {tab === "size" && <SizeDistribution d={d} />}
            {tab === "similar" && <Similarity d={d} />}
          </div>
        </div>
      }
    />
  );
}
