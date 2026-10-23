// Process views of a run (research/studies/search-space-visualization.md §3.1, §3.3):
// per-worker convergence, schedule (temperature, acceptance), symbol-migration raster and
// settling order. Pure functions over `progress.jsonl` records; drawn by runs/ProcessPanel.

import type { Progress } from "./api";

/** One worker's current score over time: [elapsed ms, score]. */
export function workerLines(progress: Progress[]): [number, number][][] {
  const out: [number, number][][] = [];
  for (const p of progress) (p.workers ?? []).forEach((w, k) => (out[k] ??= []).push([p.elapsed_ms, w.current]));
  return out.map((l) => l ?? []);
}

export interface Rate { t: number; temp: number | null; accept: number | null; improving: number | null }

/** Per record, over all workers: mean temperature, share of worsening moves accepted, share
 *  of proposals that improve. Null where no worker reports it. */
export function schedule(progress: Progress[]): Rate[] {
  const out: Rate[] = [];
  for (const p of progress) {
    const ws = p.workers ?? [];
    if (!ws.length) continue;
    const temps = ws.flatMap((w) => (w.temp !== undefined ? [w.temp] : []));
    const sum = (f: (w: (typeof ws)[number]) => number) => ws.reduce((a, w) => a + f(w), 0);
    const worse = sum((w) => w.worse), prop = sum((w) => w.proposed);
    out.push({
      t: p.elapsed_ms,
      temp: temps.length ? temps.reduce((a, b) => a + b, 0) / temps.length : null,
      accept: worse > 0 ? sum((w) => w.worse_accepted) / worse : null,
      improving: prop > 0 ? sum((w) => w.improving) / prop : null,
    });
  }
  return out;
}

export interface Frame { t: number; chars: string[] }

/** Layouts over time: the run's best (`worker` null) or one worker's current layout. Records
 *  that repeat the previous layout are dropped. */
export function frames(progress: Progress[], worker: number | null): Frame[] {
  const out: Frame[] = [];
  let last = "";
  for (const p of progress) {
    const c = worker === null ? p.best_chars : p.workers?.[worker]?.chars;
    if (!c || c === last) continue;
    last = c;
    out.push({ t: p.elapsed_ms, chars: [...c] });
  }
  return out;
}

export interface Settle { sym: string; t: number; moves: number }

/** For each symbol: when it last changed slot and how often it moved, latest first. */
export function settling(fs: Frame[]): Settle[] {
  if (!fs.length) return [];
  const last = new Map<string, Settle>();
  const slotOf = (f: Frame) => new Map(f.chars.map((c, i) => [c, i]));
  let prev = slotOf(fs[0]);
  for (const c of fs[0].chars) last.set(c, { sym: c, t: fs[0].t, moves: 0 });
  for (const f of fs.slice(1)) {
    const cur = slotOf(f);
    for (const [c, i] of cur) {
      const s = last.get(c) ?? { sym: c, t: f.t, moves: 0 };
      if (prev.get(c) !== i) { s.t = f.t; s.moves++; }
      last.set(c, s);
    }
    prev = cur;
  }
  return [...last.values()].sort((a, b) => b.t - a.t || b.moves - a.moves || a.sym.localeCompare(b.sym));
}

export interface StnNode { chars: string; score: number; visits: number; workers: number[] }
export interface Stn { nodes: StnNode[]; edges: { a: number; b: number; w: number; worker: number }[]; best: number }

const hamming = (a: string, b: string) => { let d = 0; for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) d++; return d; };

/** Search trajectory network (Ochoa, Malan, Blum 2021): each worker's current layouts in order,
 *  coarse-grained so a layout within `radius` differing slots of an existing location joins it
 *  (the location keeps its first layout and its best score). Edges join consecutive locations of
 *  one worker. At most `cap` locations; later ones join their nearest. */
export function stn(progress: Progress[], radius: number, cap = 300): Stn {
  const nodes: StnNode[] = [];
  const edges = new Map<string, { a: number; b: number; w: number; worker: number }>();
  const last: (number | undefined)[] = [];
  const locate = (c: string, score: number) => {
    let best = -1, bd = Infinity;
    for (let i = 0; i < nodes.length; i++) { const d = hamming(nodes[i].chars, c); if (d < bd) { bd = d; best = i; } if (d === 0) break; }
    if (best < 0 || (bd > radius && nodes.length < cap)) { nodes.push({ chars: c, score, visits: 0, workers: [] }); return nodes.length - 1; }
    nodes[best].score = Math.min(nodes[best].score, score);
    return best;
  };
  for (const p of progress) (p.workers ?? []).forEach((w, k) => {
    const i = locate(w.chars, w.current);
    const n = nodes[i];
    n.visits++;
    if (!n.workers.includes(k)) n.workers.push(k);
    const prev = last[k];
    if (prev !== undefined && prev !== i) {
      const key = `${k}:${prev}:${i}`;
      const e = edges.get(key);
      if (e) e.w++; else edges.set(key, { a: prev, b: i, w: 1, worker: k });
    }
    last[k] = i;
  });
  const best = nodes.reduce((b, n, i) => (b < 0 || n.score < nodes[b].score ? i : b), -1);
  return { nodes, edges: [...edges.values()], best };
}
