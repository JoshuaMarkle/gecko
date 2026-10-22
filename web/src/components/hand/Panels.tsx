// The Hand tab's bottom panels: the scorer's per-key reach costs, measurements, the typing
// timeline, and where the hand sits among the size distribution and other hands.

import { useEffect, useMemo, useRef, type MutableRefObject } from "react";
import type { Finger, Geometry } from "../../api";
import { ACCENT, ERROR, fingerColor, HAIR, INK, MUTED } from "../../colors";
import {
  AVERAGE, dimsVector, FINGER_NAMES, fingerKey, fingerLength, FOUR, handLayout, handLength, LENGTH_MEAN, LENGTH_SD, naturalTip,
  normalCdf, normalPdf, population, PRESET_SCALE, scaleDims, similarity, sizeBand, typicality,
  type Body, type FingerName, type HandDims, type Plan, type Side, type V3,
} from "../../hand";
import { cap } from "../../format";
import { useSize } from "../Charts";
import type { HandCosts } from "../../wasm";
import { CODE_OF, LOOP_GAP, MM } from "./rigs";


const FINGER_ORDER: Finger[] = ["LP", "LR", "LM", "LI", "LT", "RT", "RI", "RM", "RR", "RP"];

/**
 * The scorer's per-key hand costs by finger: home key, mean and worst key (G01 reach), mean
 * press angle (G05), and keys the fingertip cannot reach with the hand resting on the home keys;
 * then each wrist's resting posture and G03.
 */
export function Reach({ geometry, chars, costs, body }: { geometry: Geometry; chars: string; costs: HandCosts | null; body: Body }) {
  const bodyText = `${body.elbowWidth.toFixed(0)} mm`;
  if (!costs) return <div className="empty">Solving the hand…</div>;
  const sym = [...chars];
  const name = (i: number) => (sym[i] && sym[i] !== " " ? sym[i].toUpperCase() : `r${geometry.key[i].row}c${geometry.key[i].col}`);
  const base = geometry.key.map((k, i) => ({ k, i }));
  const rows = FINGER_ORDER.map((f) => {
    const ks = base.filter((x) => x.k.finger === f).map((x) => x.i);
    if (!ks.length) return null;
    const home = ks.find((i) => geometry.key[i].home && !geometry.key[i].stretch) ?? ks.find((i) => geometry.key[i].home);
    const worst = ks.reduce((a, b) => (costs.reach[b] > costs.reach[a] ? b : a));
    const mean = (v: number[]) => ks.reduce((t, i) => t + v[i], 0) / ks.length;
    const short = ks.filter((i) => costs.miss[i] > 0);
    return { f, n: ks.length, home, worst, reach: mean(costs.reach), align: mean(costs.align), short };
  }).filter((r) => r !== null);
  const all = base.map((x) => x.i);
  const avg = all.reduce((t, i) => t + costs.reach[i], 0) / Math.max(all.length, 1);
  const sideMean = (side: string, v: number[]) => {
    const ks = all.filter((i) => geometry.key[i].finger.startsWith(side));
    return ks.reduce((t, i) => t + v[i], 0) / Math.max(ks.length, 1);
  };
  const deg = (v: number) => `${v.toFixed(0)}°`;
  return (
    <div className="reachtab">
      <table className="tbl">
        <thead><tr><th>Finger</th><th>Keys</th><th title="G01 on the finger's home key">Home</th><th title="Mean G01 over the finger's keys">Mean reach</th><th>Hardest key</th><th title="Mean G05 press angle cost">Press angle</th><th title="Keys the fingertip cannot reach with the hand on the home keys (shortfall, mm)">Out of reach</th></tr></thead>
        <tbody>
          {rows.map((r) => (
            <tr key={r.f}>
              <td><i className="dot" style={{ background: fingerColor(r.f) }} />{r.f}</td>
              <td className="num">{r.n}</td>
              <td className="num">{r.home === undefined ? "–" : costs.reach[r.home].toFixed(2)}</td>
              <td className="num">{r.reach.toFixed(2)}</td>
              <td>{name(r.worst)} <span className="muted num">{costs.reach[r.worst].toFixed(1)}</span></td>
              <td className="num">{r.align.toFixed(2)}</td>
              <td className={r.short.length ? "err" : "muted"}>{r.short.length ? r.short.map((i) => `${name(i)} ${costs.miss[i].toFixed(0)}`).join(", ") : "none"}</td>
            </tr>
          ))}
        </tbody>
      </table>
      <table className="tbl posture">
        <thead><tr><th>Wrist</th><th title="Ulnar (+, toward the pinky) or radial (−) deviation of the hand from the forearm">Deviation</th><th title="Extension (+, back of the hand up) or flexion (−)">Extension</th><th title="Forearm rotation: 90° = palm flat down, 0° = thumb up">Pronation</th><th title="Mean G03 over the hand's keys: the resting posture plus sideways reaches">Posture cost</th></tr></thead>
        <tbody>
          {(["L", "R"] as const).map((side, h) => costs.placed[h] && (
            <tr key={side}>
              <td>{side === "L" ? "Left" : "Right"}</td>
              <td className="num">{deg(costs.posture[h].deviation)} {costs.posture[h].deviation >= 0 ? "ulnar" : "radial"}</td>
              <td className="num">{deg(costs.posture[h].extension)}</td>
              <td className="num">{deg(costs.posture[h].pronation)}</td>
              <td className="num">{sideMean(side, costs.wrist).toFixed(2)}</td>
            </tr>
          ))}
        </tbody>
      </table>
      <div className="hint">
        Mean reach over all {all.length} keys {avg.toFixed(2)}{costs.placed.includes(false) ? " · a hand without four home keys is not placed (its keys cost 0)" : ""}.
        Reach = joint deviation from the natural curl + sideways splay + fingertip shortfall (hand fixed on the home keys); 0 at rest, about 1 for a comfortable off-home key.
        Posture: forearms from elbows {bodyText} apart; split halves are assumed turned to line up with the forearm (no resting deviation), a tent lowers pronation. Neutral is 0° deviation and extension, 30° pronation.
        The layout's G01 / G03 / G05 (per keystroke) are in the Profile tab's Hand model section and in Analysis.
      </div>
    </div>
  );
}

/** One right hand, palm down, turned so the fingers point right (thumb on top), with dimension lines. */
export function Measurements({ d }: { d: HandDims }) {
  const [box, { w, h }] = useSize<HTMLDivElement>();
  const lay = handLayout(d);
  // Hand-local mm → drawing mm: forward (−z) to the right, toward the pinky (+x) down.
  const P = (p: V3): [number, number] => [-p[2], p[0]];
  const fingers = FOUR.map((f, i) => {
    const k = lay.knuckles[i];
    const pts: [number, number][] = [P(k)];
    let z = k[2];
    for (const l of d[f]) { z -= l; pts.push(P([k[0], 0, z])); }
    return pts;
  });
  // Thumb: out from its base toward the index side, bending a little toward the fingers.
  const thumb: [number, number][] = [P(lay.thumb)];
  d.thumb.forEach((l, j) => {
    const a = ((48 - 10 * j) * Math.PI) / 180;
    const [x, y] = thumb[j];
    thumb.push([x + l * Math.cos(a), y - l * Math.sin(a)]);
  });
  const outline = lay.outline.map(P);
  const all = [...fingers.flat(), ...thumb, ...outline];
  const xMax = Math.max(...all.map((p) => p[0])), yMin = Math.min(...all.map((p) => p[1])), yMax = Math.max(...all.map((p) => p[1]));
  const indexTip = fingers[0][3], middleTip = fingers[1][3], pinkyTip = fingers[3][3], thumbTip = thumb[3];
  // Fit: room for the reach circle on the right, dimension lines below, labels left and top.
  const mx0 = -48, mx1 = Math.max(xMax, indexTip[0] + d.reach) + 34, my0 = Math.min(yMin, indexTip[1] - d.reach) - 10, my1 = yMax + 40;
  const s = Math.max(0.1, Math.min((w - 16) / (mx1 - mx0), (h - 12) / (my1 - my0)));
  const ox = (w - (mx1 - mx0) * s) / 2 - mx0 * s, oy = (h - (my1 - my0) * s) / 2 - my0 * s;
  const X = (x: number) => ox + x * s, Y = (y: number) => oy + y * s;
  const els: React.ReactNode[] = [];
  const chain = (pts: [number, number][], lens: number[], key: string, below: boolean) => {
    pts.forEach(([x, y], j) => {
      els.push(<circle key={`${key}j${j}`} cx={X(x)} cy={Y(y)} r={j ? 2 : 2.6} fill={INK} />);
      if (j === 0) return;
      const [px, py] = pts[j - 1];
      els.push(<line key={`${key}s${j}`} x1={X(px)} y1={Y(py)} x2={X(x)} y2={Y(y)} stroke={INK} strokeWidth={1.4} />);
      els.push(<text key={`${key}t${j}`} x={(X(px) + X(x)) / 2} y={(Y(py) + Y(y)) / 2 + (below ? 12 : -5)} textAnchor="middle">{lens[j - 1].toFixed(0)}</text>);
    });
  };
  fingers.forEach((pts, i) => chain(pts, d[FOUR[i]], FOUR[i], false));
  chain(thumb, d.thumb, "thumb", false);
  const kn = fingers.map((f) => f[0]);
  const edge = (d.palmWidth - 0.76 * d.palmWidth) / 2;
  const dimY1 = yMax + 14, dimY2 = yMax + 30;
  const pwX = kn[1][0] - 10;
  return (
    <div className="measure">
      <div ref={box} className="chartfill">
        <svg width={w} height={h}>
          <defs><marker id="arr" viewBox="0 0 6 6" refX="3" refY="3" markerWidth="6" markerHeight="6" orient="auto-start-reverse"><path d="M0,0L6,3L0,6" fill={ACCENT} /></marker></defs>
          <path d={outline.map(([x, y], i) => `${i ? "L" : "M"}${X(x)},${Y(y)}`).join(" ") + " Z"} fill="#f6f6f6" stroke={MUTED} />
          {/* reach around the index tip */}
          <circle cx={X(indexTip[0])} cy={Y(indexTip[1])} r={d.reach * s} fill="none" stroke={ACCENT} strokeDasharray="3 3" opacity={0.6} />
          <text x={X(indexTip[0] + d.reach) + 4} y={Y(indexTip[1]) - 4}>reach {d.reach.toFixed(0)}</text>
          {/* span */}
          <line x1={X(thumbTip[0])} y1={Y(thumbTip[1])} x2={X(pinkyTip[0])} y2={Y(pinkyTip[1])} stroke={HAIR} strokeDasharray="4 3" />
          <text x={X(thumbTip[0]) + 6} y={Y(thumbTip[1]) - 4}><title>Thumb tip to pinky tip, spread flat</title>span {d.span.toFixed(0)}</text>
          {els}
          {/* palm width, across the knuckles */}
          <line x1={X(pwX)} x2={X(pwX)} y1={Y(kn[0][1] - edge)} y2={Y(kn[3][1] + edge)} stroke={ACCENT} markerEnd="url(#arr)" markerStart="url(#arr)" />
          <text className="ink" x={X(pwX) - 6} y={Y((kn[0][1] + kn[3][1]) / 2) + 4} textAnchor="end">palm width {d.palmWidth.toFixed(0)}</text>
          {/* palm length and hand length, wrist crease to the middle knuckle / tip */}
          <line x1={X(0)} x2={X(0)} y1={Y(yMin)} y2={Y(dimY2) + 4} stroke={HAIR} />
          <line x1={X(kn[1][0])} x2={X(0) + (kn[1][0]) * s} y1={Y(kn[1][1])} y2={Y(dimY1) + 4} stroke={HAIR} />
          <line x1={X(middleTip[0])} x2={X(middleTip[0])} y1={Y(middleTip[1])} y2={Y(dimY2) + 4} stroke={HAIR} />
          <line x1={X(0)} x2={X(kn[1][0])} y1={Y(dimY1)} y2={Y(dimY1)} stroke={ACCENT} markerEnd="url(#arr)" markerStart="url(#arr)" />
          <text className="ink" x={X(kn[1][0] / 2)} y={Y(dimY1) - 4} textAnchor="middle">palm length {d.palmLength.toFixed(0)}</text>
          <line x1={X(0)} x2={X(middleTip[0])} y1={Y(dimY2)} y2={Y(dimY2)} stroke={ACCENT} markerEnd="url(#arr)" markerStart="url(#arr)" />
          <text className="ink" x={X(middleTip[0] / 2)} y={Y(dimY2) + 14} textAnchor="middle">hand length {handLength(d).toFixed(0)}</text>
          <text x={X(0) - 6} y={Y((yMin + yMax) / 2)} textAnchor="end">wrist</text>
        </svg>
      </div>
      <div className="measurelist">
        <div className="kv"><span>Hand length</span><span>{handLength(d).toFixed(0)} mm</span></div>
        <div className="kv"><span>Palm</span><span>{d.palmWidth.toFixed(0)} × {d.palmLength.toFixed(0)} mm</span></div>
        {FINGER_NAMES.map((f) => <div key={f} className="kv"><span>{cap(f)}</span><span>{fingerLength(d, f).toFixed(0)} mm</span></div>)}
        <div className="kv"><span title="Knuckle to fingertip, forward, in the natural typing curl (middle finger)">Curled reach</span><span>{naturalTip(d.middle)[0].toFixed(0)} mm · {(naturalTip(d.middle)[0] * MM).toFixed(2)} u</span></div>
        <div className="kv"><span title="Centre to centre, index to pinky knuckles, divided by 3">Knuckle pitch</span><span>{((0.76 * d.palmWidth) / 3).toFixed(1)} mm</span></div>
        <div className="kv"><span>Span · reach</span><span>{d.span.toFixed(0)} · {d.reach.toFixed(0)} mm</span></div>
      </div>
    </div>
  );
}

/** Lanes top → bottom: left pinky … left thumb, right thumb … right pinky. */
const LANES: [Side, FingerName][] = [
  ["L", "pinky"], ["L", "ring"], ["L", "middle"], ["L", "index"], ["L", "thumb"],
  ["R", "thumb"], ["R", "index"], ["R", "middle"], ["R", "ring"], ["R", "pinky"],
];
/** Seconds shown in the timeline; the playhead sits at PLAYHEAD of the width. */
const WINDOW = 2.4;
const PLAYHEAD = 0.65;

/** Per-finger timeline of the typing plan (moves light, presses solid) with a playhead, plus stats. */
export function Typing({ plan, clock, playing }: { plan: Plan | null; clock: MutableRefObject<number>; playing: boolean }) {
  const [box, { w, h }] = useSize<HTMLDivElement>();
  const cv = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const el = cv.current;
    if (!el || !w || !h) return;
    const dpr = window.devicePixelRatio || 1;
    el.width = w * dpr;
    el.height = h * dpr;
    const g = el.getContext("2d")!;
    let raf = 0;
    const draw = () => {
      g.setTransform(dpr, 0, 0, dpr, 0, 0);
      g.clearRect(0, 0, w, h);
      g.font = "11px Inter, -apple-system, Helvetica, Arial, sans-serif";
      const lx = 70, top = 18, lane = (h - top - 16) / LANES.length;
      const now = clock.current;
      const t0 = playing && now >= 0 ? now - PLAYHEAD * WINDOW : -0.1;
      const sx = (t: number) => lx + ((t - t0) / WINDOW) * (w - lx - 4);
      g.textBaseline = "middle";
      LANES.forEach(([side, f], i) => {
        const y = top + i * lane;
        g.fillStyle = i % 2 ? "#fafafa" : "#f3f3f3";
        g.fillRect(lx, y, w - lx, lane);
        g.fillStyle = MUTED;
        g.fillText(`${side} ${f}`, 4, y + lane / 2);
      });
      g.fillStyle = HAIR;
      g.fillRect(lx, top + 5 * lane - 0.5, w - lx, 1);
      if (plan) {
        const period = plan.end + LOOP_GAP;
        for (const shift of [-period, 0, period]) {
          for (const pr of plan.presses) {
            const i = LANES.findIndex(([s, f]) => s === pr.side && f === pr.finger);
            const y = top + i * lane;
            const [a, r, b, c] = [sx(pr.depart + shift), sx(pr.ready + shift), sx(pr.arrive + shift), sx(pr.up + shift)];
            if (c < lx || a > w) continue;
            const col = fingerColor(`${pr.side}${CODE_OF[pr.finger]}` as Finger);
            g.globalAlpha = 0.35;
            g.fillStyle = col;
            g.fillRect(Math.max(a, lx), y + lane * 0.3, Math.max(0, r - Math.max(a, lx)), lane * 0.4);
            g.fillRect(Math.max(r, lx), y + lane * 0.46, Math.max(0, b - Math.max(r, lx)), lane * 0.08);
            if (Number.isFinite(pr.back)) g.fillRect(Math.max(c, lx), y + lane * 0.38, Math.max(0, sx(pr.back + shift) - Math.max(c, lx)), lane * 0.24);
            g.globalAlpha = 1;
            g.fillRect(Math.max(b, lx), y + lane * 0.15, Math.max(1.5, c - Math.max(b, lx)), lane * 0.7);
            g.fillStyle = INK;
            if (b >= lx) g.fillText(pr.ch === " " ? "␣" : pr.ch, c + 3, y + lane / 2);
            if (pr.limit !== "rhythm" && b >= lx) {
              g.fillStyle = pr.limit === "finger" ? ERROR : MUTED;
              g.beginPath();
              g.moveTo(b, y + lane * 0.15); g.lineTo(b - 4, y + 1); g.lineTo(b + 4, y + 1);
              g.fill();
            }
          }
        }
      }
      g.fillStyle = MUTED;
      g.fillText("light: moving · thin: ready over the key · solid: key down · ▼ waited for the same finger (red) or a late start (grey)", lx, 8);
      if (playing && now >= 0) {
        g.fillStyle = ACCENT;
        g.fillRect(sx(now) - 0.75, top, 1.5, h - top - 16);
      }
      g.fillStyle = MUTED;
      g.textAlign = "right";
      g.fillText(`${WINDOW.toFixed(1)} s window`, w - 4, h - 7);
      g.textAlign = "left";
      if (playing) raf = requestAnimationFrame(draw);
    };
    draw();
    return () => cancelAnimationFrame(raf);
  }, [plan, playing, w, h, clock]);
  const st = plan?.stats;
  const pct = (x: number) => `${(100 * x).toFixed(0)}%`;
  const n = plan?.presses.length ?? 0;
  return (
    <div className="typingpanel">
      <div ref={box} className="chartfill"><canvas ref={cv} style={{ width: w, height: h, display: "block" }} /></div>
      {st && (
        <div className="measurelist">
          <div className="kv"><span title="Speed reached with every finger at its natural pace and this parallelism, vs the target: a key waits until its finger can get there">Speed</span><span>{st.wpm.toFixed(0)} of {st.targetWpm} wpm</span></div>
          <div className="kv"><span title="Keys that went down later than the target rhythm because their finger could not get there in time">Late keys</span><span>{st.fingerWaits + st.serialWaits} of {n}</span></div>
          <div className="kv"><span title="Keys whose finger set off before the previous key was released">Started early</span><span>{pct(st.early)}</span></div>
          <div className="kv"><span title="Share of the time with two or more fingers moving, waiting over a key, or pressing at once">Fingers overlap</span><span>{pct(st.overlap)}</span></div>
          <div className="kv"><span title="Waited because the same finger had just pressed its previous key">· same finger</span><span>{st.fingerWaits}</span></div>
          <div className="kv"><span title="Waited because the finger could not set off before the previous key was released (raise Parallel)">· late start</span><span>{st.serialWaits}</span></div>
          <div className="kv"><span>Hand alternation</span><span>{pct(st.alternation)}</span></div>
          <div className="kv"><span title="Fingertip travel per keystroke, including returns home">Travel / key</span><span>{st.travel.toFixed(1)} mm</span></div>
          <div className="kv"><span title="Presses after which the finger goes back to its home key before its next press">Home returns</span><span>{pct(st.homeReturns)}</span></div>
        </div>
      )}
      {st && (
        <div className="measurelist">
          <div className="kv head"><span>Finger</span><span>keys · mm · busy</span></div>
          {LANES.filter(([s, f]) => st.perFinger[fingerKey(s, f)]).map(([s, f]) => {
            const x = st.perFinger[fingerKey(s, f)];
            return <div key={s + f} className="kv"><span>{s} {f}</span><span>{x.presses} · {x.travel.toFixed(0)} · {pct(x.busy)}</span></div>;
          })}
        </div>
      )}
    </div>
  );
}

export function SizeDistribution({ d }: { d: HandDims }) {
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

export function Similarity({ d }: { d: HandDims }) {
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
