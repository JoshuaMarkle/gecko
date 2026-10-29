// Scene 8: the text becomes data. A word processor holds the opening of Pride and Prejudice. The page
// tilts (right edge toward the camera) with its text on it, and the text stays on it; the
// first lines unravel character by character into one long line that the camera follows, larger.
// The line runs on through a loop (still text, glitching through the corpus's symbols), comes out as
// data (one tick per letter, its height the letter's frequency) and flows along the baseline into a
// letter-frequency chart, whose bars grow as each tick enters.

import { AbsoluteFill } from "remotion";
import { data } from "../lib/data";
import { clamp01, EASE, interp, lerp, progress } from "../lib/motion";
import { hash01 } from "../lib/rng";
import { C, H, MONO, W, WEIGHT } from "../lib/tokens";
import { CHROME_H, DOC_CANVAS, DocChrome } from "../ui/Document";
import { Space3D } from "../ui/space";
import { useLayout } from "./common";

/** The page (px at 1:1): size, top on screen, margin; body type. */
const DOC = { w: 780, h: 1010, top: CHROME_H + 30, margin: 96, size: 18, lh: 28, gap: 12 } as const;
const TITLE = "Pride and Prejudice";
/** Letters in corpus-frequency order (the chart's columns) and their shares. */
const LETTERS = (data.language.corpus.symbols_per_mille as { symbol: string; share: number }[]).filter((s) => /^[a-z]$/.test(s.symbol)).slice(0, 26);
const TOP_SHARE = Math.max(...LETTERS.map((l) => l.share));
const COL = new Map(LETTERS.map((l, i) => [l.symbol, i] as const));
const NOISE = "abcdefghijklmnopqrstuvwxyz;,./'-0123456789()[]{}<>=+*&|!?:_#$%^~@";

interface Line { text: string; xs: number[]; w: number; y: number; size: number; weight: number; color: string; body: number }

/** The page's lines (page-local px), measured on a canvas in the page's fonts; cached once the fonts are in. */
let cache: Line[] | null = null;
function pageLines(): Line[] {
  if (cache) return cache;
  const g = document.createElement("canvas").getContext("2d")!;
  const ready = document.fonts.check(`400 ${DOC.size}px Inter`);
  const out: Line[] = [];
  const maxW = DOC.w - 2 * DOC.margin;
  let y = DOC.margin, body = 0;
  const push = (text: string, size: number, weight: number, color: string, lh: number, isBody: boolean) => {
    g.font = `${weight} ${size}px Inter`;
    const xs = [...text].map((_, j) => g.measureText([...text].slice(0, j).join("")).width);
    out.push({ text, xs, w: g.measureText(text).width, y, size, weight, color, body: isBody ? body++ : -1 });
    y += lh;
  };
  push(TITLE, 44, WEIGHT.head, C.ink, 62, false);
  push("Jane Austen", 22, WEIGHT.text, C.muted, 52, false);
  push("Chapter I", 24, WEIGHT.head, C.ink, 44, false);
  g.font = `400 ${DOC.size}px Inter`;
  for (const para of data.language.chapter) {
    let line = "";
    for (const word of para.split(" ")) {
      const next = line ? `${line} ${word}` : word;
      if (g.measureText(next).width > maxW && line) { push(line, DOC.size, 400, C.ink, DOC.lh, true); g.font = `400 ${DOC.size}px Inter`; line = word; }
      else line = next;
    }
    if (line) push(line, DOC.size, 400, C.ink, DOC.lh, true);
    g.font = `400 ${DOC.size}px Inter`;
    y += DOC.gap;
    if (y > DOC.h - DOC.margin) break;
  }
  const fit = out.filter((l) => l.y + l.size < DOC.h - DOC.margin + 10);
  if (ready) cache = fit;
  return fit;
}

/** Timing (scene frames). The chained lines' glyphs leave one after another over CHAIN frames (glyph
 * fraction c 0..1 leaves at DEPART(c)) and dock FLY frames later. */
const N = 4, FLY = 16, CHAIN = 110;
const T = { hold: 16, chromeOut: 64, tilt: 66, chain: 146, flow: 0, flowDur: 250, out: 0 };
const DEPART = (c: number) => T.chain + CHAIN * c;
T.flow = DEPART(1) + FLY + 6;
T.out = T.flow + T.flowDur + 32;
/** Scene length (s) for the timeline. */
export const LANGUAGE_S = (T.out + 14) / 30;

/** The chain: scale, plane depth (world px), y. The path runs along it, through a loop, to the chart. */
const S = 2, ZC = 140, YC = 0, R = 300, DRIFT = 180;
/** A data tick's foot below the line's top (glyph px, before the chain's scale). */
const TICK_BASE = DOC.size * 1.05;
const TILT = -26;

/** Polyline of the path after the chain: the loop (rises, turns over, comes down a little ahead), then straight to the chart. */
function pathGeometry(lines: Line[]) {
  const chain = lines.filter((l) => l.body >= 0 && l.body < N);
  const space = DOC.size * 0.45;
  const off: number[] = [];
  let acc = 0;
  for (const l of chain) { off.push(acc); acc += (l.w + space) * S; }
  const Lc = acc;
  const X0 = -(chain[0].w * S) / 2;
  const loopS = Lc + 300;
  const XL = X0 + loopS;
  const pts: [number, number][] = [];
  for (let i = 0; i <= 240; i++) {
    const a = (i / 240) * Math.PI * 2;
    pts.push([XL + R * Math.sin(a) + (DRIFT * a) / (Math.PI * 2), YC - R + R * Math.cos(a)]);
  }
  const lens = [0];
  for (let i = 1; i < pts.length; i++) lens.push(lens[i - 1] + Math.hypot(pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]));
  const loopEnd = loopS + lens[lens.length - 1];
  const entry = loopEnd + 420;
  // The chart's baseline is the ticks' baseline: they run along it into the chart.
  const chart = { x: XL + DRIFT + 420, base: YC + TICK_BASE * S, h: 480, col: 40 };
  return { chain, off, Lc, X0, loopS, XL, pts, lens, loopEnd, entry, chart };
}

/** World point and tangent angle (deg) at path length s. */
function at(G: ReturnType<typeof pathGeometry>, s: number): [number, number, number] {
  if (s < G.loopS) return [G.X0 + s, YC, 0];
  if (s < G.loopEnd) {
    const d = s - G.loopS;
    let i = 1;
    while (i < G.lens.length - 1 && G.lens[i] < d) i++;
    const f = (d - G.lens[i - 1]) / Math.max(1e-6, G.lens[i] - G.lens[i - 1]);
    const [ax, ay] = G.pts[i - 1], [bx, by] = G.pts[i];
    return [lerp(ax, bx, f), lerp(ay, by, f), (Math.atan2(by - ay, bx - ax) * 180) / Math.PI];
  }
  return [G.XL + DRIFT + (s - G.loopEnd), YC, 0];
}

/** Path length over which a tick fades into the chart's left edge (before, after the entry). */
const ABSORB = [70, 10] as const;

export function Language() {
  const { frame } = useLayout();
  const lines = pageLines();
  const G = pathGeometry(lines);
  const pageTop = DOC.top - H / 2; // world y of the page's top edge at 1:1
  // Page pose: flat, then tilted with its right edge toward the camera, set back a little.
  const tilt = progress(frame, T.tilt, 44, EASE.inOut);
  const theta = TILT * tilt, rad = (theta * Math.PI) / 180;
  // The chained glyphs in reading order: glyph c (0..nChain−1) is line k's character j.
  const nChain = G.chain.reduce((n, l) => n + l.text.length, 0);
  const firstOf = G.chain.map((_, k) => G.chain.slice(0, k).reduce((n, l) => n + l.text.length, 0));
  // The glyph leaving at frame f, continuous: line k, fractional character j (the x of a fractional
  // character interpolates its neighbours' x, so the page and camera targets move without steps).
  const glyphAt = (f: number) => {
    const c = clamp01((f - T.chain) / CHAIN) * (nChain - 1);
    const k = Math.max(0, firstOf.findLastIndex((n) => n <= c));
    const j = Math.min(G.chain[k].text.length - 1, c - firstOf[k]), j0 = Math.floor(j);
    const xs = G.chain[k].xs;
    return { k, x: lerp(xs[j0], xs[Math.min(xs.length - 1, j0 + 1)], j - j0) };
  };
  // The page slides right ahead of the chain's end, so each glyph flies left from the page to the end
  // of the line: they leave and land in reading order (a fixed page puts later lines' glyphs ever
  // farther from their slots, and they overtake one another). The ideal slide steps a line's width at
  // each line break; a Hann window two lines wide spreads the steps into a steady glide (its
  // overlapping halves sum to a constant), where a box filter left a lurch at every break.
  const need = (f: number) => { const { k, x } = glyphAt(f); return G.off[k] + x * (S - Math.cos((TILT * Math.PI) / 180)); };
  const lineDur = CHAIN / G.chain.length;
  let slide = 0, wsum = 0;
  for (let d = -lineDur; d <= lineDur; d += 0.5) {
    const w = 1 + Math.cos((Math.PI * d) / lineDur);
    slide += w * need(frame + d + FLY / 2); wsum += w;
  }
  slide /= wsum;
  const pc = { x: lerp(0, -30, tilt) + slide, y: lerp(pageTop + DOC.h / 2, 40, tilt), z: lerp(0, -380, tilt) };
  const chrome = 1 - progress(frame, T.chromeOut, 26, EASE.inOut);
  const pageFade = 1 - progress(frame, T.flow - 50, 50, EASE.inOut);
  // Where each line sits: on the (tilted) page (a hair in front of it, so the sheet never covers it),
  // or, glyph by glyph, flying to / docked on the chain.
  const onPage = (l: Line) => {
    const lift = 0.5;
    const lx = -DOC.w / 2 + DOC.margin, ly = l.y - DOC.h / 2;
    return { x: pc.x + lx * Math.cos(rad) + lift * Math.sin(rad), y: pc.y + ly, z: pc.z - lx * Math.sin(rad) + lift * Math.cos(rad), ry: theta, sc: 1 };
  };
  // A chained glyph: on the page (dx along the line), then flying to its place on the chain.
  const glyphPose = (l: Line, j: number) => {
    const p = onPage(l), dx = l.xs[j];
    const pg = { x: p.x + dx * Math.cos(rad), y: p.y, z: p.z - dx * Math.sin(rad) };
    const k = l.body;
    // Mostly linear, so glyphs keep their spacing in flight instead of bunching where they land.
    const x = clamp01((frame - DEPART((firstOf[k] + j) / Math.max(1, nChain - 1))) / FLY), f = lerp(x, EASE.inOut(x), 0.35);
    const q = { x: G.X0 + G.off[k] + dx * S, y: YC, z: ZC };
    return { x: lerp(pg.x, q.x, f), y: lerp(pg.y, q.y, f), z: lerp(pg.z, q.z, f) + Math.sin(Math.PI * f) * 110, ry: lerp(theta, 0, f), sc: lerp(1, S, f) };
  };
  // Flow: every chained glyph moves along the path by `shift`.
  const flowing = frame >= T.flow;
  const total = G.entry + ABSORB[1] + 60;
  const shift = total * EASE.inOut(clamp01((frame - T.flow) / T.flowDur));
  const head = G.Lc + shift;
  // Camera: still on the page; dolly in as the first glyphs dock; follow the chain's docking end
  // (averaged over a second, so it glides); then follow the head into the loop and pull back to frame
  // the loop and the chart.
  const dockX = (f: number) => { const { k, x } = glyphAt(f - FLY); return G.X0 + G.off[k] + x * S; };
  // Framed between the docking end and the glyph leaving the page (its slot plus the page's lead).
  const lead = -30 + (-DOC.w / 2 + DOC.margin) * Math.cos((TILT * Math.PI) / 180) - G.X0;
  let camX = 0;
  for (let d = -15; d <= 15; d++) camX += Math.max(0, (dockX(frame + d) + dockX(frame + d + FLY) + lead) / 2 + 80) / 31;
  let cam = { x: camX, y: interp(frame, [[DEPART(0), 0], [DEPART(0) + FLY, YC + 20]]), z: interp(frame, [[DEPART(0), 0], [DEPART(0) + FLY + 10, -200]]) };
  if (flowing) {
    const [hx, hy] = at(G, Math.min(head, G.loopS + 200));
    const follow = { x: hx - 420, y: lerp(YC + 20, hy, 0.4), z: -200 - 100 * progress(frame, T.flow, 40, EASE.inOut) };
    const wide = { x: (G.XL - R + G.chart.x + 26 * G.chart.col) / 2, y: YC - 150, z: 560 };
    const w = progress(frame, T.flow + 40, 90, EASE.inOut);
    const settle = { x: G.chart.x + 13 * G.chart.col, y: G.chart.base - G.chart.h / 2 - 10, z: 330 };
    const v = progress(frame, T.flow + T.flowDur - 80, 90, EASE.inOut);
    cam = { x: lerp(lerp(lerp(cam.x, follow.x, progress(frame, T.flow, 20, EASE.inOut)), wide.x, w), settle.x, v),
      y: lerp(lerp(follow.y, wide.y, w), settle.y, v), z: lerp(lerp(follow.z, wide.z, w), settle.z, v) };
  }
  // Chart counts: each letter tick adds to its column as it fades into the chart.
  const counts = new Array(LETTERS.length).fill(0);
  const glyphs: { ch: string; s: number; li: number }[] = [];
  if (flowing) G.chain.forEach((l, k) => [...l.text, " "].forEach((ch, j) => {
    const s0 = G.off[k] + (j < l.xs.length ? l.xs[j] : l.w) * S;
    const s = s0 + shift;
    if (ch !== " ") glyphs.push({ ch, s, li: k });
    const c = COL.get(ch.toLowerCase());
    if (c !== undefined) counts[c] += EASE.inOut(clamp01((s - G.entry + ABSORB[0]) / (ABSORB[0] + ABSORB[1])));
  }));
  const maxCount = Math.max(4, ...counts);
  const landed = counts.reduce((a, b) => a + b, 0);
  const chartIn = progress(frame, T.flow + 40, 40, EASE.out);
  const out = progress(frame, T.out, 12, EASE.linear);
  const tick = Math.floor(frame / 2);
  return (
    <AbsoluteFill style={{ background: C.canvas, opacity: progress(frame, 0, 14, EASE.linear) * (1 - out) }}>
      <AbsoluteFill style={{ background: DOC_CANVAS, opacity: chrome }} />
      <Space3D cam={{ ...cam, perspective: 1400 }}>
        {/* the page sheet */}
        <div style={{ position: "absolute", left: 0, top: 0, transformStyle: "preserve-3d", opacity: pageFade,
          transform: `translate3d(${pc.x}px, ${pc.y}px, ${pc.z}px) rotateY(${theta}deg)` }}>
          <div style={{ width: DOC.w, height: DOC.h, transform: "translate(-50%, -50%)", background: C.canvas,
            boxShadow: `0 0 0 1px ${C.hair}, 0 ${6 + 20 * tilt}px ${18 + 40 * tilt}px rgba(0,0,0,${0.08 + 0.06 * tilt})` }} />
        </div>
        {/* the lines; chained ones glyph by glyph until the flow takes over */}
        {lines.map((l, i) => {
          const chained = l.body >= 0 && l.body < N;
          if (chained && flowing) return null;
          if (chained) return [...l.text].map((ch, j) => {
            if (ch === " ") return null;
            const p = glyphPose(l, j);
            return (
              <div key={`${i}-${j}`} style={{ position: "absolute", left: 0, top: 0, transformOrigin: "0 0", whiteSpace: "pre",
                transform: `translate3d(${p.x}px, ${p.y}px, ${p.z}px) rotateY(${p.ry}deg) scale(${p.sc})`,
                fontSize: l.size, fontWeight: l.weight, color: l.color, lineHeight: `${l.size * 1.3}px`, height: l.size * 1.3 }}>{ch}</div>
            );
          });
          const p = onPage(l);
          const op = pageFade;
          if (op <= 0.001) return null;
          return (
            <div key={i} style={{ position: "absolute", left: 0, top: 0, transformOrigin: "0 0", opacity: op, whiteSpace: "pre",
              transform: `translate3d(${p.x}px, ${p.y}px, ${p.z}px) rotateY(${p.ry}deg) scale(${p.sc})`,
              fontSize: l.size, fontWeight: l.weight, color: l.color, lineHeight: `${l.size * 1.3}px`, height: l.size * 1.3 }}>
              {[...l.text].map((ch, j) => <span key={j} style={{ position: "absolute", left: l.xs[j], top: 0 }}>{ch}</span>)}
              {i === 0 && frame < T.chromeOut && Math.floor(frame / 15) % 2 === 0 && (
                <span style={{ position: "absolute", left: l.w + 6, top: 4, width: 3, height: l.size * 1.1, background: C.ink }} />
              )}
            </div>
          );
        })}
        {/* the flow: text, then glitching through the loop, then data ticks running into the chart */}
        {glyphs.map((g, n) => {
          if (g.s > G.entry + ABSORB[1]) return null;
          const c = COL.get(g.ch.toLowerCase());
          const share = c === undefined ? 0 : LETTERS[c].share / TOP_SHARE;
          const absorb = 1 - clamp01((g.s - G.entry + ABSORB[0]) / (ABSORB[0] + ABSORB[1]));
          const [x, y, ang] = at(G, g.s);
          const glitch = clamp01((g.s - (G.loopS - 160)) / 520);
          const data_ = clamp01((g.s - G.loopEnd) / 220);
          const noisy = data_ < 1 && hash01(9, n) < glitch * 0.65;
          const ch = noisy ? NOISE[Math.floor(hash01(tick, n) * NOISE.length)] : g.ch;
          return (
            <div key={n} style={{ position: "absolute", left: 0, top: 0, transformOrigin: "0 0", whiteSpace: "pre", opacity: absorb,
              transform: `translate3d(${x}px, ${y}px, ${ZC}px) rotate(${ang}deg) scale(${S})`, fontSize: DOC.size, lineHeight: `${DOC.size * 1.3}px` }}>
              {data_ < 1 && <span style={{ position: "absolute", left: 0, top: 0, opacity: 1 - data_, color: noisy ? C.accent : C.ink, fontFamily: noisy ? MONO : undefined }}>{ch}</span>}
              {data_ > 0 && c !== undefined && (
                <div style={{ position: "absolute", left: 3, top: TICK_BASE - (8 + 46 * share) * data_, width: 4, height: (8 + 46 * share) * data_, background: C.ink }} />
              )}
            </div>
          );
        })}
        {/* the letter-frequency chart */}
        {flowing && chartIn > 0 && (
          <div style={{ position: "absolute", left: 0, top: 0, opacity: chartIn, transform: `translate3d(${G.chart.x}px, ${G.chart.base - G.chart.h - 60}px, ${ZC}px)` }}>
            <svg width={26 * G.chart.col} height={G.chart.h + 130} style={{ overflow: "visible" }}>
              <text x={0} y={20} style={{ fontFamily: MONO, fontSize: 26, fill: C.muted }}>{`${Math.round(landed)} letters`}</text>
              <line x1={0} x2={26 * G.chart.col} y1={G.chart.h + 60} y2={G.chart.h + 60} stroke={C.hair} strokeWidth={2} />
              {LETTERS.map((l, c) => {
                const h = (counts[c] / maxCount) * G.chart.h;
                return (
                  <g key={l.symbol}>
                    <rect x={c * G.chart.col + 5} y={G.chart.h + 60 - h} width={G.chart.col - 10} height={h} rx={3} fill={C.ink} />
                    <text x={c * G.chart.col + G.chart.col / 2} y={G.chart.h + 100} textAnchor="middle" style={{ fontFamily: MONO, fontSize: 26, fill: c < 6 ? C.ink : C.muted }}>{l.symbol}</text>
                  </g>
                );
              })}
            </svg>
          </div>
        )}
      </Space3D>
      <div style={{ position: "absolute", left: 0, top: 0, opacity: chrome, transform: `translateY(${(1 - chrome) * -40}px)` }}>
        <DocChrome title={TITLE} width={W} pageW={DOC.w} margin={DOC.margin} />
      </div>
    </AbsoluteFill>
  );
}
