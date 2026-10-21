// Hand tab (frontend.md "Hand"): hand settings | 3D wireframe hands posed by IK on the board's
// home keys, with a typing simulation | measurements, size distribution, similarity.
// Preview only: the scorer has no hand model yet (#92).

import { useFrame } from "@react-three/fiber";
import { Html } from "@react-three/drei";
import { useMemo, useRef, useState } from "react";
import * as THREE from "three";
import type { Geometry } from "../../api";
import { ACCENT, HAIR, INK, MUTED } from "../../colors";
import {
  AVERAGE, FINGER_NAMES, dimsVector, handDims, handLength, LENGTH_MEAN, LENGTH_SD, normalCdf, normalPdf, population, poseHand,
  PRESET_SCALE, scaleDims, similarity, sizeBand, typicality,
  type FingerName, type HandDims, type HandPose, type HandSettings, type Preset, type V3,
} from "../../hand";
import { boardBounds, CAP_H, Keyboard3D, place, type ViewMode } from "../../three/Keyboard3D";
import { useSize } from "../Charts";
import { Frame, Num, PanelTabs, Sec, Seg, Toggle } from "../Form";

interface Props {
  geometry: Geometry;
  chars: string;
  hand: HandSettings;
  setHand: (h: HandSettings) => void;
}

const MM = 1 / 19.05;
const SAMPLES: [string, string][] = [
  ["pangram", "the quick brown fox jumps over the lazy dog"],
  ["prose", "it was the best of times, it was the worst of times, it was the age of wisdom"],
  ["code", "for (let i = 0; i < n; i++) { total += values[i]; }"],
];
const SPEEDS: [string, string][] = [["slow", "Slow"], ["normal", "Normal"], ["fast", "Fast"]];
const SPEED_WPM: Record<string, number> = { slow: 15, normal: 40, fast: 80 };
const FINGER_OF: Record<string, FingerName> = { P: "pinky", R: "ring", M: "middle", I: "index", T: "thumb" };
const SEG_NAMES: Record<FingerName, [string, string, string]> = {
  thumb: ["Metacarpal", "Proximal", "Distal"], index: ["Proximal", "Middle", "Distal"], middle: ["Proximal", "Middle", "Distal"],
  ring: ["Proximal", "Middle", "Distal"], pinky: ["Proximal", "Middle", "Distal"],
};
const cap = (s: string) => s[0].toUpperCase() + s.slice(1);

// ---------- 3D ----------

type Targets = Record<"L" | "R", Record<FingerName, V3>>;

/** Home targets (key tops) per hand and finger, in board-centred three coordinates. */
function homeTargets(g: Geometry): { home: Targets; keyTop: (slot: number) => V3 } {
  const b = boardBounds(g.key);
  const keyTop = (i: number): V3 => { const p = place(g.key[i], b.cx, b.cy).p; return [p[0], p[1] + CAP_H, p[2]]; };
  const home = { L: {}, R: {} } as Targets;
  for (const side of ["L", "R"] as const) {
    for (const [code, name] of Object.entries(FINGER_OF)) {
      const f = `${side}${code}`;
      const ks = g.key.map((k, i) => ({ k, i })).filter((x) => x.k.finger === f);
      const pick = ks.find((x) => x.k.home && !x.k.stretch) ?? ks.find((x) => x.k.home) ?? ks[0];
      if (pick) home[side][name] = keyTop(pick.i);
    }
    // A board without thumb keys: rest the thumb below the bottom row, inside of the index finger.
    if (!home[side].thumb && home[side].index) {
      const ix = home[side].index;
      home[side].thumb = [ix[0] + (side === "L" ? 1 : -1) * 1.1, ix[1] - 0.3, ix[2] + 2.4];
    }
  }
  return { home, keyTop };
}

/** One frame of typing: which finger moves where, as a fraction of the way out and back. */
function typingTargets(home: Targets, slotOf: Map<string, number>, g: Geometry, keyTop: (i: number) => V3, text: string, t: number): Targets {
  const out: Targets = { L: { ...home.L }, R: { ...home.R } };
  if (!text) return out;
  const pos = t % text.length;
  const ch = text[Math.floor(pos)];
  const phase = pos - Math.floor(pos);
  const slot = slotOf.get(ch === " " ? "␣" : ch) ?? slotOf.get(ch.toLowerCase());
  let side: "L" | "R" | null = null, finger: FingerName | null = null, target: V3 | null = null;
  if (slot !== undefined) {
    const f = g.key[slot].finger;
    side = f[0] as "L" | "R";
    finger = FINGER_OF[f[1]];
    target = keyTop(slot);
  } else if (ch === " ") {
    side = "R"; finger = "thumb"; target = home.R.thumb;
  }
  if (!side || !finger || !target || !out[side][finger]) return out;
  const h = home[side][finger];
  // Out (0–0.4), press (0.4–0.6), back (0.6–1); smoothstep easing; a 0.25u lift on the way.
  const ease = (x: number) => x * x * (3 - 2 * x);
  const k = phase < 0.4 ? ease(phase / 0.4) : phase < 0.6 ? 1 : 1 - ease((phase - 0.6) / 0.4);
  const lift = Math.sin(Math.PI * Math.min(1, phase / 0.4)) * 0.25 * (phase < 0.4 ? 1 : 0);
  const press = phase >= 0.4 && phase < 0.6 ? -0.08 * Math.sin(Math.PI * (phase - 0.4) / 0.2) : 0;
  out[side][finger] = [h[0] + (target[0] - h[0]) * k, h[1] + (target[1] - h[1]) * k + lift + press, h[2] + (target[2] - h[2]) * k];
  return out;
}

/** Line segments (pairs of points) and joints of both hands. */
function poseLines(poses: HandPose[]): { seg: number[]; joints: number[] } {
  const seg: number[] = [], joints: number[] = [];
  for (const p of poses) {
    for (const ch of p.fingers) {
      for (let i = 0; i < ch.length - 1; i++) seg.push(...ch[i], ...ch[i + 1]);
      for (const j of ch) joints.push(...j);
    }
    const ring = p.palm;
    for (let i = 0; i < ring.length; i++) seg.push(...ring[i], ...ring[(i + 1) % ring.length]);
    joints.push(...ring[0], ...ring[1]);
  }
  return { seg, joints };
}

function Hands({ g, chars, dims, text, wpm, playing, labels }: { g: Geometry; chars: string; dims: HandDims; text: string; wpm: number; playing: boolean; labels: boolean }) {
  const { home, keyTop } = useMemo(() => homeTargets(g), [g]);
  const slotOf = useMemo(() => new Map([...chars].map((c, i) => [c, i])), [chars]);
  const rest = useMemo(() => (["L", "R"] as const).filter((s) => home[s].index).map((s) => poseHand(s, home[s], dims, MM)), [home, dims]);
  const init = useMemo(() => poseLines(rest), [rest]);
  const lines = useRef<THREE.LineSegments>(null);
  const points = useRef<THREE.Points>(null);
  const clock = useRef(0);
  useFrame((_, dt) => {
    if (!playing || !lines.current || !points.current) return;
    clock.current += dt * (wpm * 5) / 60;
    const tg = typingTargets(home, slotOf, g, keyTop, text, clock.current);
    const poses = (["L", "R"] as const).filter((s) => home[s].index).map((s) => poseHand(s, tg[s], dims, MM));
    const { seg, joints } = poseLines(poses);
    (lines.current.geometry.attributes.position as THREE.BufferAttribute).set(seg);
    lines.current.geometry.attributes.position.needsUpdate = true;
    (points.current.geometry.attributes.position as THREE.BufferAttribute).set(joints);
    points.current.geometry.attributes.position.needsUpdate = true;
  });
  // Reset to the rest pose when stopped.
  const key = `${playing}|${init.seg.length}|${dims.palmWidth}|${g.id}|${handLength(dims)}`;
  const right = rest[rest.length - 1];
  return (
    <group key={key}>
      <lineSegments ref={lines} renderOrder={10}>
        <bufferGeometry><bufferAttribute attach="attributes-position" args={[new Float32Array(init.seg), 3]} /></bufferGeometry>
        <lineBasicMaterial color={INK} depthTest={false} transparent opacity={0.85} />
      </lineSegments>
      <points ref={points} renderOrder={11}>
        <bufferGeometry><bufferAttribute attach="attributes-position" args={[new Float32Array(init.joints), 3]} /></bufferGeometry>
        <pointsMaterial color={INK} size={6} sizeAttenuation={false} depthTest={false} />
      </points>
      {labels && !playing && right && right.fingers.map((ch, fi) => ch.slice(0, -1).map((a, si) => {
        const b = ch[si + 1];
        const f = FINGER_NAMES[fi];
        return (
          <Html key={`${fi}-${si}`} position={[(a[0] + b[0]) / 2, (a[1] + b[1]) / 2 + 0.05, (a[2] + b[2]) / 2]} center zIndexRange={[1, 0]}>
            <span className="handlabel">{dims[f][si].toFixed(0)}</span>
          </Html>
        );
      }))}
    </group>
  );
}

// ---------- bottom panels ----------

/** One hand palm-down in 2D (right hand, fingers up) with dimension lines. */
function Measurements({ d }: { d: HandDims }) {
  const [box, { w, h }] = useSize<HTMLDivElement>();
  const fingers: FingerName[] = ["index", "middle", "ring", "pinky"];
  const len = (f: FingerName) => d[f][0] + d[f][1] + d[f][2];
  const totalH = d.palmLength + Math.max(...fingers.map(len)) + 30;
  const totalW = d.palmWidth + d.thumb[0] + d.thumb[1] + d.thumb[2] + 120;
  const s = Math.min((w - 20) / totalW, (h - 20) / totalH);
  const ox = 20 + (d.thumb[0] + d.thumb[1] + d.thumb[2]) * 0.75 * s + 40, oy = 10 + (Math.max(...fingers.map(len)) + 14) * s;
  const kx = (i: number) => ox + (i / 3) * d.palmWidth * s;
  const arch = [3, 0, 2, 8];
  const tips: [number, number][] = [];
  const els: React.ReactNode[] = [];
  fingers.forEach((f, i) => {
    let y = oy + arch[i] * s;
    const x = kx(i);
    els.push(<circle key={`k${f}`} cx={x} cy={y} r={2.5} fill={INK} />);
    d[f].forEach((l, j) => {
      const y2 = y - l * s;
      els.push(<line key={`${f}${j}`} x1={x} y1={y} x2={x} y2={y2} stroke={INK} strokeWidth={1.4} />);
      els.push(<circle key={`${f}j${j}`} cx={x} cy={y2} r={2} fill={INK} />);
      els.push(<text key={`${f}t${j}`} x={x + 5} y={(y + y2) / 2 + 3}>{l.toFixed(0)}</text>);
      y = y2;
    });
    tips.push([x, y]);
  });
  // Thumb: from the base of the palm on the left (right hand, palm down), angled out 50°.
  const tb: [number, number] = [ox - 6 * s, oy + d.palmLength * 0.62 * s];
  let [tx, ty] = tb;
  const ang = (-140 * Math.PI) / 180;
  els.push(<circle key="tb" cx={tx} cy={ty} r={2.5} fill={INK} />);
  d.thumb.forEach((l, j) => {
    const a = ang + j * 0.12;
    const x2 = tx + Math.cos(a) * l * s, y2 = ty + Math.sin(a) * l * s;
    els.push(<line key={`th${j}`} x1={tx} y1={ty} x2={x2} y2={y2} stroke={INK} strokeWidth={1.4} />);
    els.push(<circle key={`thj${j}`} cx={x2} cy={y2} r={2} fill={INK} />);
    els.push(<text key={`tht${j}`} x={(tx + x2) / 2 - 6} y={(ty + y2) / 2 + 12} textAnchor="end">{l.toFixed(0)}</text>);
    [tx, ty] = [x2, y2];
  });
  const wristY = oy + d.palmLength * s;
  const pinkTip = tips[3];
  return (
    <div ref={box} className="chartfill">
      <svg width={w} height={h}>
        <path d={`M${kx(0)},${oy + arch[0] * s} L${kx(3)},${oy + arch[3] * s} L${kx(3) - 4 * s},${wristY} L${kx(0) + 8 * s},${wristY} L${tb[0]},${tb[1]} Z`} fill="none" stroke={MUTED} />
        {els}
        {/* palm width */}
        <line x1={kx(0)} x2={kx(3)} y1={oy + 16 * s} y2={oy + 16 * s} stroke={ACCENT} markerEnd="url(#arr)" markerStart="url(#arr)" />
        <text className="ink" x={(kx(0) + kx(3)) / 2} y={oy + 16 * s - 4} textAnchor="middle">palm width {d.palmWidth.toFixed(0)}</text>
        {/* palm length */}
        <line x1={kx(3) + 18 * s} x2={kx(3) + 18 * s} y1={oy} y2={wristY} stroke={ACCENT} markerEnd="url(#arr)" markerStart="url(#arr)" />
        <text className="ink" x={kx(3) + 22 * s} y={(oy + wristY) / 2}>palm length {d.palmLength.toFixed(0)}</text>
        {/* span */}
        <line x1={tx} y1={ty} x2={pinkTip[0]} y2={pinkTip[1]} stroke={HAIR} strokeDasharray="4 3" />
        <text x={tx - 4} y={ty - 6} textAnchor="end"><title>Thumb tip to pinky tip, spread flat</title>span {d.span.toFixed(0)}</text>
        {/* reach around the index tip */}
        <circle cx={tips[1][0]} cy={tips[1][1]} r={d.reach * s} fill="none" stroke={ACCENT} strokeDasharray="3 3" opacity={0.6} />
        <text x={tips[1][0] + d.reach * s + 4} y={tips[1][1] - 4}>reach {d.reach.toFixed(0)}</text>
        <defs><marker id="arr" viewBox="0 0 6 6" refX="3" refY="3" markerWidth="6" markerHeight="6" orient="auto-start-reverse"><path d="M0,0L6,3L0,6" fill={ACCENT} /></marker></defs>
      </svg>
    </div>
  );
}

function SizeDistribution({ d }: { d: HandDims }) {
  const [box, { w, h }] = useSize<HTMLDivElement>();
  const m = { l: 16, r: 16, t: 26, b: 28 };
  const lo = LENGTH_MEAN - 3.5 * LENGTH_SD, hi = LENGTH_MEAN + 3.5 * LENGTH_SD;
  const sx = (x: number) => m.l + ((x - lo) / (hi - lo)) * (w - m.l - m.r);
  const peak = normalPdf(LENGTH_MEAN, LENGTH_MEAN, LENGTH_SD);
  const sy = (y: number) => h - m.b - (y / peak) * (h - m.t - m.b);
  const pts = Array.from({ length: 121 }, (_, i) => lo + ((hi - lo) * i) / 120);
  const curve = (a: number, b: number) => {
    const xs = pts.filter((x) => x >= a && x <= b);
    return `M${sx(xs[0])},${sy(0)} ` + xs.map((x) => `L${sx(x)},${sy(normalPdf(x, LENGTH_MEAN, LENGTH_SD))}`).join(" ") + ` L${sx(xs[xs.length - 1])},${sy(0)} Z`;
  };
  const cur = handLength(d);
  const pct = 100 * normalCdf((cur - LENGTH_MEAN) / LENGTH_SD);
  const band = (a: number, b: number, label: string, fill: string) => (
    <g>
      <path d={curve(a, b)} fill={fill} />
      <text x={sx((Math.max(a, lo) + Math.min(b, hi)) / 2)} y={h - 10} textAnchor="middle">{label}</text>
    </g>
  );
  return (
    <div ref={box} className="chartfill">
      <svg width={w} height={h}>
        {band(lo, LENGTH_MEAN - LENGTH_SD, "Small", "#ececec")}
        {band(LENGTH_MEAN - LENGTH_SD, LENGTH_MEAN + LENGTH_SD, "Average", "#dcdcdc")}
        {band(LENGTH_MEAN + LENGTH_SD, hi, "Large", "#ececec")}
        <path d={pts.map((x, i) => `${i ? "L" : "M"}${sx(x)},${sy(normalPdf(x, LENGTH_MEAN, LENGTH_SD))}`).join(" ")} fill="none" stroke={INK} strokeWidth={1.2} />
        {(["small", "average", "large"] as const).map((p) => {
          const x = sx(handLength(scaleDims(AVERAGE, PRESET_SCALE[p])));
          return <line key={p} x1={x} x2={x} y1={h - m.b} y2={h - m.b + 5} stroke={MUTED} />;
        })}
        <line x1={sx(cur)} x2={sx(cur)} y1={m.t - 6} y2={h - m.b} stroke={ACCENT} strokeWidth={2} />
        <text className="ink" x={sx(cur) + 6} y={m.t + 4}>this hand · {cur.toFixed(0)} mm · {sizeBand(cur)} · {pct.toFixed(0)}th percentile</text>
        <text x={m.l} y={14}>hand length (wrist crease to middle fingertip), adult population · bands at ±1 sd</text>
      </svg>
    </div>
  );
}

function Similarity({ d }: { d: HandDims }) {
  const [box, { w, h }] = useSize<HTMLDivElement>();
  const data = useMemo(() => {
    const pop = population(600, 7);
    const ref = pop.slice(0, 150);
    return { pop, typ: pop.map((v) => typicality(v, ref)), ref };
  }, []);
  const v = dimsVector(d);
  const mine = typicality(v, data.ref);
  const nearest = Math.max(...data.pop.map((p) => similarity(v, p)));
  const bins = 30;
  const lo = Math.min(...data.typ, mine), hi = Math.max(...data.typ, mine);
  const counts = new Array(bins).fill(0);
  for (const t of data.typ) counts[Math.min(bins - 1, Math.floor(((t - lo) / (hi - lo)) * bins))]++;
  const below = data.typ.filter((t) => t < mine).length / data.typ.length;
  const m = { l: 16, r: 16, t: 26, b: 24 };
  const bw = (w - m.l - m.r) / bins;
  const max = Math.max(...counts);
  const sx = (t: number) => m.l + ((t - lo) / (hi - lo)) * (w - m.l - m.r);
  return (
    <div ref={box} className="chartfill">
      <svg width={w} height={h}>
        <text x={m.l} y={14}>typicality: mean similarity to other hands (synthetic population until reference data lands)</text>
        {counts.map((c, i) => (
          <rect key={i} x={m.l + i * bw + 1} y={h - m.b - (c / max) * (h - m.t - m.b)} width={bw - 2} height={(c / max) * (h - m.t - m.b)} fill={HAIR} rx={1} />
        ))}
        <line x1={sx(mine)} x2={sx(mine)} y1={m.t - 6} y2={h - m.b} stroke={ACCENT} strokeWidth={2} />
        <text className="ink" x={Math.min(sx(mine) + 6, w - 260)} y={m.t + 4}>this hand · more typical than {(100 * below).toFixed(0)}% · closest match {(100 * nearest).toFixed(0)}%</text>
        <text x={m.l} y={h - 6}>less typical</text>
        <text x={w - m.r} y={h - 6} textAnchor="end">more typical</text>
      </svg>
    </div>
  );
}

// ---------- view ----------

export function HandView(p: Props) {
  const h = p.hand;
  const d = handDims(h);
  const off = !h.use;
  const [tab, setTab] = useState<"measure" | "size" | "similar">("measure");
  const [sample, setSample] = useState("pangram");
  const [wpm, setWpm] = useState(40);
  const [playing, setPlaying] = useState(false);
  const [mode, setMode] = useState<ViewMode>("3d");
  const setDims = (next: HandDims) => p.setHand({ ...h, preset: "custom", custom: next });
  const setSeg = (f: FingerName, i: number, v: number) => { const s = [...d[f]] as [number, number, number]; s[i] = Math.max(1, v); setDims({ ...d, [f]: s }); };
  const speed = Object.entries(SPEED_WPM).find(([, v]) => v === wpm)?.[0] ?? "";

  return (
    <Frame
      title="Hand"
      side={
        <>
          <div className="pad">
            <Toggle label="Use hand in optimization" help="Score layouts with this hand's reach (needs the hand model in the scorer, #92). Off: preview only." value={h.use} onChange={(use) => p.setHand({ ...h, use })} />
            {off && <div className="hint">Off: the scorer ignores the hand. The views on the right still preview it.</div>}
          </div>
          <fieldset className="plain" disabled={off}>
            <div className="pad">
              <div className="row">
                <label title="Preset hand sizes. Editing a value below switches to Custom.">Preset</label>
                <Seg value={h.preset} disabled={off} onChange={(preset: Preset) => p.setHand({ ...h, preset })}
                  options={[["average", "Average"], ["large", "Large"], ["small", "Small"], ...(h.custom ? [["custom", "Custom"] as [Preset, string]] : [])]} />
                <span />
              </div>
              <div className="kv"><span>Hand length</span><span>{handLength(d).toFixed(0)} mm · {sizeBand(handLength(d))}</span></div>
            </div>
            <Sec id="hand-adv" title="Advanced" note="custom values, mm" defaultOpen={false}>
              {FINGER_NAMES.map((f) => (
                <div key={f} className="subgroup">
                  <div className="subhead">{cap(f)}</div>
                  {d[f].map((v, i) => (
                    <Num key={i} label={SEG_NAMES[f][i]} help={`${cap(f)} ${SEG_NAMES[f][i].toLowerCase()} segment length.`} value={+v.toFixed(1)} step={0.5} unit="mm" disabled={off} onChange={(x) => setSeg(f, i, x)} />
                  ))}
                </div>
              ))}
              <div className="subgroup">
                <div className="subhead">Palm and reach</div>
                <Num label="Palm width" help="Across the index–pinky knuckles." value={+d.palmWidth.toFixed(1)} step={0.5} unit="mm" disabled={off} onChange={(x) => setDims({ ...d, palmWidth: Math.max(30, x) })} />
                <Num label="Palm length" help="Wrist crease to the middle knuckle." value={+d.palmLength.toFixed(1)} step={0.5} unit="mm" disabled={off} onChange={(x) => setDims({ ...d, palmLength: Math.max(40, x) })} />
                <Num label="Span" help="Thumb tip to pinky tip, spread flat." value={+d.span.toFixed(1)} step={1} unit="mm" disabled={off} onChange={(x) => setDims({ ...d, span: Math.max(80, x) })} />
                <Num label="Reach" help="Comfortable fingertip travel from the home key." value={+d.reach.toFixed(1)} step={1} unit="mm" disabled={off} onChange={(x) => setDims({ ...d, reach: Math.max(5, x) })} />
              </div>
              <div className="btns"><button className="btn soft small" disabled={off || h.preset !== "custom"} onClick={() => p.setHand({ ...h, preset: "average", custom: null })}>Reset to Average</button></div>
            </Sec>
          </fieldset>
        </>
      }
      main={
        <div className="viewport">
          <Keyboard3D geometry={p.geometry} chars={p.chars} heat={[]} arcs={[]} overlay="none" mode={mode} selected={null} onKey={() => {}}>
            <Hands g={p.geometry} chars={p.chars} dims={d} text={SAMPLES.find(([k]) => k === sample)?.[1] ?? ""} wpm={wpm} playing={playing} labels />
          </Keyboard3D>
          <div className="viewlabel"><span>Hands on {p.geometry.name}</span><span className="muted">fingertips on the home keys (IK)</span></div>
          <div className="viewctl">
            <div className="seg inline"><button className={mode === "3d" ? "on" : ""} onClick={() => setMode(mode === "3d" ? "top" : "3d")} title="3D view (off: flat top-down)">3D</button></div>
          </div>
          <div className="typingbar">
            <button className="btn primary small" onClick={() => setPlaying(!playing)}>{playing ? "Stop" : "Type"}</button>
            <select value={sample} onChange={(e) => setSample(e.target.value)} title="Sample text">
              {SAMPLES.map(([k, t]) => <option key={k} value={k}>{t.length > 34 ? `${t.slice(0, 34)}…` : t}</option>)}
            </select>
            <Seg inline value={speed} options={SPEEDS} onChange={(s) => setWpm(SPEED_WPM[s])} />
            <input type="range" min={5} max={120} step={5} value={wpm} onChange={(e) => setWpm(+e.target.value)} title="Words per minute" />
            <span className="muted">{wpm} wpm</span>
          </div>
        </div>
      }
      bottom={
        <div className="bottomtabs">
          <PanelTabs value={tab} onChange={setTab} options={[["measure", "Measurements"], ["size", "Size distribution"], ["similar", "Similarity"]]}
            right={<span className="hint">{h.preset === "custom" ? "Custom" : cap(h.preset)} hand</span>} />
          <div className="bottombody">
            {tab === "measure" && <Measurements d={d} />}
            {tab === "size" && <SizeDistribution d={d} />}
            {tab === "similar" && <Similarity d={d} />}
          </div>
        </div>
      }
    />
  );
}
