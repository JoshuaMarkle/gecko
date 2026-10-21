// 2D boards for Explore: the consensus keyboard (backbone of the focused
// layouts: modal symbol per key, its share, and how contested the key is) (thumbnails: `Board2D.Thumb`).

import type { Geometry } from "../../api";
import { heatColor, heatText } from "../../colors";
import { flatBounds, flatKeys } from "../../format";
import { KeyGrid, type Cell } from "../Board2D";
import { useSize } from "../Charts";

export { Thumb } from "../Board2D";

/** Consensus keyboard: fill = agreement (1 − entropy), legend = modal symbol and its share. */
export function ConsensusBoard({ geometry, cons, ent, alphabet, current }: {
  geometry: Geometry; cons: Float32Array; ent: Float32Array; alphabet: string[]; current: string | null;
}) {
  const [box, { w, h }] = useSize<HTMLDivElement>();
  const s = alphabet.length;
  const b = flatBounds(flatKeys(geometry));
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
