// Local optima network layout (research/studies/search-space-visualization.md §3.2, 15):
// a deterministic force-directed layout of the best optima of an atlas and the kicks between them.

export interface LonEdge { a: number; b: number; w: number }

/** Keep the `k` best rows (by score) and the edges among them, re-indexed 0..k. Self-loops and
 *  repeated pairs are merged (weights add). */
export function subgraph(order: ArrayLike<number>, k: number, edges: ArrayLike<number>): { rows: number[]; edges: LonEdge[] } {
  const rows = Array.from(order).slice(0, k);
  const idx = new Map(rows.map((r, i) => [r, i]));
  const merged = new Map<number, LonEdge>();
  // `edges` is AtlasView.edges(): [from, to, count, kick << 8 | strength] per edge.
  for (let e = 0; e + 3 < edges.length; e += 4) {
    const a = idx.get(edges[e]), b = idx.get(edges[e + 1]);
    if (a === undefined || b === undefined || a === b) continue;
    const key = Math.min(a, b) * 65536 + Math.max(a, b);
    const m = merged.get(key);
    if (m) m.w += edges[e + 2];
    else merged.set(key, { a: Math.min(a, b), b: Math.max(a, b), w: edges[e + 2] });
  }
  return { rows, edges: [...merged.values()] };
}

/** Fruchterman–Reingold in the unit square, seeded so the same graph always lands the same way. */
export function forceLayout(n: number, edges: LonEdge[], iters = 300, seed = 1): Float32Array {
  const xy = new Float32Array(2 * n);
  let s = seed >>> 0 || 1;
  const rnd = () => ((s = (Math.imul(s, 1664525) + 1013904223) >>> 0) / 4294967296);
  for (let i = 0; i < 2 * n; i++) xy[i] = rnd();
  if (n < 2) return xy.fill(0.5);
  const k = Math.sqrt(1 / n);
  const dx = new Float32Array(2 * n);
  for (let it = 0; it < iters; it++) {
    const temp = 0.1 * (1 - it / iters) + 0.002;
    dx.fill(0);
    for (let i = 0; i < n; i++) for (let j = i + 1; j < n; j++) {
      const x = xy[2 * i] - xy[2 * j], y = xy[2 * i + 1] - xy[2 * j + 1];
      const d2 = Math.max(x * x + y * y, 1e-6), f = (k * k) / d2;
      dx[2 * i] += x * f; dx[2 * i + 1] += y * f; dx[2 * j] -= x * f; dx[2 * j + 1] -= y * f;
    }
    for (const e of edges) {
      const x = xy[2 * e.a] - xy[2 * e.b], y = xy[2 * e.a + 1] - xy[2 * e.b + 1];
      const d = Math.max(Math.sqrt(x * x + y * y), 1e-3), f = (d / k) * Math.min(1 + Math.log(e.w), 3);
      dx[2 * e.a] -= x * f; dx[2 * e.a + 1] -= y * f; dx[2 * e.b] += x * f; dx[2 * e.b + 1] += y * f;
    }
    for (let i = 0; i < n; i++) {
      // Gentle pull to the centre keeps unconnected optima from drifting to the border.
      dx[2 * i] += (0.5 - xy[2 * i]) * 0.05; dx[2 * i + 1] += (0.5 - xy[2 * i + 1]) * 0.05;
      const l = Math.hypot(dx[2 * i], dx[2 * i + 1]) || 1, m = Math.min(l, temp) / l;
      xy[2 * i] += dx[2 * i] * m; xy[2 * i + 1] += dx[2 * i + 1] * m;
    }
  }
  // Fit to [0, 1]².
  let x0 = Infinity, x1 = -Infinity, y0 = Infinity, y1 = -Infinity;
  for (let i = 0; i < n; i++) { x0 = Math.min(x0, xy[2 * i]); x1 = Math.max(x1, xy[2 * i]); y0 = Math.min(y0, xy[2 * i + 1]); y1 = Math.max(y1, xy[2 * i + 1]); }
  for (let i = 0; i < n; i++) { xy[2 * i] = (xy[2 * i] - x0) / (x1 - x0 || 1); xy[2 * i + 1] = (xy[2 * i + 1] - y0) / (y1 - y0 || 1); }
  return xy;
}
