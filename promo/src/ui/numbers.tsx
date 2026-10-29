// Numbers: the big counter (the product's "big number" at display size) and the ring gauge with a
// thin curved arrow to what it measures. Tabular figures; the best value may turn accent.

import { useCurrentFrame } from "remotion";
import { clamp01, DUR, EASE, lerp, progress } from "../lib/motion";
import { C, MONO, TYPE, WEIGHT } from "../lib/tokens";
import { Trace } from "./trace";

/** A number counting from → to over `count` (0..1); `accent` turns it blue (the best value). */
export function MetricCounter({ from = 0, to, count, decimals = 2, unit, label, size = TYPE.display, accent = false, start = 0, align = "left", format }: {
  from?: number; to: number; count: number; decimals?: number; unit?: string; label?: string; size?: number; accent?: boolean;
  start?: number; align?: "left" | "right" | "center"; format?: (x: number) => string;
}) {
  const frame = useCurrentFrame();
  const p = progress(frame, start, DUR.enter);
  const v = lerp(from, to, clamp01(count));
  return (
    <div style={{ opacity: p, transform: `translateY(${(1 - p) * 16}px)`, textAlign: align }}>
      <div style={{ fontSize: size, fontWeight: WEIGHT.head, lineHeight: 1, color: accent ? C.accent : C.ink, fontVariantNumeric: "tabular-nums" }}>
        {format ? format(v) : v.toFixed(decimals)}
        {unit && <span style={{ fontSize: size * 0.4, fontWeight: WEIGHT.light, color: C.muted, marginLeft: size * 0.08 }}>{unit}</span>}
      </div>
      {label && <div style={{ fontSize: TYPE.body, color: C.muted, marginTop: 12 }}>{label}</div>}
    </div>
  );
}

/**
 * Ring gauge: --soft track, ink arc for `value` within [min, max], the number in the middle,
 * label under. `arrow` draws a copper PCB trace (ui/trace.tsx) from a pad on a point (px, relative
 * to the ring centre) to a via on the ring's edge, revealed with the ring; `lead` sets where it turns.
 */
export function StatRing({ value, min = 0, max, label, unit = "", decimals = 1, size = 220, reveal = 1, accent = false, arrow, lead = 0.4 }: {
  value: number; min?: number; max: number; label: string; unit?: string; decimals?: number; size?: number; reveal?: number; accent?: boolean;
  arrow?: [number, number]; lead?: number;
}) {
  const r = size / 2 - 10, sw = 6;
  const f = clamp01((value - min) / (max - min)) * clamp01(reveal);
  const circ = 2 * Math.PI * r;
  const ink = accent ? C.accent : C.ink;
  const pad = 400; // svg overflow room for the arrow
  let arrowEl = null;
  if (arrow) {
    // The trace leaves the ring radially at the nearest 45° and routes octilinear to a pad on the key.
    const [ax, ay] = arrow;
    const ang = Math.round(Math.atan2(ay, ax) / (Math.PI / 4)) * (Math.PI / 4);
    const start: [number, number] = [Math.cos(ang) * (r + 10), Math.sin(ang) * (r + 10)];
    arrowEl = <Trace from={[ax, ay]} to={start} reveal={EASE.out(clamp01(reveal * 1.4 - 0.4))} lead={lead} />;
  }
  return (
    <div style={{ position: "relative", width: size, height: size, opacity: clamp01(reveal * 3) }}>
      <svg width={size + 2 * pad} height={size + 2 * pad} style={{ position: "absolute", left: -pad, top: -pad, overflow: "visible" }}>
        <g transform={`translate(${pad + size / 2} ${pad + size / 2})`}>
          <circle r={r} fill="none" stroke={C.soft} strokeWidth={sw} />
          <circle r={r} fill="none" stroke={ink} strokeWidth={sw} strokeLinecap="round" strokeDasharray={circ} strokeDashoffset={circ * (1 - f)}
            transform="rotate(-90)" />
          {arrowEl}
        </g>
      </svg>
      <div style={{ position: "absolute", inset: 0, display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center" }}>
        <div style={{ fontSize: size * 0.24, fontWeight: WEIGHT.head, color: ink, lineHeight: 1 }}>
          {value.toFixed(decimals)}<span style={{ fontSize: size * 0.11, fontWeight: WEIGHT.light, color: C.muted }}>{unit}</span>
        </div>
        <div style={{ fontSize: Math.max(TYPE.hint, size * 0.1), color: C.muted, marginTop: 6 }}>{label}</div>
      </div>
    </div>
  );
}

/** Mono key–value line (n-grams, weights, counts). */
export function MonoValue({ k, v, size = TYPE.body }: { k: string; v: string; size?: number }) {
  return (
    <div style={{ display: "flex", justifyContent: "space-between", gap: 24, fontFamily: MONO, fontSize: size }}>
      <span style={{ color: C.muted }}>{k}</span><span>{v}</span>
    </div>
  );
}
