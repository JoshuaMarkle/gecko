// The board run's real STL files (`gecko build stl`: plate, case = plate + walls, bottom), placed in
// the Corne3D scene. STL space is board millimetres with y mirrored (STL y = −board y), z up, the
// board lifted so its lowest switch sits `clearance` above the floor (build.json report.lift).
// `explode` 0..1 pulls the parts apart vertically; `rise` 0..1 brings them up from below the floor.

import { useThree } from "@react-three/fiber";
import { useEffect, useRef, useState } from "react";
import { continueRender, delayRender, staticFile } from "remotion";
import * as THREE from "three";
import { STLLoader } from "three/examples/jsm/loaders/STLLoader.js";
import { data } from "../lib/data";
import { U } from "../lib/tokens";
import { BOARD_CENTRE } from "./Corne3D";
import { matte } from "./materials";

export type StlPart = "plate" | "case" | "bottom";
const cache = new Map<string, Promise<THREE.BufferGeometry>>();

function load(part: StlPart): Promise<THREE.BufferGeometry> {
  let p = cache.get(part);
  if (!p) {
    p = fetch(staticFile(data.stl[part].file)).then((r) => r.arrayBuffer()).then((buf) => {
      const g = new STLLoader().parse(buf);
      g.rotateX(-Math.PI / 2); // (x, y, z) → (x, z, −y): z up → three y up, −STL y = board y → three z
      g.computeVertexNormals();
      return g;
    });
    cache.set(part, p);
  }
  return p;
}

/** STL scene lift (mm) vs Corne3D: Corne3D `lift` that puts its keycap bottoms where the STL expects them. */
export const STL_KEY_LIFT = (data.stl.build.report as { lift: number }).lift - 16.7;

export function STLModel({ parts = ["bottom", "case"], explode = 0, rise = 1, opacity = 1, color = "#fbfbfa" }: {
  parts?: StlPart[]; explode?: number; rise?: number; opacity?: number; color?: string;
}) {
  const [geo, setGeo] = useState<Partial<Record<StlPart, THREE.BufferGeometry>>>({});
  const { advance } = useThree();
  const hold = useRef<number | null>(null);
  useEffect(() => {
    hold.current = delayRender("stl");
    Promise.all(parts.map((p) => load(p).then((g) => [p, g] as const)))
      .then((gs) => setGeo(Object.fromEntries(gs)))
      .catch((e) => { console.error(e); if (hold.current !== null) continueRender(hold.current); });
  }, [parts.join(",")]); // eslint-disable-line react-hooks/exhaustive-deps
  // ThreeCanvas only advances on frame changes: draw once the meshes are mounted, then release.
  useEffect(() => {
    if (hold.current === null || Object.keys(geo).length === 0) return;
    const h = hold.current;
    hold.current = null;
    requestAnimationFrame(() => { advance(performance.now()); continueRender(h); });
  }, [geo, advance]);
  const offset: Record<StlPart, number> = { bottom: -40, case: -10, plate: 20 };
  return (
    <group position={[-BOARD_CENTRE.x * U, -(1 - rise) * 220, -BOARD_CENTRE.y * U]}>
      {parts.map((p) => geo[p] && (
        <mesh key={p} geometry={geo[p]} position={[0, offset[p] * explode, 0]} material={matte(color, 0.55, opacity)} castShadow receiveShadow />
      ))}
    </group>
  );
}
