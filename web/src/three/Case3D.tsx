// 3D case (Export tab): the plate, walls and bottom from gecko-wasm `CaseBuild` (the STL files'
// triangles), with the board's switches and keycaps on top. Scene units are key units like
// Keyboard3D, centred on the key bounds (`boardBounds`, `place`), so the caps line up with the plate.

import { Canvas, useThree } from "@react-three/fiber";
import { Grid, OrbitControls } from "@react-three/drei";
import { useEffect, useMemo } from "react";
import * as THREE from "three";
import type { Geometry } from "../api";
import { INK } from "../colors";
import type { CaseModel, CasePart } from "../wasm";
import { boardBounds, CAP_H, capGeometry, keyTop, legend, place } from "./Keyboard3D";

const U_MM = 19.05;
const COLOR: Record<CasePart, string> = { plate: "#d6d6d6", walls: "#d6d6d6", bottom: "#a9a9a9" };
/** Keycaps that hit the case within their travel (`report.cap_hits`). */
const HIT = "#f2a0a0";
const SWITCH = "#5a5a5a";
const EMPTY = "·";

/** STL millimetres (z up, y = −board y) → scene (key units, y up, board-centred). */
function sceneGeometry(tri: Float32Array, cx: number, cy: number): THREE.BufferGeometry {
  const p = new Float32Array(tri.length);
  for (let i = 0; i < tri.length; i += 3) {
    p[i] = tri[i] / U_MM - cx;
    p[i + 1] = tri[i + 2] / U_MM;
    p[i + 2] = -tri[i + 1] / U_MM - cy;
  }
  const g = new THREE.BufferGeometry();
  g.setAttribute("position", new THREE.BufferAttribute(p, 3));
  g.computeVertexNormals();
  g.computeBoundingBox();
  return g;
}

/** Orbit camera that frames `box` from the front, above. */
function CameraRig({ box }: { box: THREE.Box3 }) {
  const { camera, controls, size } = useThree() as unknown as {
    camera: THREE.PerspectiveCamera; controls: { target: THREE.Vector3; update: () => void } | null; size: { width: number; height: number };
  };
  const aspect = size.width / Math.max(size.height, 1);
  useEffect(() => {
    // Viewed from about 45° above, the case's depth and height each show at ~0.7 of their size;
    // fit that and its width (plus a margin, and room for the nearer front edge's perspective) in the viewport.
    const c = box.getCenter(new THREE.Vector3());
    const s = box.getSize(new THREE.Vector3());
    const v = (camera.fov * Math.PI) / 180;
    const hfov = 2 * Math.atan(Math.tan(v / 2) * aspect);
    const d = 1.3 * Math.max((s.x / 2 + 0.4) / Math.tan(hfov / 2), (0.36 * (s.z + s.y) + 0.4) / Math.tan(v / 2));
    camera.position.set(c.x, c.y + d * 0.72, c.z + d * 0.7);
    camera.lookAt(c);
    if (controls) {
      controls.target.copy(c);
      controls.update();
    }
  }, [box, aspect, camera, controls]);
  return null;
}

export function Case3D({ geometry, chars, model, caps, bottom }: { geometry: Geometry; chars: string; model: CaseModel; caps: boolean; bottom: boolean }) {
  const keys = geometry.key;
  const { cx, cy } = useMemo(() => boardBounds(keys), [keys]);
  const parts = useMemo(() => (Object.keys(model.mesh) as CasePart[]).map((k) => [k, sceneGeometry(model.mesh[k], cx, cy)] as const), [model, cx, cy]);
  useEffect(() => () => parts.forEach(([, g]) => g.dispose()), [parts]);
  const box = useMemo(() => {
    const b = new THREE.Box3();
    parts.forEach(([, g]) => g.boundingBox && b.union(g.boundingBox));
    return b;
  }, [parts]);
  const lift = model.report.lift / U_MM;
  const gap = model.params.cap_gap / U_MM;
  const hole = model.params.hole / U_MM;
  const hits = new Set(model.report.cap_hits);
  const sym = [...chars];

  return (
    <Canvas camera={{ fov: 32, near: 0.05, far: 300 }} dpr={[1, 2]}>
      <color attach="background" args={["#f3f3f3"]} />
      <ambientLight intensity={1.6} />
      <directionalLight position={[3, 10, 6]} intensity={3.2} />
      <directionalLight position={[-6, 4, -4]} intensity={0.8} />
      <Grid position={[0, box.min.y - 0.001, 0]} infiniteGrid cellSize={1} sectionSize={5} cellThickness={0.6} sectionThickness={1} cellColor="#dcdcdc" sectionColor="#c4c4c4" fadeDistance={60} fadeStrength={1.5} />
      <CameraRig box={box} />
      {parts.map(([k, g]) => (k !== "bottom" || bottom) && (
        <mesh key={k} geometry={g}>
          <meshStandardMaterial color={COLOR[k]} roughness={0.85} side={THREE.DoubleSide} />
        </mesh>
      ))}
      {caps && keys.map((k, i) => {
        const { p, r } = place(k, cx, cy);
        const n = keyTop(k, cx, cy).n;
        // Keycap centre, and the switch housing between the cap bottom and the plate (along the key normal).
        const mid: [number, number, number] = [p[0], p[1] + lift + CAP_H / 2, p[2]];
        const sw = (CAP_H + gap) / 2;
        const ch = sym[i] && sym[i] !== EMPTY ? sym[i] : "";
        return (
          <group key={i}>
            <group position={mid} rotation={r}>
              <mesh geometry={capGeometry(k.size[0] - 0.08, k.size[1] - 0.08)}>
                <meshStandardMaterial color={hits.has(i) ? HIT : "#ffffff"} roughness={0.75} />
              </mesh>
              {ch && (
                <mesh position={[0, CAP_H / 2 + 0.002, 0]} rotation={[-Math.PI / 2, 0, 0]}>
                  <planeGeometry args={[0.6, 0.6]} />
                  <meshBasicMaterial map={legend(ch, INK)} transparent depthWrite={false} />
                </mesh>
              )}
            </group>
            <mesh position={[mid[0] - n[0] * sw, mid[1] - n[1] * sw, mid[2] - n[2] * sw]} rotation={r}>
              <boxGeometry args={[hole, gap, hole]} />
              <meshStandardMaterial color={SWITCH} roughness={0.6} />
            </mesh>
          </group>
        );
      })}
      <OrbitControls makeDefault enableDamping dampingFactor={0.12} minDistance={1} maxDistance={80} />
    </Canvas>
  );
}
