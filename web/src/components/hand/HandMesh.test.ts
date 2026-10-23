import { describe, expect, it } from "vitest";
import * as THREE from "three";
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
import { AVERAGE, FINGER_NAMES, FOUR, handLayout, naturalTip, poseHand, toWorld, type FingerName, type Side, type V3 } from "../../hand";
import { padLift, Rigged } from "./HandMesh";
import { MM } from "./rigs";
import left from "./assets/left.glb?inline";
import right from "./assets/right.glb?inline";

const scene = async (side: Side) => (await new GLTFLoader().parseAsync(await (await fetch(side === "L" ? left : right)).arrayBuffer(), "")).scene;

/** A hand at the origin, fingertips at their natural typing curl (key tops), thumb at its rest point; targets lifted for the mesh. */
function frame(side: Side) {
  const lift = padLift(AVERAGE);
  const pl = { wrist: [0, 3, 0] as V3, yaw: 0.17 * (side === "L" ? -1 : 1) };
  const lay = handLayout(AVERAGE);
  const targets = {} as Record<FingerName, V3>;
  FOUR.forEach((f, i) => {
    const k = lay.knuckles[i], [fwd, drop] = naturalTip(AVERAGE[f]);
    targets[f] = toWorld(side, pl, MM, [k[0], k[1] - drop, k[2] - fwd]);
  });
  targets.thumb = toWorld(side, pl, MM, lay.thumbRest);
  const keyTops = FINGER_NAMES.map((f) => targets[f]);
  for (const f of FINGER_NAMES) targets[f] = [targets[f][0], targets[f][1] + lift[f], targets[f][2]];
  const pose = poseHand(side, pl, targets, AVERAGE, MM);
  return { keyTops, pose, states: Object.fromEntries(FINGER_NAMES.map((f) => [f, "rest"])) as Record<FingerName, "rest"> };
}

describe("skinned hand", () => {
  for (const side of ["L", "R"] as const) it(`${side}: skin follows the posed joints`, async () => {
    const h = new Rigged(await scene(side));
    const fr = frame(side);
    h.pose(fr);
    h.root.updateMatrixWorld(true);
    let mesh: THREE.SkinnedMesh | null = null;
    h.root.traverse((o) => { if ((o as THREE.SkinnedMesh).isSkinnedMesh) mesh = o as THREE.SkinnedMesh; });
    const m = mesh!;
    m.skeleton.update();
    const pos = m.geometry.attributes.position, v = new THREE.Vector3();
    fr.pose.fingers.forEach((_, f) => {
      const tip = new THREE.Vector3(...fr.keyTops[f]);
      // The finger's skin meets its posed fingertip; the pad sits on it, not through the key.
      let near = Infinity, low = Infinity;
      for (let i = 0; i < pos.count; i++) {
        if (h.vfinger[i] !== f) continue;
        m.getVertexPosition(i, v);
        near = Math.min(near, v.distanceTo(tip));
        if (v.distanceTo(tip) < 15 * MM) low = Math.min(low, v.y);
      }
      expect(near / MM, `${side} ${FINGER_NAMES[f]} skin to key top`).toBeLessThan(6);
      expect(Math.abs(low - tip.y) / MM, `${side} ${FINGER_NAMES[f]} pad depth`).toBeLessThan(4);
    });
  });
});
