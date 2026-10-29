// Keyframed camera for the 3D scene: position, target, fov and an orthographic blend.
// `ortho` 1 narrows the fov toward 1° while pulling back so the target plane keeps its framing
// (a dolly zoom): the board flattens into the product's 2D drawing without a cut.

import { useThree } from "@react-three/fiber";
import { useLayoutEffect } from "react";
import { useCurrentFrame } from "remotion";
import * as THREE from "three";
import { EASE, lerp } from "../lib/motion";

export interface Shot {
  /** Frame (relative to the sequence) this shot is reached. */
  f: number;
  pos: [number, number, number];
  target: [number, number, number];
  fov?: number;
  ortho?: number;
  /** Roll about the view axis (degrees). */
  roll?: number;
  /** Easing into this shot from the previous one. */
  ease?: (t: number) => number;
}

const ORTHO_FOV = 1.2;

/** Camera state at `frame` along `path` (sorted by f). */
export function sample(path: Shot[], frame: number) {
  let a = path[0], b = path[0], t = 0;
  if (frame >= path[path.length - 1].f) a = b = path[path.length - 1];
  else for (let i = 1; i < path.length; i++) {
    if (frame < path[i].f) {
      a = path[i - 1]; b = path[i];
      t = (b.ease ?? EASE.inOut)((frame - a.f) / Math.max(1e-6, b.f - a.f));
      break;
    }
  }
  const v = (p: [number, number, number], q: [number, number, number]) => new THREE.Vector3(lerp(p[0], q[0], t), lerp(p[1], q[1], t), lerp(p[2], q[2], t));
  return {
    pos: v(a.pos, b.pos), target: v(a.target, b.target),
    fov: lerp(a.fov ?? 30, b.fov ?? 30, t), ortho: lerp(a.ortho ?? 0, b.ortho ?? 0, t), roll: lerp(a.roll ?? 0, b.roll ?? 0, t),
  };
}

/** Keyframes every other frame along a parametric move (`at` gets eased 0..1), linear between them. */
export function along(f0: number, f1: number, at: (t: number) => Omit<Shot, "f">, ease = EASE.linear): Shot[] {
  const out: Shot[] = [];
  for (let f = f0; f <= f1; f += 2) out.push({ f, ...at(ease((f - f0) / (f1 - f0))), ease: EASE.linear });
  return out;
}

/** Point `cam` along a sampled shot (dolly zoom for `ortho`); returns the target distance. */
function aim(cam: THREE.PerspectiveCamera, s: ReturnType<typeof sample>): number {
  // Dolly zoom: keep tan(fov/2) · distance constant while the fov narrows.
  const fov = THREE.MathUtils.lerp(s.fov, ORTHO_FOV, Math.pow(s.ortho, 0.6));
  const k = Math.tan(THREE.MathUtils.degToRad(s.fov) / 2) / Math.tan(THREE.MathUtils.degToRad(fov) / 2);
  const dir = s.pos.clone().sub(s.target);
  cam.position.copy(s.target).add(dir.multiplyScalar(k));
  cam.fov = fov;
  cam.near = Math.max(1, dir.length() * 0.05);
  cam.far = dir.length() * 4 + 8000;
  cam.up.set(0, 1, 0);
  if (Math.abs(dir.clone().normalize().y) > 0.999) cam.up.set(0, 0, -1); // straight down: keep the board's top up
  cam.lookAt(s.target);
  if (s.roll) cam.rotateZ(THREE.MathUtils.degToRad(s.roll));
  cam.updateProjectionMatrix();
  return dir.length();
}

/** Screen position (px, frame of `w` × `h`) of world point `p` at `frame` of `path`: anchors for 2D callouts. */
export function project(path: Shot[], frame: number, p: [number, number, number] | THREE.Vector3, w = 1920, h = 1080): [number, number] {
  const cam = new THREE.PerspectiveCamera(30, w / h, 1, 8000);
  aim(cam, sample(path, frame));
  cam.updateMatrixWorld();
  const v = (Array.isArray(p) ? new THREE.Vector3(...p) : p.clone()).project(cam);
  return [(v.x + 1) / 2 * w, (1 - v.y) / 2 * h];
}

export function CameraRig({ path, offset = 0 }: { path: Shot[]; offset?: number }) {
  const frame = useCurrentFrame() - offset;
  const { camera, scene } = useThree();
  useLayoutEffect(() => {
    const d = aim(camera as THREE.PerspectiveCamera, sample(path, frame));
    // Fog is specified around the target (Scene3D `fog`), so it survives the dolly zoom.
    const fog = scene.fog as THREE.Fog | null, around = scene.userData.fogAround as [number, number] | undefined;
    if (fog && around) { fog.near = Math.max(0, d + around[0]); fog.far = d + around[1]; }
  }, [frame, path, camera, scene]);
  return null;
}
