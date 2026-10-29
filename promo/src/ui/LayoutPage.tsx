// The product's Optimize › Layout tab (web/src/components/views/LayoutView.tsx) in the app shell:
// search settings in the sidebar, the board with an overlay switch in the main view, convergence
// and the score against QWERTY in the bottom panel. All values come from the layout run.

import { data, fmt, type Stats } from "../lib/data";
import { clamp01, EASE } from "../lib/motion";
import { C, HAIRLINE, MONO, R, WEIGHT } from "../lib/tokens";
import { Board2D, BOARD_U } from "./Board2D";
import { LineChart } from "./charts";
import { AppFrame } from "./panels";

const L = data.layout;
const ALGO: Record<string, string> = { sa: "Simulated annealing", ga: "Genetic", bls: "Breakout local search", memetic: "Memetic" };
/** Bottom-panel metric rows: stat, name, unit, lower is better. */
const METRICS: { k: keyof Stats; name: string; unit: string; lower: boolean }[] = [
  { k: "sfb", name: "Same-finger bigrams", unit: "%", lower: true },
  { k: "rolls", name: "Rolls", unit: "%", lower: false },
  { k: "alternate", name: "Alternation", unit: "%", lower: false },
  { k: "pinky_ring", name: "Pinky–ring", unit: "%", lower: true },
  { k: "reach", name: "Reach", unit: "", lower: true },
  { k: "effort", name: "Effort", unit: "", lower: true },
];

function Sec({ title, note, s, children }: { title: string; note: string; s: number; children: React.ReactNode }) {
  return (
    <div style={{ paddingBottom: 12 * s, marginBottom: 12 * s, borderBottom: `${HAIRLINE}px solid ${C.hairSoft}` }}>
      <div style={{ display: "flex", alignItems: "baseline", gap: 8 * s, marginBottom: 8 * s }}>
        <span style={{ fontSize: 13 * s, fontWeight: WEIGHT.head }}>{title}</span>
        <span style={{ fontSize: 11 * s, color: C.muted }}>{note}</span>
      </div>
      {children}
    </div>
  );
}

function Row({ label, value, s, mono = false }: { label: string; value: string; s: number; mono?: boolean }) {
  return (
    <div style={{ display: "grid", gridTemplateColumns: "42% 1fr", alignItems: "center", gap: 8 * s, marginBottom: 6 * s, fontSize: 12 * s }}>
      <span style={{ color: C.muted }}>{label}</span>
      <span style={{ background: C.field, borderRadius: R.xs, padding: `${4 * s}px ${8 * s}px`, fontFamily: mono ? MONO : undefined,
        whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{value}</span>
    </div>
  );
}

function Seg({ options, on, s }: { options: string[]; on: string; s: number }) {
  return (
    <div style={{ display: "inline-flex", background: C.soft, borderRadius: R.sm, padding: 2 * s, gap: 2 * s, fontSize: 11 * s }}>
      {options.map((o) => (
        <span key={o} style={{ padding: `${3 * s}px ${8 * s}px`, borderRadius: R.xs, background: o === on ? C.canvas : "transparent",
          boxShadow: o === on ? `0 0 0 ${HAIRLINE}px ${C.hair}` : undefined, color: o === on ? C.ink : C.muted }}>{o}</span>
      ))}
    </div>
  );
}

/**
 * The Layout tab at `t` (0..1) of the run: the board shows the best layout so far, the convergence
 * line and score follow it; `reveal` builds the panels in. `overlay` picks the board fill.
 */
export function LayoutPage({ t = 1, reveal = 1, overlay = "Fingers", scale = 1.4, width, height }: {
  t?: number; reveal?: number; overlay?: "Default" | "Fingers"; scale?: number; width?: number; height?: number;
}) {
  const s = scale;
  const n = L.trace.length;
  const i = Math.round(clamp01(t) * (n - 1));
  const cur = L.trace[i];
  const conv = L.trace.slice(0, i + 1).map((p) => [p.evals / 1e6, p.best] as [number, number]);
  const done = i === n - 1;
  const fade = (k: number) => ({ opacity: EASE.out(clamp01(reveal * 3 - k)), transform: `translateY(${(1 - EASE.out(clamp01(reveal * 3 - k))) * 12}px)` });
  const best = cur.best, ref = L.qwerty.score;
  const u = 44;
  return (
    <AppFrame active="layout" scale={s} width={width} height={height}
      model={`${data.geometry} · ${done ? "run " + L.run.slice(9, 15) : "running"}`} score={fmt.n(best, 2)}
      status={done ? undefined : `${L.algo.toUpperCase()} · ${(cur.evals / 1e6).toFixed(1)}M evals`}
      sidebar={
        <div style={fade(0)}>
          <div style={{ fontSize: 15 * s, fontWeight: WEIGHT.head, marginBottom: 14 * s }}>Optimize layout</div>
          <Sec title="Algorithm" note="how the search moves" s={s}><Row label="Algorithm" value={ALGO[L.algo] ?? L.algo} s={s} /></Sec>
          <Sec title="Starting layout" note="where it begins" s={s}><Row label="Layout" value="QWERTY" s={s} /><Row label="Pinned" value={L.pins} s={s} mono /></Sec>
          <Sec title="Symbols" note="what the search places" s={s}><Row label="Symbols" value="Starting layout" s={s} /></Sec>
          <Sec title="Budget" note="how long, how wide" s={s}>
            <Row label="Evaluations" value={`${(L.evals / 1e6).toFixed(0)}M`} s={s} />
            <Row label="Threads" value={String(L.workers[0]?.length ?? 1)} s={s} />
            <Row label="Seed" value={String(L.seed)} s={s} />
          </Sec>
        </div>
      }
      main={
        <div style={{ position: "absolute", inset: 0, ...fade(0.5) }}>
          <div style={{ position: "absolute", left: 12 * s, top: 12 * s, background: C.panel, borderRadius: R.xs, padding: `${4 * s}px ${8 * s}px`,
            fontSize: 12 * s, display: "flex", gap: 8 * s }}>
            <span style={{ fontWeight: WEIGHT.head }}>{done ? "Best layout" : "Live best"}</span>
            <span style={{ color: C.muted }}>{best.toFixed(3)}</span>
          </div>
          <div style={{ position: "absolute", left: "50%", top: "50%", transform: `translate(-50%, -46%)`, width: BOARD_U.w * u, height: BOARD_U.h * u }}>
            <Board2D u={u} chars={cur.best_chars} fill={overlay === "Fingers" ? "fingers" : "white"} />
          </div>
          <div style={{ position: "absolute", right: 12 * s, top: 12 * s, display: "flex", gap: 6 * s }}>
            <Seg options={["Result", "Algorithm"]} on="Result" s={s} />
            <Seg options={["3D"]} on="" s={s} />
            <Seg options={["Default", "Heat", "Fingers", "N-grams"]} on={overlay} s={s} />
          </div>
        </div>
      }
      bottom={
        <div style={{ position: "absolute", inset: 0, padding: 14 * s, display: "grid", gridTemplateColumns: "1.5fr 1fr", gap: 20 * s, ...fade(1) }}>
          <div>
            <div style={{ fontSize: 13 * s, fontWeight: WEIGHT.head, marginBottom: 4 * s }}>Convergence <span style={{ fontWeight: 400, color: C.muted, fontSize: 11 * s }}>{L.algo} · {(cur.ms / 1000).toFixed(1)} s</span></div>
            <LineChart w={1000} h={300} series={[{ points: conv, step: true }]} highlight={conv.at(-1)!} threshold={{ y: ref, label: "QWERTY" }}
              yDomain={[0, Math.ceil(ref / 10) * 10]} xFormat={(x) => `${x.toFixed(0)}M`} yFormat={(y) => y.toFixed(0)} />
          </div>
          <div>
            <div style={{ fontSize: 13 * s, fontWeight: WEIGHT.head }}>Score <span style={{ fontWeight: 400, color: C.muted, fontSize: 11 * s }}>lower is better</span></div>
            <div style={{ display: "flex", alignItems: "baseline", gap: 10 * s, margin: `${6 * s}px 0 ${8 * s}px` }}>
              <b style={{ fontSize: 28 * s, fontWeight: WEIGHT.head }}>{best.toFixed(3)}</b>
              <span style={{ background: best < ref ? C.success : C.soft, color: C.white, borderRadius: R.xs, padding: `${2 * s}px ${6 * s}px`, fontSize: 11 * s, fontWeight: WEIGHT.head }}>
                {(best - ref).toFixed(3)} vs QWERTY
              </span>
            </div>
            {METRICS.map((m) => {
              const v = cur.stats[m.k], r = L.qwerty[m.k];
              const better = m.lower ? v < r - 1e-9 : v > r + 1e-9, worse = m.lower ? v > r + 1e-9 : v < r - 1e-9;
              return (
                <div key={m.k} style={{ display: "grid", gridTemplateColumns: "1fr auto 18px", gap: 8 * s, alignItems: "center", fontSize: 11.5 * s,
                  padding: `${2.5 * s}px 0`, borderBottom: `1px solid ${C.hairSoft}` }}>
                  <span>{m.name}</span>
                  <span style={{ fontVariantNumeric: "tabular-nums" }}>{v.toFixed(2)}{m.unit}</span>
                  <i style={{ width: 8 * s, height: 8 * s, borderRadius: "50%", background: better ? C.success : worse ? C.error : "transparent" }} />
                </div>
              );
            })}
          </div>
        </div>
      }
    />
  );
}
