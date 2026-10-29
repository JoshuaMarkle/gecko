// Scene 5–6: the top view becomes the product's 2D drawing (white keys, exactly over the 3D board),
// the keys turn into numbers (finger fills wipe in, pose labels cascade), then the layout search
// plays back (legends change in place): the board rises and a stat strip under it counts each snapshot's real values (no
// leaders: the stats describe the whole layout, not one key). The score lands and turns blue; then
// the camera dives into the home row.

import { AbsoluteFill } from "remotion";
import { data, type Stats } from "../lib/data";
import { clamp01, EASE, interp, lerp, progress, sec } from "../lib/motion";
import { C, H, MONO, W, WEIGHT } from "../lib/tokens";
import { Board2D, BOARD_U, keyCentre } from "../ui/Board2D";
import { useLayout } from "./common";

const TRACE = data.layout.trace;
/** px per key unit that matches the opening's orthographic top view (640 mm up, 30° fov). */
export const TOP_U = (19.05 * 1080) / (2 * 640 * Math.tan((15 * Math.PI) / 180));

/** The stat strip: stat, label, decimals, unit. */
const STATS: { k: keyof Stats; label: string; d: number; unit: string }[] = [
  { k: "score", label: "Score", d: 2, unit: "" },
  { k: "sfb", label: "Same finger", d: 2, unit: "%" },
  { k: "rolls", label: "Rolls", d: 1, unit: "%" },
  { k: "alternate", label: "Alternation", d: 1, unit: "%" },
  { k: "pinky_ring", label: "Pinky–ring", d: 2, unit: "%" },
];

/** The fills and pose numbers slide across (wipe → labels) and straight back (unwipe) in one motion. */
const T = { grow: 8, wipe: 26, labels: 28, unwipe: 58, rise: 66, panel: 78, play: 100, settle: sec(5.9), dive: sec(7.1) };
const BOARD_PX = 74;
/** Board centre once it has risen, the stat strip's top and cell widths (px). */
const BOARD_Y = 400, STRIP_Y = 690, CELL = [430, 300, 300, 300, 300];
/** The camera dives into the gap between F and G (left index home row): the two caps become the tunnel walls. */
const DIVE_KEYS = [16, 17] as const;

export function Optimize() {
  const { frame } = useLayout();
  const u = lerp(TOP_U, BOARD_PX, progress(frame, T.grow, 30, EASE.inOut));
  const bw = BOARD_U.w * u, bh = BOARD_U.h * u;
  const rise = progress(frame, T.rise, 36, EASE.inOut);
  const bx = (W - bw) / 2, by = lerp(H / 2, BOARD_Y, rise) - bh / 2;
  // Trace position: the log-spaced snapshots already favour the early chaos; ease in, then run.
  const pos = interp(frame, [[T.play, 0], [T.play + 50, 0.22], [T.settle, 1]], EASE.inOut) * (TRACE.length - 1);
  const i = Math.min(TRACE.length - 2, Math.floor(pos));
  const prev = TRACE[i], cur = TRACE[i + 1], t = clamp01(pos - i);
  const playing = frame >= T.play;
  const settled = frame >= T.settle;
  const best = TRACE[TRACE.length - 1];
  const chars = playing ? (settled ? best.chars : cur.chars) : data.layout.start;
  const stats: Stats = playing ? (settled ? best.stats : cur.stats) : TRACE[0].stats;
  const wipe = frame < T.unwipe ? progress(frame, T.wipe, 26, EASE.inOut) : 1 - progress(frame, T.unwipe, 20, EASE.inOut);
  const labels = frame < T.unwipe ? progress(frame, T.labels, 26, EASE.linear) : 1 - progress(frame, T.unwipe - 2, 18, EASE.linear);
  // The dive: the strip leaves, then the board scales up about the dive key and the frame goes white.
  const colOut = progress(frame, T.dive - 14, 14, EASE.in);
  const dive = progress(frame, T.dive, sec(0.9), EASE.in);
  const [a, b] = DIVE_KEYS.map((k) => keyCentre(k, u));
  const dkx = (a[0] + b[0]) / 2, dky = (a[1] + b[1]) / 2;
  const zoom = Math.pow(120, dive);
  const out = progress(frame, T.dive + sec(0.6), 8, EASE.linear);
  const fadeIn = progress(frame, 0, 8, EASE.linear);
  const ref = data.layout.qwerty.score;
  const cell = (k: number) => progress(frame, T.panel + k * 5, 24, EASE.out) * (1 - colOut);
  return (
    <AbsoluteFill style={{ background: C.canvas, opacity: fadeIn * (1 - out) }}>
      <AbsoluteFill style={{ transform: `scale(${zoom})`, transformOrigin: `${bx + dkx}px ${by + dky}px`, filter: dive > 0.05 ? `blur(${dive * 6}px)` : undefined }}>
        <div style={{ position: "absolute", left: bx, top: by }}>
          <Board2D u={u} chars={chars} swap={playing && !settled ? { from: prev.chars, t } : undefined} wipe={wipe} labels={labels} accent={null} />
        </div>
      </AbsoluteFill>
      <div style={{ position: "absolute", left: (W - CELL.reduce((x, y) => x + y)) / 2, top: STRIP_Y, display: "flex" }}>
        {STATS.map((r, n) => {
          const p = cell(n), blue = r.k === "score" && settled;
          return (
            <div key={r.k} style={{ width: CELL[n], padding: "0 36px", boxSizing: "border-box", borderLeft: n ? `2px solid ${C.hair}` : undefined,
              opacity: p, transform: `translateY(${(1 - p) * 18}px)` }}>
              <div style={{ fontSize: 28, color: C.muted }}>{r.label}</div>
              <div style={{ fontSize: n ? 64 : 112, fontWeight: WEIGHT.head, lineHeight: 1, marginTop: n ? 26 : 10, color: blue ? C.accent : C.ink }}>
                {stats[r.k].toFixed(r.d)}<span style={{ fontSize: n ? 28 : 40, fontWeight: WEIGHT.light, color: C.muted, marginLeft: 6 }}>{r.unit}</span>
              </div>
              {n === 0 && <div style={{ fontFamily: MONO, fontSize: 24, color: C.muted, marginTop: 14 }}>QWERTY {ref.toFixed(2)}</div>}
            </div>
          );
        })}
      </div>
      <div style={{ position: "absolute", right: 72, bottom: 56, fontFamily: MONO, fontSize: 24, color: C.muted, opacity: progress(frame, T.play, 12) * (1 - colOut) }}>
        {`${(cur.evals / 1e6).toFixed(1).padStart(4, " ")}M layouts scored`}
      </div>
    </AbsoluteFill>
  );
}
