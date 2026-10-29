// The product's skinned hands (the WebXR generic hand, MIT, public/hands/LICENSE.md), their 25 bones
// driven from solved joint positions. A port of web/src/components/hand/HandMesh.tsx `Rigged` to
// the promo's millimetre scene, with an opacity (the skin dissolve) and an accent tint for the
// pressing finger instead of the Hand tab's per-state colours.

import { staticFile } from "remotion";
import * as THREE from "three";
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
import { clone } from "three/examples/jsm/utils/SkeletonUtils.js";
import { C, MAT } from "../lib/tokens";
import { FINGER_NAMES, type Frame3, type Side, type V3 } from "./hands";

const loaded: Partial<Record<Side, Promise<THREE.Group>>> = {};
export const loadHand = (side: Side) =>
  (loaded[side] ??= new GLTFLoader().loadAsync(staticFile(`hands/${side === "L" ? "left" : "right"}.glb`)).then((g) => g.scene));

/** WebXR joint names per finger, thumb first (metacarpal → tip), matching `FINGER_NAMES`. */
const CHAINS = [
  ["thumb-metacarpal", "thumb-phalanx-proximal", "thumb-phalanx-distal", "thumb-tip"],
  ...["index", "middle", "ring", "pinky"].map((f) => ["metacarpal", "phalanx-proximal", "phalanx-intermediate", "phalanx-distal", "tip"].map((j) => `${f}-finger-${j}`)),
];

const SKIN = new THREE.Color(MAT.skin);
const PRESS = new THREE.Color(C.accent);
/** Forearm shown behind the wrist crease, mm. */
const FOREARM = 30;

const v3 = (p: V3) => new THREE.Vector3(p[0], p[1], p[2]);

interface Bone { bone: THREE.Bone; pos: THREE.Vector3; rot: THREE.Quaternion; scale: THREE.Vector3; dir: THREE.Vector3; axis: number; len: number; parent: number }

/** A cloned, flattened hand: every bone a direct child of `root`, posed in the parent group's coordinates. */
export class Rigged {
  root = new THREE.Group();
  bones: Bone[] = [];
  wrist = 0;
  chains: number[][];
  colors: THREE.BufferAttribute;
  /** Finger index (0–4) per vertex, −1 for the palm. */
  vfinger: Int8Array;
  restFwd: THREE.Vector3; restLat: THREE.Vector3;
  cut = new THREE.Plane();
  material: THREE.MeshStandardMaterial;

  constructor(scene: THREE.Group) {
    const copy = clone(scene);
    let mesh: THREE.SkinnedMesh | null = null;
    copy.traverse((o) => { if ((o as THREE.SkinnedMesh).isSkinnedMesh) mesh = o as THREE.SkinnedMesh; });
    if (!mesh) throw new Error("hand model has no skinned mesh");
    const m = mesh as THREE.SkinnedMesh;
    const sk = m.skeleton;
    const idx = (n: string) => sk.bones.findIndex((b) => b.name === n);
    this.wrist = idx("wrist");
    this.chains = CHAINS.map((c) => c.map(idx));
    const parentOf = new Int32Array(sk.bones.length).fill(-1);
    for (const c of this.chains) c.forEach((b, i) => { parentOf[b] = i ? c[i - 1] : this.wrist; });
    const child = new Int32Array(sk.bones.length).fill(-1);
    for (const c of this.chains) c.forEach((b, i) => { if (i < c.length - 1) child[b] = c[i + 1]; });
    this.bones = sk.bones.map((bone, i) => {
      const pos = new THREE.Vector3(), rot = new THREE.Quaternion(), scale = new THREE.Vector3();
      new THREE.Matrix4().copy(sk.boneInverses[i]).invert().decompose(pos, rot, scale);
      return { bone, pos, rot, scale, dir: new THREE.Vector3(), axis: -1, len: 0, parent: parentOf[i] };
    });
    this.bones.forEach((b, i) => {
      if (child[i] < 0) return;
      const d = this.bones[child[i]].pos.clone().sub(b.pos);
      b.len = d.length();
      b.dir.copy(d).normalize();
      const local = b.dir.clone().applyQuaternion(b.rot.clone().invert());
      const a = [Math.abs(local.x), Math.abs(local.y), Math.abs(local.z)];
      const k = a.indexOf(Math.max(...a));
      if (a[k] > 0.98) b.axis = k;
    });
    const knuckle = (f: number) => this.bones[this.chains[f][1]].pos;
    this.restFwd = knuckle(2).clone().sub(this.bones[this.wrist].pos);
    this.restLat = knuckle(4).clone().sub(knuckle(1));

    for (const b of sk.bones) { b.removeFromParent(); this.root.add(b); }
    m.removeFromParent();
    m.bindMode = THREE.AttachedBindMode;
    m.frustumCulled = false;
    m.castShadow = true;
    m.receiveShadow = true;
    this.root.add(m);

    const g = (m.geometry = m.geometry.clone());
    const n = g.attributes.position.count;
    const si = g.attributes.skinIndex, sw = g.attributes.skinWeight;
    const fingerOf = new Int8Array(sk.bones.length).fill(-1);
    this.chains.forEach((c, f) => c.slice(f ? 1 : 0).forEach((b) => { fingerOf[b] = f; }));
    this.vfinger = new Int8Array(n);
    for (let v = 0; v < n; v++) {
      let best = 0, w = -1;
      for (let k = 0; k < 4; k++) if (sw.getComponent(v, k) > w) { w = sw.getComponent(v, k); best = si.getComponent(v, k); }
      this.vfinger[v] = fingerOf[best];
    }
    this.colors = new THREE.BufferAttribute(new Float32Array(n * 3), 3);
    g.setAttribute("color", this.colors);
    this.material = new THREE.MeshStandardMaterial({ vertexColors: true, roughness: 0.6, metalness: 0, clippingPlanes: [this.cut], side: THREE.DoubleSide });
    m.material = this.material;
  }

  dispose() {
    this.root.traverse((o) => {
      const m = o as THREE.SkinnedMesh;
      if (m.isSkinnedMesh) m.geometry.dispose();
    });
    this.material.dispose();
  }

  /** Pose the bones from a frame of joint positions; `tint` 0..1 colours the pressing finger. */
  pose({ pose, states }: Frame3, opacity = 1, tint = 0.3) {
    const W = this.bones[this.wrist];
    const wrist = v3(pose.wrist);
    const k = (f: number) => v3(pose.fingers[f][0]);
    const fwd = k(2).sub(wrist), lat = k(4).sub(k(1));
    const s = 0.5 * (fwd.length() / this.restFwd.length() + lat.length() / this.restLat.length());
    const frame = (f: THREE.Vector3, l: THREE.Vector3) => {
      const z = f.clone().normalize(), y = new THREE.Vector3().crossVectors(z, l).normalize(), x = new THREE.Vector3().crossVectors(y, z);
      return new THREE.Quaternion().setFromRotationMatrix(new THREE.Matrix4().makeBasis(x, y, z));
    };
    const dw = frame(fwd, lat).multiply(frame(this.restFwd, this.restLat).invert());
    const delta: THREE.Quaternion[] = [];
    const place = (i: number, p: THREE.Vector3, d: THREE.Quaternion, len: number | null) => {
      const b = this.bones[i];
      delta[i] = d;
      b.bone.position.copy(p);
      b.bone.quaternion.copy(d).multiply(b.rot);
      b.bone.scale.copy(b.scale).multiplyScalar(s);
      if (len !== null && b.axis >= 0) b.bone.scale.setComponent(b.axis, b.scale.getComponent(b.axis) * len / b.len);
    };
    place(this.wrist, wrist, dw, null);
    const n = fwd.clone().normalize();
    this.root.updateWorldMatrix(true, false);
    this.cut.setFromNormalAndCoplanarPoint(n, wrist.clone().addScaledVector(n, -FOREARM)).applyMatrix4(this.root.matrixWorld);
    this.chains.forEach((c, f) => {
      const pts = pose.fingers[f].map(v3);
      const at = (j: number) => (f === 0 ? pts[j].clone() : j === 0 ? this.bones[c[0]].pos.clone().sub(W.pos).multiplyScalar(s).applyQuaternion(dw).add(wrist) : pts[j - 1].clone());
      c.forEach((bi, j) => {
        const b = this.bones[bi], p = at(j);
        const pd = delta[b.parent];
        if (j === c.length - 1) return place(bi, p, pd, null);
        if (f > 0 && j === 0) return place(bi, p, dw, null);
        const seg = at(j + 1).sub(p);
        const len = seg.length();
        if (f > 0) return place(bi, p, frame(seg, v3(pose.axes[f - 1])).multiply(frame(b.dir, this.restLat).invert()), len);
        const swing = new THREE.Quaternion().setFromUnitVectors(b.dir.clone().applyQuaternion(pd), seg.normalize());
        place(bi, p, swing.multiply(pd), len);
      });
    });
    const pressing = FINGER_NAMES.map((nm) => states[nm] === "press");
    const c = new THREE.Color(), arr = this.colors.array as Float32Array;
    for (let v = 0; v < this.vfinger.length; v++) {
      const f = this.vfinger[v];
      c.copy(SKIN);
      if (f > 0 && pressing[f]) c.lerp(PRESS, tint); // not the thumb: its weights spread over the palm
      arr[v * 3] = c.r; arr[v * 3 + 1] = c.g; arr[v * 3 + 2] = c.b;
    }
    this.colors.needsUpdate = true;
    this.material.opacity = opacity;
    this.material.transparent = opacity < 1;
    this.material.depthWrite = opacity > 0.5;
    this.root.visible = opacity > 0.001;
  }
}
