// The physical Corne 3x6+3 (data/geometries/corne-42.param.toml): both halves with bottom plate, PCB
// (silkscreen, pads, traces), switch plate, standoffs, nice!nano, TRRS, reset, MX switches and dished
// caps. Keys are placed from GECKO key poses (the flat board or any board-run snapshot); the plate
// stack is built from the flat board and fades out (`stack`) when the keys leave it.
// Units: mm. Scene origin = centre of the flat board on the floor.

import { useMemo } from "react";
import * as THREE from "three";
import { data, symbols, type Pose } from "../lib/data";
import { C, fingerColor, fingerInk, KEY, MAT, U } from "../lib/tokens";
import { convexHull, roundBox, slab, STACK, unionOutline, unionPolygons, type Rect } from "./geometry";
import { faceTexture } from "./legend";
import { matte, metal, plateMat } from "./materials";
import { NANO, NiceNano, ResetButton, TRRS } from "./NiceNano";
import { Key } from "./Switch";

const KEYS = data.board.keys;
const FLAT: Pose[] = KEYS.map((k) => k.pose);
/** Board centre (u) of the flat drawing; the scene is centred on it. */
const BX = (Math.min(...KEYS.map((k) => k.pose[0])) + Math.max(...KEYS.map((k) => k.pose[0] + k.size[0]))) / 2;
const BY = (Math.min(...KEYS.map((k) => k.pose[1])) + Math.max(...KEYS.map((k) => k.pose[1] + k.size[1]))) / 2;
/** Case margin around the keys (u), the microcontroller column (u, left half; mirrored for the right). */
const P = 0.12;
const MCU = { x0: 6.0, x1: 7.15, y0: 0.2 - P, y1: 3.2 };
const MIRROR = 15; // x' = MIRROR − x maps the left half onto the right (u)
/** Standoff posts between keys (u, left half). */
const POSTS: [number, number][] = [[1, 1.3], [4, 1.1], [1, 2.3], [4, 2.1], [3.5, 3.15], [5.02, 3.3]];
/** Home-row index keys (homing bars). */
const HOMING = KEYS.map((k) => k.row === 1 && ((k.finger === "LI" && k.col === 4) || (k.finger === "RI" && k.col === 7)));

/** Outer-column (cols 0 and 11, rows 0–2) and thumb keys: grey caps. */
const isEnd = (k: { col: number; row: number }) => k.row === 3 || k.col === 0 || k.col === 11;

export const isLeft = (i: number) => KEYS[i].finger[0] === "L";
const mm = (u: number) => u * U;

export interface CorneProps {
  /** One symbol per key (42), e.g. data.layout.start or a trace snapshot. */
  chars?: string;
  /** Key poses (default: the flat board). */
  poses?: Pose[];
  /** Key fill: "white", "fingers", or one colour per key. */
  fill?: "white" | "fingers" | string[];
  /** Press depth per key, 0..1. */
  press?: number[];
  /** Key index drawn in the accent (the one blue thing). */
  accent?: number | null;
  /** 0..1: layers separate vertically (bottom, PCB, plate, switches, caps). */
  explode?: number;
  /** 0..1: opacity of the case stack (bottom, PCB, plate, MCU). 0 = keys only. */
  stack?: number;
  /** Extra distance between the halves (mm). */
  spread?: number;
  /** Lift added to every key (mm), e.g. to keep a tented board above the floor. */
  lift?: number;
  /** 0..1: squash toward the floor (the 3D → 2D flatten). */
  flatten?: number;
  /** Draw switches (off for far shots). */
  switches?: boolean;
  /** Legends on? (off = blank caps) */
  legends?: boolean;
  /** 0..1: a plate tile under every key (boards whose keys have left the flat stack). */
  tiles?: number;
}

/**
 * Thumb cluster of the flat board, outside in (left half, key centres in u; the right half mirrors it), as in
 * ../kinetic/keyboard: a higher key, a lower one, and the big inner key lower still, turned `rot` degrees
 * clockwise seen from above. Only the flat pose gets it (board-run poses carry their own).
 */
const THUMBS = [
  { x: 4.5, y: 3.9, rot: 0 },
  { x: 5.55, y: 4.0, rot: 0 },
  { x: 6.95, y: 4.0, rot: 25 },
];
const thumbOf = (i: number, p: Pose) => (KEYS[i].row === 3 && p.every((v, j) => v === FLAT[i][j]) ? THUMBS[isLeft(i) ? KEYS[i].col : 5 - KEYS[i].col] : null);

/** Position (mm) and rotation (rad) of key i at pose p, as in web/src/three/Keyboard3D.tsx `place`. */
export function keyTransform(i: number, p: Pose, spread = 0, lift = 0): { pos: THREE.Vector3; rot: THREE.Euler } {
  const k = KEYS[i];
  const d = Math.PI / 180;
  const left = isLeft(i);
  const side = left ? -1 : 1;
  const t = thumbOf(i, p);
  const cx = t ? (left ? t.x : MIRROR - t.x) : p[0] + k.size[0] / 2, cy = t ? t.y : p[1] + k.size[1] / 2;
  const a = t ? (left ? -1 : 1) * t.rot * d : 0;
  return {
    pos: new THREE.Vector3(mm(cx - BX) + side * spread / 2, STACK.plate1 + mm(p[2]) + lift, mm(cy - BY)),
    rot: new THREE.Euler(p[3] * d, -p[5] * d + a, p[4] * d),
  };
}

/** Per-half plan geometry (mm, board-centred, before spread): plate and PCB outlines, MCU anchor. */
function halfPlan(left: boolean) {
  const idx = KEYS.map((_, i) => i).filter((i) => isLeft(i) === left);
  const fx = (x: number) => (left ? x : MIRROR - x);
  const rect = (x0: number, y0: number, x1: number, y1: number): Rect => {
    const [a, b] = [fx(x0), fx(x1)].sort((p, q) => p - q);
    return { x0: mm(a - BX), x1: mm(b - BX), z0: mm(y0 - BY), z1: mm(y1 - BY) };
  };
  const keyRects = idx.filter((i) => KEYS[i].row !== 3).map((i) => {
    const k = KEYS[i];
    const [x, y] = k.pose;
    const lx = left ? x : MIRROR - x - k.size[0];
    return rect(lx - P, y - P, lx + k.size[0] + P, y + k.size[1] + P);
  });
  // Under the middle column: board down to the pinky columns' bottom edge, out to the thumb cluster, so the
  // post under it sits on the board.
  keyRects.push(rect(3 - P, 3.0, 4 - P, 3.3 + P));
  // Thumb cluster: each key's rectangle (turned as the key is), so the bottom edge steps around the keys,
  // plus a convex filler from the keys' tops (and down between the second key and the big one) up to the
  // finger rows and the MCU column; one piece with the rest of the half.
  const toPlan = (x: number, y: number) => [mm(fx(x) - BX), mm(y - BY)] as [number, number];
  const turned = (t: (typeof THUMBS)[number], w: number, h: number) => {
    const r = (t.rot * Math.PI) / 180, [sn, cs] = [Math.sin(r), Math.cos(r)];
    return [[-w, -h], [w, -h], [w, h], [-w, h]].map(([x, y]) => toPlan(t.x + x * cs - y * sn, t.y + x * sn + y * cs));
  };
  const thumbKeys = THUMBS.map((t, n) => turned(t, 0.5 + P, (n === 2 ? 0.75 : 0.5) + P));
  const [t0, t1] = THUMBS;
  const filler = convexHull([
    ...thumbKeys[2].slice(0, 2), toPlan(t0.x - 0.5 - P, t0.y - 0.5 - P), toPlan(t1.x + 0.5 + P, t1.y + 0.5),
    toPlan(t0.x - 0.5 - P, 3.0), toPlan(MCU.x0, 3.0), toPlan(MCU.x1, MCU.y1),
  ]);
  const keys = unionOutline(keyRects), m = rect(MCU.x0, MCU.y0, MCU.x1, MCU.y1);
  const mcu: [number, number][] = [[m.x0, m.z0], [m.x1, m.z0], [m.x1, m.z1], [m.x0, m.z1]];
  return {
    idx,
    plate: [unionPolygons([keys, filler, ...thumbKeys])],
    pcb: [unionPolygons([keys, mcu, filler, ...thumbKeys])],
    mcu: { x: mm(fx((MCU.x0 + MCU.x1) / 2 + 0.005) - BX), z: mm(0.3 - BY) },
    glass: rect(MCU.x0 + 0.02, MCU.y0, MCU.x1 - 0.02, 2.28), // nano spans y 0.3–2.03 u; the standoffs sit past its ends
    jack: { x: mm(fx(MCU.x1) - BX) + (left ? -6.5 : 6.5), z: mm(2.42 - BY) },
    reset: { x: mm(fx(6.45) - BX), z: mm(2.86 - BY) },
    posts: POSTS.map(([x, y]) => [mm(fx(x) - BX), mm(y - BY)] as [number, number]),
  };
}

/** PCB top texture over the outline's bounding box: silkscreen footprints, pads, diodes, traces, logo. */
function pcbTexture(left: boolean, outline: [number, number][], keys: number[]) {
  const xs = outline.map((p) => p[0]), zs = outline.map((p) => p[1]);
  const x0 = Math.min(...xs), x1 = Math.max(...xs), z0 = Math.min(...zs), z1 = Math.max(...zs);
  const w = x1 - x0, l = z1 - z0;
  const t = faceTexture(`pcb-${left}`, w, l, MAT.pcb, (g) => {
    g.translate(-w / 2 - x0, -z0); // plan mm coordinates from here on
    const tf = keys.map((i) => keyTransform(i, FLAT[i]));
    const centres = tf.map((t) => t.pos);
    const turns = tf.map((t) => -t.rot.y); // canvas turn = −(scene y rotation)
    // Traces: rows (horizontal) and columns, a shade lighter under the soldermask.
    g.strokeStyle = MAT.pcbTrace; g.lineWidth = 0.35; g.lineCap = "round";
    for (const [n, cc] of centres.entries()) {
      g.save(); g.translate(cc.x, cc.z); g.rotate(turns[n]); const c = { x: 0, z: 0 };
      g.beginPath(); g.moveTo(c.x - 3.8, c.z + 2.5); g.lineTo(c.x - 3.8, c.z + 7.5); g.lineTo(c.x + 6, c.z + 7.5); g.lineTo(c.x + 9.5, c.z + 4); g.stroke();
      g.beginPath(); g.moveTo(c.x + 2.5, c.z - 5.1); g.lineTo(c.x + 5.5, c.z - 8.1); g.lineTo(c.x + 5.5, c.z - 9.5); g.stroke();
      g.restore();
    }
    // Switch footprints: silkscreen squares, centre hole, two plated pads, LED window, diode.
    for (const [n, cc] of centres.entries()) {
      g.save(); g.translate(cc.x, cc.z); g.rotate(turns[n]); const c = { x: 0, z: 0 };
      g.strokeStyle = MAT.pcbSilk; g.lineWidth = 0.15;
      g.strokeRect(c.x - 7, c.z - 7, 14, 14);
      g.fillStyle = "#050507"; g.beginPath(); g.arc(c.x, c.z, 2.1, 0, Math.PI * 2); g.fill();
      for (const [dx, dz] of [[-3.81, -2.54], [2.54, -5.08]]) {
        g.fillStyle = MAT.copper; g.beginPath(); g.arc(c.x + dx, c.z + dz, 1.25, 0, Math.PI * 2); g.fill();
        g.fillStyle = "#050507"; g.beginPath(); g.arc(c.x + dx, c.z + dz, 0.75, 0, Math.PI * 2); g.fill();
      }
      g.strokeRect(c.x - 1.6, c.z + 4.6, 3.2, 1.6);
      g.fillStyle = MAT.copper; g.fillRect(c.x - 1.4, c.z + 4.9, 0.7, 1); g.fillRect(c.x + 0.7, c.z + 4.9, 0.7, 1);
      g.fillStyle = MAT.pcbSilk; g.fillRect(c.x - 0.2, c.z + 4.75, 0.25, 1.3);
      g.restore();
    }
    // Logo and board name in the MCU column's lower area.
    g.fillStyle = MAT.pcbSilk;
    g.font = "650 3.2px Inter, sans-serif"; g.textAlign = "left"; g.textBaseline = "alphabetic";
    const c0 = centres[0];
    g.fillText("corne", c0.x - 7, c0.z + 12.8 + 2 * U);
    g.font = "1.6px JetBrains Mono, monospace";
    g.fillText(left ? "crkbd v4 · L" : "crkbd v4 · R", c0.x - 7, c0.z + 15.4 + 2 * U);
  }, 24);
  t.repeat.set(1 / w, 1 / l);
  t.offset.set(-x0 / w, z1 / l); // shape y = −z
  return t;
}

/**
 * Clear glass cover (2 mm, polished) over the nice!nano on four metal standoffs. A thin transparent coat,
 * not a transmission material: transmission draws what is behind it from a low-resolution buffer, which
 * blurred the nano's silkscreen in the macro shot.
 */
const GLASS_T = 2;
const glassMat = new THREE.MeshPhysicalMaterial({
  color: "#ffffff", metalness: 0, roughness: 0.02, clearcoat: 1, clearcoatRoughness: 0.02,
  specularIntensity: 1, envMapIntensity: 0.6, transparent: true, opacity: 0.02, depthWrite: false,
});
/** The studio's reflection on the pane: two soft diagonal light bands (a wide one and a thin one), white on transparent. */
function streakTexture(): THREE.CanvasTexture {
  const c = document.createElement("canvas");
  c.width = c.height = 512;
  const g = c.getContext("2d")!;
  const band = (at: number, half: number, a: number) => {
    const grd = g.createLinearGradient(0, 0, 512, 512);
    for (const [o, v] of [[at - half, 0], [at - half * 0.4, a], [at + half * 0.4, a], [at + half, 0]] as const) grd.addColorStop(Math.max(0, Math.min(1, o)), `rgba(255,255,255,${v})`);
    g.fillStyle = grd; g.fillRect(0, 0, 512, 512);
  };
  band(0.32, 0.14, 0.03);
  band(0.52, 0.04, 0.05);
  const t = new THREE.CanvasTexture(c);
  t.colorSpace = THREE.SRGBColorSpace;
  return t;
}
let streak: THREE.MeshBasicMaterial | null = null;

function Glass({ r, y, opacity }: { r: Rect; y: number; opacity: number }) {
  const w = r.x1 - r.x0, d = r.z1 - r.z0;
  const geo = useMemo(() => ({
    pane: roundBox(w, d, GLASS_T, 2, 0.4), post: new THREE.CylinderGeometry(1.2, 1.2, 1, 12),
    top: new THREE.PlaneGeometry(w - 1.2, d - 1.2).rotateX(-Math.PI / 2),
  }), [w, d]);
  streak ??= new THREE.MeshBasicMaterial({ map: streakTexture(), transparent: true, depthWrite: false, toneMapped: false });
  streak.opacity = opacity;
  glassMat.opacity = 0.02 * opacity;
  const posts = [[1, 1], [-1, 1], [1, -1], [-1, -1]].map(([sx, sz]) => [(r.x0 + r.x1) / 2 + sx * (w / 2 - 2.4), (r.z0 + r.z1) / 2 + sz * (d / 2 - 2)]);
  const h = y - STACK.pcb1;
  return (
    <group>
      <mesh geometry={geo.pane} position={[(r.x0 + r.x1) / 2, y, (r.z0 + r.z1) / 2]} material={glassMat} renderOrder={2} />
      <mesh geometry={geo.top} position={[(r.x0 + r.x1) / 2, y + GLASS_T + 0.02, (r.z0 + r.z1) / 2]} material={streak} renderOrder={3} />
      {posts.map(([x, z], i) => (
        <mesh key={i} geometry={geo.post} position={[x, STACK.pcb1 + h / 2, z]} scale={[1, h, 1]} material={metal()} castShadow />
      ))}
    </group>
  );
}

/** One half's static stack (bottom, PCB, plate, standoffs, MCU) at the flat board. */
function Stack({ left, opacity, explode, flatten }: { left: boolean; opacity: number; explode: number; flatten: number }) {
  const plan = useMemo(() => halfPlan(left), [left]);
  const geo = useMemo(() => ({
    bottom: slab(plan.pcb, STACK.bot0, STACK.bot1, 2.2),
    pcb: slab(plan.pcb, STACK.pcb0, STACK.pcb1, 1.6),
    plate: slab(plan.plate, STACK.plate0, STACK.plate1, 1.8),
    post: new THREE.CylinderGeometry(2, 2, STACK.plate1 + 1.3 - STACK.bot1, 6).translate(0, (STACK.plate1 + 1.3 + STACK.bot1) / 2, 0),
    pcbMap: pcbTexture(left, plan.pcb.flat(), plan.idx),
  }), [plan, left]);
  const pcbTop = useMemo(() => new THREE.MeshStandardMaterial({ map: geo.pcbMap, roughness: 0.42, metalness: 0 }), [geo]);
  if (opacity <= 0.001) return null;
  const pcbSide = matte(MAT.pcbEdge, 0.7);
  const e = explode;
  return (
    <group scale={[1, 1 - 0.9 * flatten, 1]}>
      <mesh geometry={geo.bottom} position={[0, -e * 24, 0]} material={matte(MAT.bottom, 0.9, opacity)} castShadow receiveShadow />
      <group position={[0, -e * 10, 0]}>
        <mesh geometry={geo.pcb} material={[pcbTop, pcbSide]} castShadow receiveShadow />
        <group position={[plan.mcu.x, STACK.pcb1, plan.mcu.z]}><NiceNano /></group>
        <group position={[plan.jack.x, STACK.pcb1, plan.jack.z]}><TRRS flip={!left} /></group>
        <group position={[plan.reset.x, STACK.pcb1, plan.reset.z]}><ResetButton /></group>
      </group>
      <mesh geometry={geo.plate} position={[0, e * 6, 0]} material={plateMat(opacity)} castShadow receiveShadow />
      <Glass r={plan.glass} y={STACK.plate1 + 1.5 + e * 16} opacity={opacity} />
      {e < 0.05 && plan.posts.map(([x, z], i) => <mesh key={i} geometry={geo.post} position={[x, 0, z]} material={metal("#9a9ea8", 0.3)} castShadow />)}
    </group>
  );
}

export function Corne3D({
  chars = data.layout.start, poses = FLAT, fill = "white", press, accent = null, explode = 0, stack = 1,
  spread = 36, lift = 0, flatten = 0, switches = true, legends = true, tiles = 0,
}: CorneProps) {
  const tile = useMemo(() => roundBox(18.6, 18.6, 1.5, 1.2, 0.3).translate(0, -1.5, 0), []);
  const sym = symbols(chars);
  const capFill = (i: number) => (Array.isArray(fill) ? fill[i] : fill === "fingers" ? fingerColor(KEYS[i].finger) : isEnd(KEYS[i]) ? MAT.capGray : MAT.cap);
  const capInk = (i: number) => (fill === "fingers" ? fingerInk(KEYS[i].finger) : KEY.ink);
  return (
    <group>
      {[true, false].map((left) => (
        <group key={String(left)} position={[(left ? -1 : 1) * spread / 2, 0, 0]}>
          <Stack left={left} opacity={stack} explode={explode} flatten={flatten} />
        </group>
      ))}
      {KEYS.map((k, i) => {
        const { pos, rot } = keyTransform(i, poses[i], spread, lift);
        const blue = accent === i;
        return (
          <group key={i} position={[pos.x, pos.y * (1 - 0.9 * flatten) + explode * 14, pos.z]} rotation={rot} scale={[1, 1 - 0.9 * flatten, 1]}>
            <Key size={k.size} legend={legends ? sym[i] ?? "·" : "·"} color={blue ? C.accent : capFill(i)} ink={blue ? C.white : capInk(i)}
              press={press?.[i] ?? 0} explode={explode} homing={HOMING[i]} switchOn={switches} />
            {tiles > 0.001 && <mesh geometry={tile} scale={[1 + (k.size[0] - 1), 1, 1 + (k.size[1] - 1)]} material={plateMat(tiles)} castShadow receiveShadow />}
          </group>
        );
      })}
    </group>
  );
}

/** Scene-space centre (mm) of key i's cap top at pose p: camera targets and callout anchors. */
export function keyTop(i: number, p: Pose = FLAT[i], spread = 36, lift = 0): THREE.Vector3 {
  const { pos, rot } = keyTransform(i, p, spread, lift);
  return pos.clone().add(new THREE.Vector3(0, 13.5, 0).applyEuler(rot));
}

export const FLAT_POSES = FLAT;

/** Scene-space centre (mm) of the "nice!nano" silkscreen on the left half's nano (spread 36): the opening's focus pull. */
export function nanoLabel(spread = 36): THREE.Vector3 {
  const { mcu } = halfPlan(true);
  return new THREE.Vector3(mcu.x - spread / 2, STACK.pcb1 + NANO.lift + NANO.t, mcu.z + 27.8);
}
/** Board centre (u) the scene is centred on: x, y in board space. */
export const BOARD_CENTRE = { x: BX, y: BY };

/** Key poses `t` (0..1) of the way through the board run's snapshots, interpolated between neighbours. */
export function boardPosesAt(t: number): Pose[] {
  const snaps = data.boardOpt.snapshots;
  const x = Math.max(0, Math.min(1, t)) * (snaps.length - 1);
  const i = Math.min(snaps.length - 2, Math.floor(x)), f = x - i;
  const a = snaps[i].poses, b = snaps[i + 1].poses;
  return a.map((p, k) => p.map((v, j) => v + (b[k][j] - v) * f) as Pose);
}

/** Lift (mm) that keeps the lowest key of any board-run snapshot clear of the floor. */
export const BOARD_LIFT = Math.max(0, -Math.min(...data.boardOpt.snapshots.flatMap((s) => s.poses.map((p) => p[2]))) * U) + 6;
