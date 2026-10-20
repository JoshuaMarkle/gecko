// 3D keyboard: one rounded keycap per geometry key, placed from pos/rot/size (1 unit = 1u).
// Flat and (later) curved boards share this code path. Overlays: fingers, heat, SFB arcs.

import { Canvas, useThree } from "@react-three/fiber";
import { Edges, OrbitControls, QuadraticBezierLine, RoundedBox } from "@react-three/drei";
import { useEffect, useMemo, useState } from "react";
import * as THREE from "three";
import type { Geometry, Key, Ngram } from "../api";
import { ACCENT, fingerColor, heatColor, heatText, INK } from "../colors";

export type Overlay = "fingers" | "heat" | "sfb";
export type ViewMode = "3d" | "top";

interface Props {
  geometry: Geometry;
  chars: string;
  heat: number[];
  sfb: Ngram[];
  overlay: Overlay;
  mode: ViewMode;
  selected: number | null;
  onKey: (slot: number) => void;
}

const CAP_H = 0.42;
const textures = new Map<string, THREE.CanvasTexture>();

function legend(ch: string, color: string): THREE.CanvasTexture {
  const k = `${ch}|${color}`;
  let t = textures.get(k);
  if (!t) {
    const c = document.createElement("canvas");
    c.width = c.height = 128;
    const g = c.getContext("2d")!;
    g.fillStyle = color;
    g.font = "600 76px Inter, -apple-system, Helvetica, Arial, sans-serif";
    g.textAlign = "center";
    g.textBaseline = "middle";
    g.fillText(ch === " " ? "␣" : ch.toUpperCase(), 64, 70);
    t = new THREE.CanvasTexture(c);
    t.anisotropy = 4;
    t.colorSpace = THREE.SRGBColorSpace;
    textures.set(k, t);
  }
  return t;
}

/** Board-space (x right, y toward user, z up) → three (x right, y up, z toward viewer). */
function place(k: Key, cx: number, cy: number): { p: [number, number, number]; r: [number, number, number] } {
  const d = Math.PI / 180;
  return {
    p: [k.pos[0] + k.size[0] / 2 - cx, k.pos[2], k.pos[1] + k.size[1] / 2 - cy],
    r: [k.rot[0] * d, -k.rot[2] * d, k.rot[1] * d],
  };
}

function Keycap({ k, cx, cy, ch, color, text, selected, onClick }: {
  k: Key; cx: number; cy: number; ch: string; color: string; text: string; selected: boolean; onClick: () => void;
}) {
  const [hover, setHover] = useState(false);
  const { p, r } = place(k, cx, cy);
  const w = k.size[0] - 0.08;
  const h = k.size[1] - 0.08;
  const lift = selected ? 0.12 : hover ? 0.04 : 0;
  return (
    <group position={[p[0], p[1] + CAP_H / 2 + lift, p[2]]} rotation={r}>
      <RoundedBox
        args={[w, CAP_H, h]}
        radius={0.07}
        smoothness={3}
        onClick={(e) => { e.stopPropagation(); onClick(); }}
        onPointerOver={(e) => { e.stopPropagation(); setHover(true); document.body.style.cursor = "pointer"; }}
        onPointerOut={() => { setHover(false); document.body.style.cursor = ""; }}
      >
        <meshStandardMaterial color={color} roughness={0.75} metalness={0} />
        {selected && <Edges color={ACCENT} lineWidth={2} threshold={30} />}
      </RoundedBox>
      <mesh position={[0, CAP_H / 2 + 0.002, 0]} rotation={[-Math.PI / 2, 0, 0]}>
        <planeGeometry args={[0.62, 0.62]} />
        <meshBasicMaterial map={legend(ch, text)} transparent depthWrite={false} />
      </mesh>
    </group>
  );
}

function CameraRig({ mode, w, h }: { mode: ViewMode; w: number; h: number }) {
  const { camera, controls, size } = useThree() as unknown as {
    camera: THREE.PerspectiveCamera; controls: { target: THREE.Vector3; update: () => void } | null; size: { width: number; height: number };
  };
  const aspect = size.width / Math.max(size.height, 1);
  useEffect(() => {
    // Distance that fits the board (plus margin) in both directions of the viewport.
    const v = (camera.fov * Math.PI) / 180;
    const hfov = 2 * Math.atan(Math.tan(v / 2) * aspect);
    const fitW = (w / 2 + 0.8) / Math.tan(hfov / 2);
    const fitH = (h / 2 + 0.8) / Math.tan(v / 2);
    const d = Math.max(fitW, fitH * (mode === "top" ? 1 : 1.6));
    if (mode === "top") camera.position.set(0, d, 0.001);
    else camera.position.set(0, d * 0.72, d * 0.7);
    camera.lookAt(0, 0, 0);
    if (controls) {
      controls.target.set(0, 0, 0);
      controls.update();
    }
  }, [mode, w, h, aspect, camera, controls]);
  return null;
}

export function Keyboard3D({ geometry, chars, heat, sfb, overlay, mode, selected, onKey }: Props) {
  const keys = geometry.key;
  const bounds = useMemo(() => {
    const xs = keys.flatMap((k) => [k.pos[0], k.pos[0] + k.size[0]]);
    const ys = keys.flatMap((k) => [k.pos[1], k.pos[1] + k.size[1]]);
    const [x0, x1, y0, y1] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
    return { cx: (x0 + x1) / 2, cy: (y0 + y1) / 2, w: x1 - x0, h: y1 - y0 };
  }, [keys]);
  const maxHeat = Math.max(...heat, 1e-9);
  const slotOf = useMemo(() => new Map([...chars].map((c, i) => [c, i])), [chars]);
  const arcs = overlay === "sfb" ? sfb.slice(0, 12) : [];
  const maxArc = Math.max(...arcs.map((a) => a.freq), 1e-9);

  return (
    <Canvas camera={{ fov: 32, near: 0.1, far: 200 }} dpr={[1, 2]} onPointerMissed={() => onKey(-1)}>
      <color attach="background" args={["#f3f3f3"]} />
      <ambientLight intensity={1.6} />
      <directionalLight position={[4, 10, 6]} intensity={1.4} />
      <directionalLight position={[-6, 6, -4]} intensity={0.4} />
      <CameraRig mode={mode} w={bounds.w} h={bounds.h} />
      <RoundedBox args={[bounds.w + 0.6, 0.3, bounds.h + 0.6]} radius={0.12} position={[0, -0.15, 0]}>
        <meshStandardMaterial color="#e6e6e6" roughness={0.9} />
      </RoundedBox>
      {keys.map((k, i) => {
        const ch = chars[i] ?? "";
        const t = heat[i] / maxHeat;
        const color = overlay === "heat" ? heatColor(t) : fingerColor(k.finger);
        const text = overlay === "heat" ? heatText(t) : INK;
        return (
          <Keycap key={i} k={k} cx={bounds.cx} cy={bounds.cy} ch={ch} color={color} text={text} selected={selected === i} onClick={() => onKey(i)} />
        );
      })}
      {arcs.map((a) => {
        const [i, j] = [slotOf.get(a.ngram[0]), slotOf.get(a.ngram[1])];
        if (i === undefined || j === undefined) return null;
        const pi = place(keys[i], bounds.cx, bounds.cy).p;
        const pj = place(keys[j], bounds.cx, bounds.cy).p;
        const top = CAP_H + 0.05;
        const dist = Math.hypot(pi[0] - pj[0], pi[2] - pj[2]);
        return (
          <QuadraticBezierLine
            key={a.ngram}
            start={[pi[0], top, pi[2]]}
            end={[pj[0], top, pj[2]]}
            mid={[(pi[0] + pj[0]) / 2, top + 0.35 + dist * 0.35, (pi[2] + pj[2]) / 2]}
            color={INK}
            lineWidth={1 + 5 * (a.freq / maxArc)}
            transparent
            opacity={0.85}
          />
        );
      })}
      <OrbitControls makeDefault enablePan enableDamping dampingFactor={0.12} maxPolarAngle={Math.PI / 2.05} minDistance={3} maxDistance={40} />
    </Canvas>
  );
}
