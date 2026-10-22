// Boards of a board + layout search (geometry runs): a parametric board at given free-parameter
// values (WASM `param_board`), its keycap collisions, and a 2D overlay of two boards.

import { useMemo } from "react";
import type { Geometry, Key, ParamGeometry } from "../../api";
import { COLLIDE, HAIR, INK, MUTED } from "../../colors";
import { flatBounds, flatKeys } from "../../format";
import { paramBoard, type Collision } from "../../wasm";

export interface ParamBoard { geometry: Geometry; collisions: Collision[]; error?: string }

/** A board at `values` (null without a parameter file; `error` when the generator rejects them). */
export function useParamBoard(param: ParamGeometry | undefined, names: string[], values: number[] | undefined, id: string): ParamBoard | null {
  const key = values ? values.join(",") : "";
  return useMemo(() => {
    if (!param || !values || values.length !== names.length) return null;
    try {
      return paramBoard(param, names, values, id);
    } catch (e) {
      return { geometry: { id, name: param.name, description: "", key: [] }, collisions: [], error: (e as Error).message };
    }
    // `key` stands for `values`.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [param, names.join(","), key, id]);
}

/** Per-key keycap colors marking the keys in a collision. */
export function collisionColors(n: number, collisions: Collision[]): (string | undefined)[] | undefined {
  if (!collisions.length) return undefined;
  const c: (string | undefined)[] = Array(n).fill(undefined);
  for (const x of collisions) c[x.a] = c[x.b] = COLLIDE;
  return c;
}

/** Stable camera framing for a board that changes shape: the start board's size plus a margin. */
export function stableFocus(keys: Key[], margin = 1): { cx: number; cz: number; w: number; h: number } {
  const xs = keys.flatMap((k) => [k.pos[0], k.pos[0] + k.size[0]]);
  const ys = keys.flatMap((k) => [k.pos[1], k.pos[1] + k.size[1]]);
  const w = Math.max(...xs) - Math.min(...xs), h = Math.max(...ys) - Math.min(...ys);
  return { cx: 0, cz: 0, w: Math.round(w + margin), h: Math.round(h + margin) };
}

/**
 * Two boards top-down in the same coordinates: `ghost` as dashed outlines, `board` as keys with
 * `chars` (colliding keys tinted). Positions are the keys' own x / y (not the flattened drawing),
 * so the boards line up as they would on a desk.
 */
export function BoardOverlay({ board, ghost, chars, collisions, width, height }: {
  board: Geometry; ghost?: Geometry; chars: string; collisions: Collision[]; width: number; height: number;
}) {
  const plain = (g: Geometry) => flatKeys({ ...g, flat: undefined });
  const ks = plain(board), gs = ghost ? plain(ghost) : [];
  const b = flatBounds([...ks, ...gs]);
  const pad = 6;
  const u = Math.max(2, Math.min((width - 2 * pad) / (b.x1 - b.x0), (height - 2 * pad) / (b.y1 - b.y0), 48));
  const ox = (width - (b.x1 - b.x0) * u) / 2, oy = (height - (b.y1 - b.y0) * u) / 2;
  const hit = new Set(collisions.flatMap((c) => [c.a, c.b]));
  const a = [...chars];
  const rect = (k: (typeof ks)[number]) => {
    const x = ox + (k.x - b.x0) * u, y = oy + (k.y - b.y0) * u, w = k.w * u - 2, h = k.h * u - 2;
    return { x: x + 1, y: y + 1, w, h, t: k.rot ? `rotate(${k.rot} ${x + 1 + w / 2} ${y + 1 + h / 2})` : undefined };
  };
  return (
    <svg width={width} height={height}>
      {ks.map((k, i) => {
        const r = rect(k);
        return (
          <g key={i} transform={r.t}>
            <rect x={r.x} y={r.y} width={r.w} height={r.h} rx={3} fill={hit.has(i) ? COLLIDE : "#ffffff"} stroke={HAIR} />
            <text x={r.x + r.w / 2} y={r.y + r.h / 2 + 4} textAnchor="middle" style={{ fill: INK, fontWeight: 600, fontSize: Math.max(9, u * 0.3) }}>{(a[i] ?? "").toUpperCase()}</text>
          </g>
        );
      })}
      {gs.map((k, i) => {
        const r = rect(k);
        return <rect key={`g${i}`} transform={r.t} x={r.x} y={r.y} width={r.w} height={r.h} rx={3} fill="none" stroke={MUTED} strokeDasharray="3 2" pointerEvents="none" />;
      })}
    </svg>
  );
}
