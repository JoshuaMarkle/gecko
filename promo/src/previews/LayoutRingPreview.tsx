// Preview: scene 4 "layout-ring": the SA trace swapping legends on the 2D board while six ring
// gauges count each snapshot's real stats; the score lands on the best and turns accent.

import { data, type Stats } from "../lib/data";
import { EASE, interp, progress, sec } from "../lib/motion";
import { C, TYPE } from "../lib/tokens";
import { Stage, useLayout } from "../scenes/common";
import { Board2D, BOARD_U, keyCentre } from "../ui/Board2D";
import { StatRing } from "../ui/numbers";
import { Eyebrow, KineticHeadline } from "../ui/type";

const TRACE = data.layout.trace;
const U_PX = 64;
/** Rings around the board: stat, label, unit, max, position (px from frame centre), arrow target key. */
const RINGS: { k: keyof Stats; label: string; unit: string; max: number; at: [number, number]; key: number; d: number }[] = [
  { k: "score", label: "Score", unit: "", max: 50, at: [-700, -250], key: 0, d: 2 },
  { k: "sfb", label: "Same finger", unit: "%", max: 5, at: [0, -390], key: 3, d: 2 },
  { k: "rolls", label: "Rolls", unit: "%", max: 30, at: [700, -250], key: 9, d: 1 },
  { k: "alternate", label: "Alternation", unit: "%", max: 30, at: [-700, 250], key: 37, d: 1 },
  { k: "pinky_ring", label: "Pinky–ring", unit: "%", max: 2, at: [0, 390], key: 33, d: 2 },
  { k: "reach", label: "Reach", unit: "", max: 2.5, at: [700, 250], key: 35, d: 2 },
];

export function LayoutRingPreview() {
  const { frame, width, height } = useLayout();
  // Trace position: slow at first (log-spaced data already favours the early chaos), then faster.
  const pos = interp(frame, [[sec(0.5), 0], [sec(2.5), 0.25], [sec(4.5), 1]], EASE.inOut) * (TRACE.length - 1);
  // Between snapshots i and i + 1: legends travel from i's layout to i + 1's; stats show the target.
  const i = Math.min(TRACE.length - 2, Math.floor(pos));
  const prev = TRACE[i], cur = TRACE[i + 1], t = Math.min(1, pos - i);
  const settled = frame > sec(4.6);
  const bw = BOARD_U.w * U_PX, bh = BOARD_U.h * U_PX;
  const bx = (width - bw) / 2, by = (height - bh) / 2;
  return (
    <Stage>
      <div style={{ position: "absolute", left: bx, top: by }}>
        <Board2D u={U_PX} chars={cur.chars} swap={{ from: prev.chars, t }} accent={null} />
      </div>
      {RINGS.map((r, n) => {
        const [kx, ky] = keyCentre(r.key, U_PX);
        const cx = width / 2 + r.at[0], cy = height / 2 + r.at[1];
        const reveal = progress(frame, sec(0.1) + n * 3, 20);
        const best = r.k === "score" && settled;
        return (
          <div key={r.k} style={{ position: "absolute", left: cx - 100, top: cy - 100 }}>
            <StatRing value={cur.stats[r.k]} max={r.max} label={r.label} unit={r.unit} decimals={r.d} size={200} reveal={reveal} accent={best}
              arrow={[bx + kx - cx, by + ky - cy]} lead={n % 2 ? 0.25 : 0.6} />
          </div>
        );
      })}
      <div style={{ position: "absolute", left: 120, bottom: 72 }}>
        <Eyebrow text="Optimize." start={sec(0.3)} />
        <div style={{ height: 12 }} />
        <KineticHeadline text="Every swap, scored." start={sec(0.6)} size={TYPE.headline * 0.7} />
      </div>
      <div style={{ position: "absolute", right: 120, bottom: 80, fontSize: TYPE.hint, color: C.muted, fontFamily: "JetBrains Mono" }}>
        {`${data.layout.algo.toUpperCase()} · ${(cur.evals / 1e6).toFixed(1)}M evals · best ${cur.best.toFixed(2)}`}
      </div>
    </Stage>
  );
}
