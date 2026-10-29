// One MX switch and its keycap, origin at the switch plate's top surface (mm, y up).
// `press` 0..1 sinks the cap and stem by TRAVEL; `explode` lifts the cap and switch apart. No LED light (no underglow).

import { useMemo } from "react";
import * as THREE from "three";
import { toCreasedNormals } from "three/examples/jsm/utils/BufferGeometryUtils.js";
import { MAT } from "../lib/tokens";
import { CAP, capGeometry, roundRect, STACK, TRAVEL } from "./geometry";
import { legendTexture } from "./legend";
import { housingTop, legendMat, matte, metal, plastic } from "./materials";

/** Switch heights above the plate top (mm): housing top, cap base (the cap's skirt hides the housing's top), bottom housing down to the PCB. */
const HOUSING_H = 6.0;
const CAP_BASE = 3.6;
const BOTTOM = STACK.pcb1 - STACK.plate1; // −6.5: bottom housing sits on the PCB

/** Lofted rounded-rectangle frustum: (w0 × d0, r0) at y0 → (w1 × d1, r1) at y1, closed top. */
function frustum(w0: number, d0: number, r0: number, w1: number, d1: number, r1: number, y0: number, y1: number): THREE.BufferGeometry {
  const a = roundRect(w0, d0, r0, 3), b = roundRect(w1, d1, r1, 3), M = a.length;
  const pos: number[] = [], idx: number[] = [];
  a.forEach(([x, z]) => pos.push(x, y0, z));
  b.forEach(([x, z]) => pos.push(x, y1, z));
  for (let i = 0; i < M; i++) { const j = (i + 1) % M; idx.push(i, M + i, M + j, i, M + j, j); }
  const c = pos.length / 3; pos.push(0, y1, 0);
  for (let i = 0; i < M; i++) idx.push(c, M + (i + 1) % M, M + i);
  const g = new THREE.BufferGeometry();
  g.setAttribute("position", new THREE.Float32BufferAttribute(pos, 3));
  g.setIndex(idx);
  return toCreasedNormals(g, Math.PI / 5);
}

let parts: ReturnType<typeof makeParts> | null = null;
function makeParts() {
  return {
    top: frustum(15.4, 15.4, 1.0, 11.4, 10.6, 1.4, 0, HOUSING_H),
    rim: frustum(11.4, 10.6, 1.4, 10.6, 9.8, 1.2, HOUSING_H, HOUSING_H + 0.35),
    bottom: new THREE.BoxGeometry(13.9, -BOTTOM, 13.9).translate(0, BOTTOM / 2, 0),
    flange: new THREE.BoxGeometry(15.6, 0.8, 15.6).translate(0, -0.4 - 1.5, 0),
    led: new THREE.BoxGeometry(5.4, 0.2, 2.6).translate(0, HOUSING_H + 0.36, -3.4),
    stemX: new THREE.BoxGeometry(4.1, 3.8, 1.2),
    stemZ: new THREE.BoxGeometry(1.2, 3.8, 4.1),
    stemBase: new THREE.CylinderGeometry(2.8, 2.8, 0.8, 24),
    post: new THREE.CylinderGeometry(2, 2, 3.2, 20).translate(0, BOTTOM - 1.6, 0),
    pin: new THREE.CylinderGeometry(0.75, 0.75, 3.4, 10).translate(0, BOTTOM - 1.7, 0),
    homing: new THREE.CapsuleGeometry(0.45, 4.4, 4, 10).rotateZ(Math.PI / 2),
  };
}

export interface KeyProps {
  /** Footprint in u (1 u = 19.05 mm). */
  size: [number, number];
  legend: string;
  /** Cap colour (white, a finger colour, or the accent). */
  color: string;
  /** Legend glyph colour. */
  ink: string;
  press?: number;
  /** 0..1: the cap rises off the switch and the switch rises out of the plate. */
  explode?: number;
  /** Homing bar on the cap (F and J positions). */
  homing?: boolean;
  /** Draw the switch (off for thumbnails or far shots). */
  switchOn?: boolean;
  opacity?: number;
}

export function Key({ size, legend, color, ink, press = 0, explode = 0, homing = false, switchOn = true, opacity = 1 }: KeyProps) {
  parts ??= makeParts();
  const p = parts;
  const w = 18.0 + (size[0] - 1) * 19.05, d = 18.0 + (size[1] - 1) * 19.05;
  const cap = capGeometry(w, d);
  const { tw, td, off } = cap.userData;
  const top = useMemo(() => legendMat(legendTexture(legend, tw / td, color, ink), opacity), [legend, tw, td, color, ink, opacity]);
  const wall = plastic(color, opacity);
  const sink = -TRAVEL * press;
  const capLift = explode * 26, swLift = explode * 12;
  const housing = matte(MAT.housing, 0.55, opacity);
  return (
    <group>
      {switchOn && (
        <group position={[0, swLift, 0]}>
          <mesh geometry={p.top} material={housingTop()} castShadow receiveShadow />
          <mesh geometry={p.rim} material={housing} />
          <mesh geometry={p.led} material={matte(MAT.led, 0.3, opacity)} />
          <mesh geometry={p.bottom} material={matte(MAT.housingBottom, 0.7, opacity)} />
          <mesh geometry={p.post} material={housing} />
          <mesh geometry={p.pin} position={[-3.81, 0, -2.54]} material={metal("#c8b98a", 0.35)} />
          <mesh geometry={p.pin} position={[2.54, 0, -5.08]} material={metal("#c8b98a", 0.35)} />
          <group position={[0, HOUSING_H + 1.4 + sink, 0]}>
            <mesh geometry={p.stemBase} position={[0, -1.2, 0]} material={matte(MAT.stem, 0.5, opacity)} />
            <mesh geometry={p.stemX} material={matte(MAT.stem, 0.5, opacity)} />
            <mesh geometry={p.stemZ} material={matte(MAT.stem, 0.5, opacity)} />
          </group>
        </group>
      )}
      <group position={[0, CAP_BASE + sink + capLift, 0]}>
        <mesh geometry={cap} material={[wall, top]} castShadow receiveShadow />
        {homing && <mesh geometry={p.homing} position={[0, CAP.h - CAP.dish + 0.25, off + td * 0.3]} material={wall} />}
      </group>
    </group>
  );
}

/** Height of a keycap's top centre above the plate top (mm), for cameras and hands. */
export const CAP_TOP = CAP_BASE + CAP.h - CAP.dish;
