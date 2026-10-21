// 2D boards (frontend.md "2D and 3D board representations"): the flat drawing of a geometry
// (`geometry.flat` via `flatKeys`) with each key's rotation. Used for board cards, thumbnails,
// the layout editor, heatmaps and per-key overlays.

import { useLayoutEffect, useRef, useState, type ReactNode } from "react";
import type { Geometry } from "../api";
import { ACCENT, HAIR } from "../colors";
import { flatBounds, flatKeys } from "../format";

export interface Cell { fill: string; text: string; main: string; sub?: string; title?: string; ring?: boolean; dim?: boolean; mark?: string }
/** A curve between the centres of keys `i` and `j`; `w` in [0, 1] sets width and opacity. */
export interface KeyArc { i: number; j: number; color: string; w: number }

/** Keys at their geometry positions, `u` px per key unit. */
export function KeyGrid({ geometry, cells, u, onKey, arcs, children }: {
  geometry: Geometry; cells: (slot: number) => Cell; u: number; onKey?: (slot: number) => void; arcs?: KeyArc[]; children?: ReactNode;
}) {
  const ks = flatKeys(geometry);
  const b = flatBounds(ks);
  const w = (b.x1 - b.x0) * u + 4, h = (b.y1 - b.y0) * u + 4;
  const small = u < 30;
  const centre = (i: number): [number, number] => [2 + (ks[i].x - b.x0 + ks[i].w / 2) * u, 2 + (ks[i].y - b.y0 + ks[i].h / 2) * u];
  return (
    <svg width={w} height={h} className="keygrid">
      {ks.map((k, i) => {
        const c = cells(i);
        const x = 2 + (k.x - b.x0) * u, y = 2 + (k.y - b.y0) * u;
        const kw = k.w * u - (small ? 2 : 4), kh = k.h * u - (small ? 2 : 4);
        return (
          <g key={i} onClick={onKey ? () => onKey(i) : undefined} style={{ cursor: onKey ? "pointer" : undefined, opacity: c.dim ? 0.4 : 1 }}
            transform={k.rot ? `rotate(${k.rot} ${x + 1 + kw / 2} ${y + 1 + kh / 2})` : undefined}>
            {c.title && <title>{c.title}</title>}
            <rect x={x + 1} y={y + 1} width={kw} height={kh} rx={small ? 2 : 6} fill={c.fill} stroke={c.ring ? ACCENT : HAIR} strokeWidth={c.ring ? 2 : 1} />
            <text x={x + 1 + kw / 2} y={y + 1 + (c.sub && !small ? kh * 0.4 : kh / 2) + (small ? 3.5 : 4)} textAnchor="middle"
              style={{ fill: c.text, fontWeight: 600, fontSize: small ? 10 : Math.max(11, u * 0.3), fontFamily: small ? "var(--mono)" : undefined }}>{c.main}</text>
            {c.sub && !small && <text x={x + 1 + kw / 2} y={y + kh - 3} textAnchor="middle" style={{ fill: c.text, fontSize: 9, opacity: 0.8 }}>{c.sub}</text>}
            {c.mark && <text x={x + kw - 2} y={y + (small ? 8 : 12)} textAnchor="end" style={{ fill: c.text, fontSize: small ? 7 : 9 }}>{c.mark}</text>}
          </g>
        );
      })}
      {arcs?.map((a, n) => {
        const [p, q] = [centre(a.i), centre(a.j)];
        const dist = Math.hypot(q[0] - p[0], q[1] - p[1]);
        const mx = (p[0] + q[0]) / 2, my = (p[1] + q[1]) / 2 - Math.max(10, dist * 0.35);
        return <path key={n} d={`M${p[0]},${p[1]} Q${mx},${my} ${q[0]},${q[1]}`} fill="none" stroke={a.color} strokeWidth={0.8 + 4 * a.w}
          strokeOpacity={0.25 + 0.75 * a.w} strokeLinecap="round" pointerEvents="none" />;
      })}
      {children}
    </svg>
  );
}

/** A 2D board scaled to fill its container (at most `max` px per key unit). */
export function FitBoard({ geometry, cells, onKey, arcs, max = 64, pad = 8 }: {
  geometry: Geometry; cells: (slot: number) => Cell; onKey?: (slot: number) => void; arcs?: KeyArc[]; max?: number; pad?: number;
}) {
  const box = useRef<HTMLDivElement>(null);
  const [{ w, h }, setSize] = useState({ w: 0, h: 0 });
  useLayoutEffect(() => {
    const el = box.current;
    if (!el) return;
    const ro = new ResizeObserver(([e]) => setSize({ w: e.contentRect.width, h: e.contentRect.height }));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  const b = flatBounds(flatKeys(geometry));
  const u = Math.max(2, Math.min((w - 2 * pad) / (b.x1 - b.x0), (h - 2 * pad) / (b.y1 - b.y0), max));
  return (
    <div ref={box} className="fitboard">
      {w > 0 && <KeyGrid geometry={geometry} cells={cells} u={u} onKey={onKey} arcs={arcs} />}
    </div>
  );
}

/** A layout as a small key grid (symbols only); keys that differ from `diff` are shaded. */
export function Thumb({ geometry, chars, diff, u = 17 }: { geometry: Geometry; chars: string; diff?: string; u?: number }) {
  const a = [...chars], d = diff ? [...diff] : null;
  return (
    <KeyGrid geometry={geometry} u={u} cells={(i) => ({
      fill: d && d[i] !== a[i] ? "#e8e8e8" : "#ffffff",
      text: "#141414",
      main: (a[i] ?? "").toUpperCase(),
    })} />
  );
}
