// Both hands typing on the Corne: the product's skinned meshes (`skin`) and its skeleton view
// (`skeleton`: bone lines, square joints, the palm outline), posed at time `t` of a typing plan
// (three/hands.ts). The pressing finger turns accent. Units: mm, the Corne3D scene.

import { useThree } from "@react-three/fiber";
import { useEffect, useMemo, useRef, useState } from "react";
import { continueRender, delayRender } from "remotion";
import * as THREE from "three";
import { C } from "../lib/tokens";
import { loadHand, Rigged } from "./HandMesh";
import { FINGER_NAMES, type Frame3, type Side, type Typing } from "./hands";

const INK = new THREE.Color(C.ink), ACCENT = new THREE.Color(C.accent);

/** Skeleton buffers sized for two hands: 15 finger bones + palm outline per hand. */
function Skeleton({ frames, opacity }: { frames: Frame3[]; opacity: number }) {
  const geo = useMemo(() => {
    const segs = frames.reduce((n, f) => n + f.pose.fingers.reduce((m, ch) => m + ch.length - 1, 0) + f.pose.palm.length, 0);
    const joints = frames.reduce((n, f) => n + f.pose.fingers.reduce((m, ch) => m + ch.length, 0) + 1, 0);
    const lines = new THREE.BufferGeometry(), points = new THREE.BufferGeometry();
    lines.setAttribute("position", new THREE.BufferAttribute(new Float32Array(segs * 6), 3));
    lines.setAttribute("color", new THREE.BufferAttribute(new Float32Array(segs * 6), 3));
    points.setAttribute("position", new THREE.BufferAttribute(new Float32Array(joints * 3), 3));
    points.setAttribute("color", new THREE.BufferAttribute(new Float32Array(joints * 3), 3));
    return { lines, points };
  }, [frames.length]); // eslint-disable-line react-hooks/exhaustive-deps
  const lp = geo.lines.attributes.position as THREE.BufferAttribute, lc = geo.lines.attributes.color as THREE.BufferAttribute;
  const pp = geo.points.attributes.position as THREE.BufferAttribute, pc = geo.points.attributes.color as THREE.BufferAttribute;
  let li = 0, ji = 0;
  const vert = (a: THREE.BufferAttribute, c: THREE.BufferAttribute, i: number, p: number[], col: THREE.Color) => {
    a.setXYZ(i, p[0], p[1], p[2]); c.setXYZ(i, col.r, col.g, col.b);
  };
  for (const { pose, states } of frames) {
    pose.fingers.forEach((ch, f) => {
      const col = states[FINGER_NAMES[f]] === "press" ? ACCENT : INK;
      ch.forEach((p, s) => {
        vert(pp, pc, ji++, p, col);
        if (s < ch.length - 1) { vert(lp, lc, li++, p, col); vert(lp, lc, li++, ch[s + 1], col); }
      });
    });
    vert(pp, pc, ji++, pose.wrist, INK);
    pose.palm.forEach((p, i) => { vert(lp, lc, li++, p, INK); vert(lp, lc, li++, pose.palm[(i + 1) % pose.palm.length], INK); });
  }
  for (const a of [lp, lc, pp, pc]) a.needsUpdate = true;
  geo.lines.computeBoundingSphere(); geo.points.computeBoundingSphere();
  if (opacity <= 0.001) return null;
  return (
    <group renderOrder={10}>
      <lineSegments geometry={geo.lines} renderOrder={10}>
        <lineBasicMaterial vertexColors transparent opacity={opacity} depthTest={false} toneMapped={false} />
      </lineSegments>
      <points geometry={geo.points} renderOrder={11}>
        <pointsMaterial vertexColors size={9} sizeAttenuation={false} transparent opacity={opacity} depthTest={false} toneMapped={false} />
      </points>
    </group>
  );
}

/** The skinned hands; holds the render until both models are loaded and drawn. */
function Skins({ frames, opacity }: { frames: Frame3[]; opacity: number }) {
  const [scenes, setScenes] = useState<Partial<Record<Side, THREE.Group>>>({});
  const { gl, advance } = useThree();
  const hold = useRef<number | null>(null);
  useEffect(() => {
    gl.localClippingEnabled = true;
    hold.current = delayRender("hand meshes");
    Promise.all((["L", "R"] as const).map((s) => loadHand(s).then((g) => [s, g] as const)))
      .then((gs) => setScenes(Object.fromEntries(gs)))
      .catch((e) => { console.error("hand model", e); if (hold.current !== null) continueRender(hold.current); });
  }, [gl]);
  const hands = useMemo(() => frames.map((f) => (scenes[f.side] ? new Rigged(scenes[f.side]!) : null)), [scenes, frames.map((f) => f.side).join()]); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => () => hands.forEach((h) => h?.dispose()), [hands]);
  frames.forEach((f, i) => hands[i]?.pose(f, opacity));
  useEffect(() => {
    if (hold.current === null || hands.some((h) => !h)) return;
    const h = hold.current;
    hold.current = null;
    requestAnimationFrame(() => { advance(performance.now()); continueRender(h); });
  }, [hands, advance]);
  return <>{hands.map((h, i) => h && <primitive key={i} object={h.root} />)}</>;
}

export interface HandsProps {
  /** The typing session (three/hands.ts `typing`). */
  typing: Typing;
  /** Seconds into the plan (< 0: resting on the home keys). */
  t: number;
  /** 0..1: opacity of the skinned hands. */
  skin?: number;
  /** 0..1: opacity of the skeleton. */
  skeleton?: number;
}

export function Hands3D({ typing, t, skin = 1, skeleton = 0 }: HandsProps) {
  const frames = useMemo(() => typing.frames(t), [typing, t]);
  return (
    <group>
      <Skins frames={frames} opacity={skin} />
      <Skeleton frames={frames} opacity={skeleton} />
    </group>
  );
}
