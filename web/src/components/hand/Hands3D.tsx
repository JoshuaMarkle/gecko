// The Hand tab's 3D hands: skeletons drawn as segments over the board, animated through the
// typing plan, and the camera region that frames them.

import { useFrame } from "@react-three/fiber";
import { Html } from "@react-three/drei";
import { useLayoutEffect, useMemo, useRef, type MutableRefObject } from "react";
import * as THREE from "three";
import type { Geometry } from "../../api";
import { ACCENT } from "../../colors";
import { dist3, FINGER_NAMES, fingerKey, keysDown, handLayout, sub3, type FingerState, type HandDims, type Plan, type V3 } from "../../hand";
import { boardBounds, type Focus, type ViewMode } from "../../three/Keyboard3D";
import { HandMeshes, padLift } from "./HandMesh";
import { framesAt, LOOP_GAP, MM, type Frame3, type Reach, type Rig } from "./rigs";

// ---------- 3D ----------

const BONE = "#3a3a3a";
const MOVE = "#5b8def";
const STATE_COLOR: Record<FingerState, string> = { rest: BONE, move: MOVE, ready: MOVE, press: ACCENT };
/** Hover reach easing: fraction of the remaining gap closed per second (exponential), and the most the tip lifts on the way (mm). */
const REACH_RATE = 12, REACH_ARC = 10;
const ZERO: V3 = [0, 0, 0];

/**
 * Hands as skinned meshes (`skin`) or as line segments with square joints, colored per finger
 * state, the palm a translucent outline. While not typing, `reach` sends one finger to a key
 * (eased there with a small lift, the hand following); without it the fingers ease back home.
 */
export function Hands({ rigs, dims, plan, clock, keys, playing, loop, onEnd, labels, skin, reach }: {
  rigs: Rig[]; dims: HandDims; plan: Plan | null; clock: MutableRefObject<number>; keys: MutableRefObject<Float32Array>;
  playing: boolean; loop: boolean; onEnd: () => void; labels: boolean; skin: boolean; reach: Reach | null;
}) {
  const nh = rigs.length;
  const outlineN = handLayout(dims).outline.length;
  const segN = nh * (15 + outlineN) * 2, jointN = nh * 20;
  const lines = useRef<THREE.LineSegments>(null);
  const points = useRef<THREE.Points>(null);
  const palmFill = useRef<THREE.Mesh>(null);
  const color = useMemo(() => new THREE.Color(), []);
  const lift = useMemo(() => (skin ? padLift(dims) : undefined), [skin, dims]);
  const rest = useMemo(() => framesAt(rigs, null, dims, -1, lift), [rigs, dims, lift]);
  const last = useRef<Frame3[]>(rest);
  /** Fingertip offset from home per finger off its home (`fingerKey`), eased toward the hovered key. */
  const ex = useRef(new Map<string, V3>());
  /** Set when the buffers show the rest pose but a finger is off home. */
  const stale = useRef(false);

  const draw = (frames: Frame3[]) => {
    last.current = frames;
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
    stale.current = ex.current.size > 0;
  });
  /** Eases the hover excursions one step; returns the fingers off home, or null when nothing moved. */
  const stepReach = (dt: number) => {
    const k = 1 - Math.exp(-REACH_RATE * dt);
    let moved = stale.current;
    const over: Record<string, { pos: V3; state: FingerState }> = {};
    for (const r of rigs) for (const f of FINGER_NAMES) {
      const id = fingerKey(r.side, f), home = r.home[f];
      const goal = reach && reach.side === r.side && reach.finger === f ? sub3(reach.pos, home) : ZERO;
      const e = ex.current.get(id) ?? ZERO;
      const gap = dist3(e, goal);
      if (e === ZERO && goal === ZERO) continue;
      const done = gap < 1e-3;
      if (done && goal === ZERO) { ex.current.delete(id); moved = true; continue; }
      const n: V3 = done ? (gap === 0 ? e : goal) : [e[0] + (goal[0] - e[0]) * k, e[1] + (goal[1] - e[1]) * k, e[2] + (goal[2] - e[2]) * k];
      moved ||= !done || n !== e;
      ex.current.set(id, n);
      const arc = Math.min(0.5 * gap, REACH_ARC * MM);
      over[id] = { pos: [home[0] + n[0], home[1] + n[1] + arc, home[2] + n[2]], state: done ? "ready" : "move" };
    }
    stale.current = false;
    return moved ? over : null;
  };
  useFrame((_, dt) => {
    if (!playing) {
      const over = stepReach(Math.min(dt, 0.1));
      if (over) draw(framesAt(rigs, null, dims, -1, lift, over));
      return;
    }
    ex.current.clear();
    if (!plan) return;
    const next = clock.current < 0 ? 0 : clock.current + Math.min(dt, 0.1);
    if (!loop && next > plan.end) { onEnd(); return; }
    clock.current = loop ? next % (plan.end + LOOP_GAP) : next;
    keysDown(plan, clock.current, keys.current);
    draw(framesAt(rigs, plan, dims, clock.current, lift));
  });

  return (
    <group key={`${nh}|${outlineN}`}>
      {skin && <HandMeshes sides={rigs.map((r) => r.side)} frames={last} />}
      <group visible={!skin}>
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
      </group>
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
export function handFocus(g: Geometry, rigs: Rig[], d: HandDims, mode: ViewMode): Focus {
  const b = boardBounds(g.key);
  const xs = [-b.w / 2, b.w / 2], zs = [-b.h / 2, b.h / 2];
  // Frame the unadjusted hands so the camera holds still while the wrist sliders move.
  for (const { pose } of framesAt(rigs.map((r) => ({ ...r, place: r.base })), null, d, -1)) for (const p of [...pose.palm, ...pose.fingers.flat()]) { xs.push(p[0]); zs.push(p[2]); }
  const [x0, x1, z0, z1] = [Math.min(...xs), Math.max(...xs), Math.min(...zs), Math.max(...zs)];
  // The tilted camera foreshortens depth (its fit pads a shallow board's depth, see CameraRig).
  return { cx: (x0 + x1) / 2, cz: (z0 + z1) / 2, w: x1 - x0, h: (mode === "3d" ? 0.62 : 1) * (z1 - z0) };
}
