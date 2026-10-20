// 2D boards for Explore: a generic key grid, the consensus keyboard (backbone of the focused
// layouts: modal symbol per key, its share, and how contested the key is) and thumbnails.

import type { Geometry } from "../../api";
import { ACCENT, HAIR, heatColor, heatText } from "../../colors";
import { useSize } from "../Charts";

export interface Cell { fill: string; text: string; main: string; sub?: string; title?: string; ring?: boolean }

function bounds(g: Geometry) {
  const ks = g.key;
  return {
    x0: Math.min(...ks.map((k) => k.pos[0])), x1: Math.max(...ks.map((k) => k.pos[0] + k.size[0])),
    y0: Math.min(...ks.map((k) => k.pos[1])), y1: Math.max(...ks.map((k) => k.pos[1] + k.size[1])),
  };
}

/** Keys at their geometry positions, `u` px per key unit. */
export function KeyGrid({ geometry, cells, u, onKey }: { geometry: Geometry; cells: (slot: number) => Cell; u: number; onKey?: (slot: number) => void }) {
  const b = bounds(geometry);
  const w = (b.x1 - b.x0) * u + 4, h = (b.y1 - b.y0) * u + 4;
  const small = u < 30;
  return (
    <svg width={w} height={h} className="keygrid">
      {geometry.key.map((k, i) => {
        const c = cells(i);
        const x = 2 + (k.pos[0] - b.x0) * u, y = 2 + (k.pos[1] - b.y0) * u;
        const kw = k.size[0] * u - (small ? 2 : 4), kh = k.size[1] * u - (small ? 2 : 4);
        return (
          <g key={i} onClick={onKey ? () => onKey(i) : undefined} style={onKey ? { cursor: "pointer" } : undefined}>
            {c.title && <title>{c.title}</title>}
            <rect x={x + 1} y={y + 1} width={kw} height={kh} rx={small ? 2 : 6} fill={c.fill} stroke={c.ring ? ACCENT : HAIR} strokeWidth={c.ring ? 2 : 1} />
            <text x={x + 1 + kw / 2} y={y + 1 + (c.sub && !small ? kh * 0.4 : kh / 2) + (small ? 3.5 : 4)} textAnchor="middle"
              style={{ fill: c.text, fontWeight: 600, fontSize: small ? 10 : Math.max(11, u * 0.3), fontFamily: small ? "var(--mono)" : undefined }}>{c.main}</text>
            {c.sub && !small && <text x={x + 1 + kw / 2} y={y + kh - 3} textAnchor="middle" style={{ fill: c.text, fontSize: 9, opacity: 0.8 }}>{c.sub}</text>}
          </g>
        );
      })}
    </svg>
  );
}

/** Consensus keyboard: fill = agreement (1 − entropy), legend = modal symbol and its share. */
export function ConsensusBoard({ geometry, cons, ent, alphabet, current }: {
  geometry: Geometry; cons: Float32Array; ent: Float32Array; alphabet: string[]; current: string | null;
}) {
  const [box, { w, h }] = useSize<HTMLDivElement>();
  const s = alphabet.length;
  const b = bounds(geometry);
  const u = Math.min((w - 8) / (b.x1 - b.x0), (h - 8) / (b.y1 - b.y0), 64);
  const cells = (slot: number): Cell => {
    const row = Array.from(cons.subarray(slot * s, (slot + 1) * s)).map((p, sym) => ({ p, sym })).filter((x) => x.p > 0).sort((a, c) => c.p - a.p);
    const top = row[0];
    const agree = 1 - (ent[slot] ?? 1);
    const ch = top ? alphabet[top.sym] : "";
    return {
      fill: heatColor(agree),
      text: heatText(agree),
      main: ch.toUpperCase(),
      sub: top ? `${Math.round(top.p * 100)}%` : "",
      title: row.slice(0, 6).map((x) => `${alphabet[x.sym]} ${(x.p * 100).toFixed(0)}%`).join("  "),
      ring: current !== null && [...current][slot] === ch,
    };
  };
  return (
    <div ref={box} style={{ height: "100%", display: "grid", placeItems: "center" }}>
      {u > 0 && <KeyGrid geometry={geometry} cells={cells} u={u} />}
    </div>
  );
}

/** A layout as a small key grid (symbols only). */
export function Thumb({ geometry, chars, diff }: { geometry: Geometry; chars: string; diff?: string }) {
  const a = [...chars], d = diff ? [...diff] : null;
  return (
    <KeyGrid geometry={geometry} u={17} cells={(i) => ({
      fill: d && d[i] !== a[i] ? "#e8e8e8" : "#ffffff",
      text: "#141414",
      main: (a[i] ?? "").toUpperCase(),
    })} />
  );
}
