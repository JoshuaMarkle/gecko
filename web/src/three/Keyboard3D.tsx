// 3D keyboard: one tapered, round-cornered keycap per geometry key, placed from pos/rot/size (1 unit = 1u).
// Flat and (later) curved boards share this code path. Overlays: none, fingers, heat, n-gram arcs.
// Layered layouts show one layer at a time: the caller passes that layer's symbols per key and
// `muted` keys (on the finger holding the layer key), which are dimmed and not clickable.

import { Canvas, useFrame, useThree } from "@react-three/fiber";
import { Edges, Grid, OrbitControls, QuadraticBezierLine } from "@react-three/drei";
import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import * as THREE from "three";
import { toCreasedNormals } from "three/examples/jsm/utils/BufferGeometryUtils.js";
import type { Geometry, Key } from "../api";
import { ACCENT, fingerColor, heatColor, heatText, INK } from "../colors";

export type Overlay = "none" | "fingers" | "heat" | "ngrams";
/** One arc between the keys holding chars `ngram[0]` and `ngram[1]`; `w` in [0, 1] sets width and opacity. */
export interface Arc { ngram: string; w: number; color: string }
export type ViewMode = "3d" | "top";

interface Props {
  geometry: Geometry;
  chars: string;
  heat: number[];
  /** Keys that hold no slot on the shown layer. */
  muted?: boolean[];
  arcs: Arc[];
  overlay: Overlay;
  mode: ViewMode;
  selected: number | null;
  /** Clicked key (-1: background) and the click's client coordinates. */
  onKey: (slot: number, at?: { x: number; y: number }) => void;
  /** Changing this resets the camera and starts a slow rotation; any interaction eases it to a stop. 0 = never rotate. */
  spin?: number;
  /** Extra scene content in board-centred coordinates (see `boardBounds`, `place`). */
  children?: ReactNode;
}

export const CAP_H = 0.42;
/** Keycap profile: walls lean in by TAPER from base to top; plan-view corner radii at base and top. */
const TAPER = 0.09;
const R_BASE = 0.09;
const R_TOP = 0.07;
const CORNER_SEGS = 6;
/** Keycap color when no overlay colors the keys; muted keys (unusable on the shown layer). */
const CAP = "#ffffff";
const MUTED = "#c8c8c8";
/** Empty-slot glyph (gecko-core `layout::EMPTY`): drawn as a blank cap. */
const EMPTY = "\u00B7";
const caps = new Map<string, THREE.BufferGeometry>();
/** Case under the keys: margin around the key bounds, height, plan-view corner radius. */
const CASE_PAD = 0.3;
const CASE_H = 0.5;
const CASE_R = 0.3;

/** Case slab with its top at y = 0: rounded vertical corners like the caps, sharp top and bottom edges. */
function caseGeometry(w: number, d: number): THREE.BufferGeometry {
  const [hx, hz, r] = [w / 2, d / 2, CASE_R];
  const s = new THREE.Shape();
  s.absarc(hx - r, -hz + r, r, -Math.PI / 2, 0, false);
  s.absarc(hx - r, hz - r, r, 0, Math.PI / 2, false);
  s.absarc(-hx + r, hz - r, r, Math.PI / 2, Math.PI, false);
  s.absarc(-hx + r, -hz + r, r, Math.PI, 1.5 * Math.PI, false);
  const g = new THREE.ExtrudeGeometry(s, { depth: CASE_H, bevelEnabled: false, curveSegments: 12 });
  g.rotateX(-Math.PI / 2);
  g.translate(0, -CASE_H, 0);
  return toCreasedNormals(g, Math.PI / 4);
}

/**
 * Keycap centred on the origin: a rounded rectangle at the base lofted to a smaller rounded
 * rectangle at the top. Corners shade smooth, flat sides stay flat, and the top and base
 * edges are sharp (caps have their own vertices).
 */
function capGeometry(w: number, d: number): THREE.BufferGeometry {
  const key = `${w.toFixed(3)}x${d.toFixed(3)}`;
  let g = caps.get(key);
  if (g) return g;
  const pos: number[] = [], nor: number[] = [], idx: number[] = [];
  const vert = (x: number, y: number, z: number, n: [number, number, number]) => { pos.push(x, y, z); nor.push(...n); return pos.length / 3 - 1; };
  // Index triangles so they face along the first vertex's normal.
  const tri = (a: number, b: number, c: number) => {
    const v = (i: number) => new THREE.Vector3(pos[3 * i], pos[3 * i + 1], pos[3 * i + 2]);
    const n = new THREE.Vector3().subVectors(v(b), v(a)).cross(new THREE.Vector3().subVectors(v(c), v(a)));
    idx.push(...(n.dot(new THREE.Vector3(nor[3 * a], nor[3 * a + 1], nor[3 * a + 2])) >= 0 ? [a, b, c] : [a, c, b]));
  };
  // Rounded-rectangle outline: per corner, CORNER_SEGS + 1 points with their plan-view normal angle.
  const ring = (inset: number, r: number) => {
    const [hx, hz] = [w / 2 - inset - r, d / 2 - inset - r];
    const out: { x: number; z: number; t: number }[] = [];
    [[hx, hz], [-hx, hz], [-hx, -hz], [hx, -hz]].forEach(([cx, cz], c) => {
      for (let i = 0; i <= CORNER_SEGS; i++) {
        const t = (c + i / CORNER_SEGS) * (Math.PI / 2);
        out.push({ x: cx + r * Math.cos(t), z: cz + r * Math.sin(t), t });
      }
    });
    return out;
  };
  const [y0, y1] = [-CAP_H / 2, CAP_H / 2];
  const base = ring(0, R_BASE), top = ring(TAPER, R_TOP);
  const lean = Math.atan2(TAPER, CAP_H);
  const sideN = (t: number): [number, number, number] => [Math.cos(t) * Math.cos(lean), Math.sin(lean), Math.sin(t) * Math.cos(lean)];
  const m = base.length;
  const sb = base.map((p) => vert(p.x, y0, p.z, sideN(p.t)));
  const st = top.map((p) => vert(p.x, y1, p.z, sideN(p.t)));
  for (let i = 0; i < m; i++) {
    const j = (i + 1) % m;
    tri(sb[i], sb[j], st[j]);
    tri(sb[i], st[j], st[i]);
  }
  for (const [ring_, y, n] of [[top, y1, [0, 1, 0]], [base, y0, [0, -1, 0]]] as const) {
    const c = vert(0, y, 0, [...n]);
    const vs = ring_.map((p) => vert(p.x, y, p.z, [...n]));
    for (let i = 0; i < m; i++) tri(c, vs[i], vs[(i + 1) % m]);
  }
  g = new THREE.BufferGeometry();
  g.setAttribute("position", new THREE.Float32BufferAttribute(pos, 3));
  g.setAttribute("normal", new THREE.Float32BufferAttribute(nor, 3));
  g.setIndex(idx);
  caps.set(key, g);
  return g;
}

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

/** Key bounds of a geometry in board units: centre and size (the 3D scene is centred on it). */
export function boardBounds(keys: Key[]): { cx: number; cy: number; w: number; h: number } {
  const xs = keys.flatMap((k) => [k.pos[0], k.pos[0] + k.size[0]]);
  const ys = keys.flatMap((k) => [k.pos[1], k.pos[1] + k.size[1]]);
  const [x0, x1, y0, y1] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
  return { cx: (x0 + x1) / 2, cy: (y0 + y1) / 2, w: x1 - x0, h: y1 - y0 };
}

/** Board-space (x right, y toward user, z up) → three (x right, y up, z toward viewer). */
export function place(k: Key, cx: number, cy: number): { p: [number, number, number]; r: [number, number, number] } {
  const d = Math.PI / 180;
  return {
    p: [k.pos[0] + k.size[0] / 2 - cx, k.pos[2], k.pos[1] + k.size[1] / 2 - cy],
    r: [k.rot[0] * d, -k.rot[2] * d, k.rot[1] * d],
  };
}

function Keycap({ k, cx, cy, ch, color, text, selected, onClick }: {
  k: Key; cx: number; cy: number; ch: string; color: string; text: string; selected: boolean; onClick: (at: { x: number; y: number }) => void;
}) {
  const [hover, setHover] = useState(false);
  const { p, r } = place(k, cx, cy);
  const w = k.size[0] - 0.08;
  const h = k.size[1] - 0.08;
  const lift = selected ? 0.12 : hover ? 0.04 : 0;
  return (
    <group position={[p[0], p[1] + CAP_H / 2 + lift, p[2]]} rotation={r}>
      <mesh
        geometry={capGeometry(w, h)}
        onClick={(e) => { e.stopPropagation(); onClick({ x: e.nativeEvent.clientX, y: e.nativeEvent.clientY }); }}
        onPointerOver={(e) => { e.stopPropagation(); setHover(true); document.body.style.cursor = "pointer"; }}
        onPointerOut={() => { setHover(false); document.body.style.cursor = ""; }}
      >
        <meshStandardMaterial color={color} roughness={0.75} metalness={0} />
        {selected && <Edges color={ACCENT} lineWidth={2} threshold={30} />}
      </mesh>
      <mesh position={[0, CAP_H / 2 + 0.002, 0]} rotation={[-Math.PI / 2, 0, 0]}>
        <planeGeometry args={[0.6, 0.6]} />
        <meshBasicMaterial map={legend(ch, text)} transparent depthWrite={false} />
      </mesh>
    </group>
  );
}

/** Slow orbit while `on`; eases to a stop once `stop` is set. */
function AutoRotate({ on, stop }: { on: boolean; stop: boolean }) {
  const { controls } = useThree() as unknown as { controls: { autoRotate: boolean; autoRotateSpeed: number } | null };
  const speed = useRef(0);
  useEffect(() => { speed.current = on ? 0.7 : 0; }, [on]);
  useFrame(() => {
    if (!controls) return;
    if (stop) speed.current = speed.current < 0.01 ? 0 : speed.current * 0.93;
    controls.autoRotate = speed.current > 0;
    controls.autoRotateSpeed = speed.current;
  });
  return null;
}

function CameraRig({ mode, w, h, reset }: { mode: ViewMode; w: number; h: number; reset: number }) {
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
  }, [mode, w, h, aspect, camera, controls, reset]);
  return null;
}

export function Keyboard3D({ geometry, chars, heat, muted, arcs, overlay, mode, selected, onKey, spin = 0, children }: Props) {
  const keys = geometry.key;
  const bounds = useMemo(() => boardBounds(keys), [keys]);
  // Auto-rotate: a new `spin` restarts it; the first interaction stops it until the next one.
  const [touched, setTouched] = useState(false);
  useEffect(() => setTouched(false), [spin]);
  const touch = () => { if (!touched) setTouched(true); };
  const caseGeo = useMemo(() => caseGeometry(bounds.w + 2 * CASE_PAD, bounds.h + 2 * CASE_PAD), [bounds.w, bounds.h]);
  useEffect(() => () => caseGeo.dispose(), [caseGeo]);
  const maxHeat = Math.max(...heat, 1e-9);
  const sym = useMemo(() => [...chars], [chars]);
  const slotOf = useMemo(() => new Map(sym.map((c, i) => [c, i])), [sym]);

  return (
    <Canvas camera={{ fov: 32, near: 0.1, far: 200 }} dpr={[1, 2]} onPointerMissed={() => onKey(-1)}
      onPointerEnter={spin ? touch : undefined} onPointerDown={spin ? touch : undefined} onWheel={spin ? touch : undefined}>
      <color attach="background" args={["#f3f3f3"]} />
      <ambientLight intensity={1.8} />
      <directionalLight position={[3, 10, -6]} intensity={4.2} />
      <directionalLight position={[-6, 6, 4]} intensity={0.4} />
      {/* Floor grid under the case: 1u cells, 5u sections, fading out with distance. */}
      <Grid position={[0, -CASE_H - 0.001, 0]} infiniteGrid cellSize={1} sectionSize={5} cellThickness={0.6} sectionThickness={1} cellColor="#dcdcdc" sectionColor="#c4c4c4" fadeDistance={40} fadeStrength={1.5} />
      <CameraRig mode={mode} w={bounds.w} h={bounds.h} reset={spin} />
      {spin > 0 && <AutoRotate on={mode === "3d"} stop={touched} key={spin} />}
      <mesh geometry={caseGeo}>
        <meshStandardMaterial color="#d2d2d2" roughness={0.9} />
      </mesh>
      {keys.map((k, i) => {
        const off = muted?.[i] ?? false;
        const ch = off || sym[i] === EMPTY ? "" : sym[i] ?? "";
        const t = (heat[i] ?? 0) / maxHeat;
        const color = off ? MUTED : overlay === "heat" ? heatColor(t) : overlay === "fingers" || overlay === "ngrams" ? fingerColor(k.finger) : CAP;
        const text = overlay === "heat" ? heatText(t) : INK;
        return (
          <Keycap key={i} k={k} cx={bounds.cx} cy={bounds.cy} ch={ch} color={color} text={text} selected={selected === i} onClick={(at) => onKey(i, at)} />
        );
      })}
      {overlay === "ngrams" && arcs.map((a, n) => {
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
            mid={[(pi[0] + pj[0]) / 2, top + 0.25 + dist * 0.3, (pi[2] + pj[2]) / 2]}
            color={a.color}
            lineWidth={0.6 + 5 * a.w}
            transparent
            opacity={0.15 + 0.85 * a.w}
            depthWrite={false}
            renderOrder={arcs.length - n}
          />
        );
      })}
      {children}
      <OrbitControls makeDefault enableRotate={mode === "3d"} enablePan enableDamping dampingFactor={0.12} maxPolarAngle={Math.PI / 2.05} minDistance={3} maxDistance={40} />
    </Canvas>
  );
}
