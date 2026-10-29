// Product panels at display scale: table, mono list, segmented tabs, status badge, file card,
// terminal, the paper-template page, navbar + app frame, the Home flowchart and the end card.
// Tokens only; reveal / count props (0..1) drive the motion.

import type { CSSProperties, ReactNode } from "react";
import { useCurrentFrame } from "remotion";
import { clamp01, DUR, EASE, lerp, presence, progress, STAGGER } from "../lib/motion";
import { C, HAIRLINE, MONO, R, TYPE, WEIGHT } from "../lib/tokens";
import { IconBench, IconBoard, IconExplore, IconExport, IconHand, IconHome, IconLanguage, IconOptBoard, IconOptBoth, IconOptLayout, IconPause, IconPlay, IconProfile, IconRuns, IconStop, IconSummary } from "./icons";

/** Table (11 px → 22 px): muted header over a hairline, rows divided by --hair-soft, numbers right. Rows reveal top-down. */
export function DataTable({ columns, rows, reveal = 1, width, size = 24, maxRows }: {
  columns: { label: string; align?: "left" | "right"; mono?: boolean; width?: number }[]; rows: (string | number)[][]; reveal?: number; width?: number;
  size?: number; maxRows?: number;
}) {
  const shown = rows.slice(0, maxRows ?? rows.length);
  const cell = (c: (typeof columns)[number]): CSSProperties => ({ padding: "10px 14px", textAlign: c.align ?? "right", fontFamily: c.mono ? MONO : undefined, width: c.width, whiteSpace: "nowrap" });
  return (
    <table style={{ borderCollapse: "collapse", fontSize: size, fontVariantNumeric: "tabular-nums", width }}>
      <thead>
        <tr style={{ borderBottom: `${HAIRLINE}px solid ${C.hair}`, opacity: clamp01(reveal * 4) }}>
          {columns.map((c, i) => <th key={i} style={{ ...cell(c), color: C.muted, fontWeight: WEIGHT.text }}>{c.label}</th>)}
        </tr>
      </thead>
      <tbody>
        {shown.map((r, i) => {
          const t = clamp01(reveal * 1.4 - (i / shown.length) * 0.4) ;
          return (
            <tr key={i} style={{ borderBottom: `${HAIRLINE}px solid ${C.hairSoft}`, opacity: t, transform: `translateX(${(1 - EASE.out(t)) * -24}px)` }}>
              {r.map((v, j) => <td key={j} style={{ ...cell(columns[j]), color: j === 0 ? C.ink : C.inkSoft }}>{v}</td>)}
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}

/** Mono lines (n-grams, weights) that can scroll: `scroll` in lines. */
export function MonoList({ lines, height, scroll = 0, size = 24, reveal = 1 }: { lines: [string, string][]; height: number; scroll?: number; size?: number; reveal?: number }) {
  const lh = size * 1.6;
  return (
    <div style={{ height, overflow: "hidden", fontFamily: MONO, fontSize: size, position: "relative",
      maskImage: "linear-gradient(transparent, black 12%, black 88%, transparent)" }}>
      <div style={{ transform: `translateY(${-scroll * lh}px)` }}>
        {lines.map(([k, v], i) => (
          <div key={i} style={{ height: lh, display: "flex", justifyContent: "space-between", gap: 32, opacity: clamp01(reveal * lines.length * 0.4 - i * 0.4) }}>
            <span>{k}</span><span style={{ color: C.muted }}>{v}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

/** Segmented control: --soft track, the active option a white segment with black type that slides (`active` may be fractional). */
export function SegmentedTabs({ options, active, size = TYPE.pane, w = 220 }: { options: string[]; active: number; size?: number; w?: number }) {
  const pad = 6, h = size * 2.2;
  return (
    <div style={{ position: "relative", display: "flex", background: C.soft, borderRadius: R.sm, padding: pad, height: h, boxSizing: "content-box" }}>
      <div style={{ position: "absolute", top: pad, left: pad + active * w, width: w, height: h, background: C.ink, borderRadius: R.xs }} />
      {options.map((o, i) => {
        const on = 1 - clamp01(Math.abs(active - i));
        return (
          <div key={o} style={{ position: "relative", width: w, height: h, display: "flex", alignItems: "center", justifyContent: "center",
            fontSize: size, fontWeight: on > 0.5 ? WEIGHT.head : WEIGHT.text, color: on > 0.5 ? C.canvas : C.muted }}>{o}</div>
        );
      })}
    </div>
  );
}

/** Status badge: --r-xs chip; success / error solid with white text, warning fill with ink text. Enters with a small rise. */
export function StatusBadge({ kind, text, start = 0, size = TYPE.body }: { kind: "success" | "error" | "warning" | "best"; text: string; start?: number; size?: number }) {
  const frame = useCurrentFrame();
  const p = progress(frame, start, DUR.enter);
  const bg = kind === "best" ? C.accent : C[kind];
  return (
    <span style={{ display: "inline-flex", alignItems: "center", gap: 10, padding: `${size * 0.25}px ${size * 0.5}px`, borderRadius: R.xs,
      background: bg, color: kind === "warning" ? C.canvas : C.white, fontSize: size, fontWeight: WEIGHT.head,
      opacity: p, transform: `translateY(${(1 - p) * 10}px) scale(${lerp(0.96, 1, p)})` }}>
      {kind === "success" && <svg width={size * 0.7} height={size * 0.7} viewBox="0 0 16 16"><path d="M3 8.5l3 3 7-7" fill="none" stroke="currentColor" strokeWidth={2.2} strokeLinecap="round" strokeLinejoin="round" /></svg>}
      {text}
    </span>
  );
}

/** Export download card: file icon, name, size, dimensions, a download button. `press` 0..1 clicks it:
 * the grey button sinks, turns accent with a check at the bottom of the press, and springs back. */
export function FileCard({ name, size, dims, reveal = 1, width = 560, press = 0 }: { name: string; size: string; dims: string; reveal?: number; width?: number; press?: number }) {
  const t = EASE.out(clamp01(reveal));
  const down = Math.sin(Math.PI * clamp01(press)), done = press >= 0.5;
  return (
    <div style={{ width, display: "flex", alignItems: "center", gap: 28, padding: 28, background: C.panel, border: `${HAIRLINE}px solid ${C.hair}`,
      borderRadius: R.md, opacity: clamp01(reveal * 2), transform: `translateY(${(1 - t) * 24}px)` }}>
      <svg width={64} height={80} viewBox="0 0 32 40">
        <path d="M3 2h18l8 8v28H3z" fill={C.soft} stroke={C.hair} strokeWidth={1} />
        <path d="M21 2v8h8" fill="none" stroke={C.hair} strokeWidth={1} />
        <text x={16} y={30} textAnchor="middle" style={{ fontSize: 7, fontWeight: 650, fill: C.ink, fontFamily: MONO }}>STL</text>
      </svg>
      <div style={{ flex: 1 }}>
        <div style={{ fontSize: TYPE.pane, fontWeight: WEIGHT.head }}>{name}</div>
        <div style={{ fontSize: TYPE.hint, color: C.muted, marginTop: 6, fontVariantNumeric: "tabular-nums" }}>{size} · {dims}</div>
      </div>
      <div style={{ width: 64, height: 64, borderRadius: R.sm, background: done ? C.accent : C.soft, color: done ? C.white : C.ink, display: "flex", alignItems: "center", justifyContent: "center",
        border: `${HAIRLINE}px solid ${done ? C.accent : C.hair}`, boxSizing: "border-box", transform: `scale(${1 - 0.1 * down})` }}>
        {done
          ? <svg width={32} height={32} viewBox="0 0 16 16"><path d="M3 8.5l3 3 7-7" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" /></svg>
          : <IconExport size={32} />}
      </div>
    </div>
  );
}

/** Terminal: hairline window, a muted title bar, mono lines typed in (`reveal` 0..1 over the lines). */
export function TerminalWindow({ command, lines, reveal = 1, width = 900, title = "gecko", size = 24 }: {
  command: string; lines: string[]; reveal?: number; width?: number; title?: string; size?: number;
}) {
  const total = command.length + lines.length * 8;
  const shown = Math.floor(clamp01(reveal) * total);
  const typed = command.slice(0, Math.min(command.length, shown));
  const nLines = Math.max(0, Math.floor((shown - command.length) / 8));
  return (
    <div style={{ width, background: C.panel, border: `${HAIRLINE}px solid ${C.hair}`, borderRadius: R.md, overflow: "hidden", fontFamily: MONO, fontSize: size }}>
      <div style={{ height: 52, display: "flex", alignItems: "center", gap: 10, padding: "0 20px", background: C.soft, color: C.muted, fontSize: size * 0.8 }}>
        {[0, 1, 2].map((i) => <span key={i} style={{ width: 14, height: 14, borderRadius: 7, background: C.hair }} />)}
        <span style={{ marginLeft: 12 }}>{title}</span>
      </div>
      <div style={{ padding: "20px 24px", lineHeight: 1.55, whiteSpace: "pre" }}>
        <div><span style={{ color: C.muted }}>$ </span>{typed}{nLines === 0 && <span style={{ background: C.ink, color: C.ink }}>_</span>}</div>
        {lines.slice(0, nLines).map((l, i) => <div key={i} style={{ color: i === nLines - 1 ? C.ink : C.inkSoft }}>{l}</div>)}
      </div>
    </div>
  );
}

/** The paper template's sheet: white page with thin edges and generous margins on --soft. */
export function Page({ children, width = 1100, height = 1400, margin = 120, style }: { children: ReactNode; width?: number; height?: number; margin?: number; style?: CSSProperties }) {
  return (
    <div style={{ width, height, background: C.panel, border: `${HAIRLINE}px solid ${C.hair}`, padding: margin, boxSizing: "border-box", ...style }}>
      {children}
    </div>
  );
}

/** Navbar tab groups as in the product (Unity-style): Home | Inputs | Optimize | Saved. */
export const NAV_GROUPS: { label: string; tabs: { id: string; Icon: (p: { size?: number }) => ReactNode }[] }[] = [
  { label: "", tabs: [{ id: "home", Icon: IconHome }] },
  { label: "Inputs", tabs: [{ id: "board", Icon: IconBoard }, { id: "hand", Icon: IconHand }, { id: "language", Icon: IconLanguage }, { id: "profile", Icon: IconProfile }, { id: "summary", Icon: IconSummary }] },
  { label: "Optimize", tabs: [{ id: "layout", Icon: IconOptLayout }, { id: "shape", Icon: IconOptBoard }, { id: "both", Icon: IconOptBoth }, { id: "explore", Icon: IconExplore }] },
  { label: "Saved", tabs: [{ id: "runs", Icon: IconRuns }, { id: "bench", Icon: IconBench }, { id: "export", Icon: IconExport }] },
];

/** Navbar (40 px → 80 px): icon tabs left, transport centre, working model right. */
export function Navbar({ active, model, score, status, scale = 2 }: { active: string; model?: string; score?: string; status?: string; scale?: number }) {
  const h = 40 * scale, icon = 16 * scale;
  return (
    <div style={{ height: h, display: "grid", gridTemplateColumns: "1fr auto 1fr", alignItems: "center", padding: `0 ${12 * scale}px`, borderBottom: `${HAIRLINE}px solid ${C.hair}`, background: C.canvas }}>
      <div style={{ display: "flex", alignItems: "center", gap: 10 * scale }}>
        {NAV_GROUPS.map((g, gi) => (
          <div key={gi} style={{ display: "flex", alignItems: "center", gap: 6 * scale }}>
            {g.label && <span style={{ fontSize: 10 * scale, color: C.muted }}>{g.label}</span>}
            <div style={{ display: "flex", background: C.soft, borderRadius: R.sm, padding: 2 * scale, gap: 2 * scale }}>
              {g.tabs.map(({ id, Icon }) => (
                <div key={id} style={{ width: 30 * scale, height: 22 * scale, display: "flex", alignItems: "center", justifyContent: "center", borderRadius: R.xs,
                  background: id === active ? C.canvas : "transparent", boxShadow: id === active ? `0 0 0 ${HAIRLINE}px ${C.hair}` : undefined, color: id === active ? C.ink : C.muted }}>
                  <Icon size={icon} />
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>
      <div style={{ display: "flex", borderRadius: R.sm, overflow: "hidden", border: `${HAIRLINE}px solid ${C.hair}` }}>
        <div style={{ width: 34 * scale, height: 24 * scale, background: C.ink, color: C.canvas, display: "flex", alignItems: "center", justifyContent: "center" }}><IconPlay size={icon * 0.8} /></div>
        <div style={{ width: 34 * scale, height: 24 * scale, display: "flex", alignItems: "center", justifyContent: "center", color: C.muted }}><IconPause size={icon * 0.8} /></div>
        <div style={{ width: 34 * scale, height: 24 * scale, display: "flex", alignItems: "center", justifyContent: "center", color: C.muted, borderLeft: `${HAIRLINE}px solid ${C.hair}` }}><IconStop size={icon * 0.8} /></div>
      </div>
      <div style={{ justifySelf: "end", fontSize: 12 * scale, display: "flex", gap: 8 * scale, alignItems: "baseline" }}>
        {status ? <span style={{ fontFamily: MONO, color: C.muted }}>{status}</span> : <>
          <span style={{ fontWeight: WEIGHT.head }}>{model ?? "No model"}</span>
          {score && <span style={{ color: C.muted }}>{score}</span>}
        </>}
      </div>
    </div>
  );
}

/** App shell: navbar over sidebar | main visual (--soft) over the bottom panel. */
export function AppFrame({ active, sidebar, main, bottom, width = 1920, height = 1080, scale = 1.4, model, score, status }: {
  active: string; sidebar?: ReactNode; main?: ReactNode; bottom?: ReactNode; width?: number; height?: number; scale?: number;
  model?: string; score?: string; status?: string;
}) {
  const side = 380 * scale;
  return (
    <div style={{ width, height, background: C.canvas, display: "flex", flexDirection: "column", overflow: "hidden" }}>
      <Navbar active={active} model={model} score={score} status={status} scale={scale} />
      <div style={{ flex: 1, display: "flex", minHeight: 0 }}>
        <div style={{ width: side, borderRight: `${HAIRLINE}px solid ${C.hair}`, padding: 16 * scale, boxSizing: "border-box", overflow: "hidden" }}>{sidebar}</div>
        <div style={{ flex: 1, display: "flex", flexDirection: "column", minWidth: 0 }}>
          <div style={{ flex: 62, background: C.soft, position: "relative", overflow: "hidden" }}>{main}</div>
          <div style={{ flex: 38, borderTop: `${HAIRLINE}px solid ${C.hair}`, position: "relative", overflow: "hidden" }}>{bottom}</div>
        </div>
      </div>
    </div>
  );
}

/** Home flowchart: input cards (3 px semantic left edge) → derived boxes (--soft) → optimize, hairline elbow connectors. */
export function FlowChart({ width = 1400, reveal = 1, activeBox = "Layout" }: { width?: number; reveal?: number; activeBox?: string }) {
  const cols: { title: string; boxes: { t: string; sub: string; derived?: boolean; edge?: string }[] }[] = [
    { title: "Inputs", boxes: [{ t: "Board", sub: "corne-42", edge: C.success }, { t: "Hand", sub: "average", edge: C.success }, { t: "Language", sub: "prose", edge: C.success }, { t: "Profile", sub: "hand", edge: C.success }] },
    { title: "Derived", boxes: [{ t: "Scoring", sub: "52 metrics", derived: true }] },
    { title: "Optimize", boxes: [{ t: "Layout", sub: "SA" }, { t: "Board", sub: "CMA-ES" }, { t: "Both", sub: "nested" }, { t: "Explore", sub: "atlas" }] },
  ];
  const bw = width * 0.28, bh = 92, gx = (width - 3 * bw) / 2, gy = 26;
  const colH = (n: number) => n * bh + (n - 1) * gy;
  const H = colH(4) + 60;
  const pos = (c: number, i: number): [number, number] => [c * (bw + gx), (H - 60 - colH(cols[c].boxes.length)) / 2 + i * (bh + gy)];
  let k = 0;
  return (
    <svg width={width} height={H} style={{ overflow: "visible" }}>
      {/* connectors: every box of a column to the next column's centre */}
      {[0, 1].map((c) => cols[c].boxes.map((_, i) => {
        const [x, y] = pos(c, i), [nx] = pos(c + 1, 0);
        const ty = (H - 60) / 2;
        const midx = x + bw + gx / 2;
        const t = clamp01(reveal * 2 - 0.6 - c * 0.3);
        const d = `M${x + bw},${y + bh / 2} H${midx} V${ty} H${nx - 8}`;
        return <g key={`${c}.${i}`} opacity={t}>
          <path d={d} fill="none" stroke={C.faint} strokeWidth={HAIRLINE} />
          <path d={`M${nx - 16},${ty - 7} L${nx - 6},${ty} L${nx - 16},${ty + 7}`} fill="none" stroke={C.faint} strokeWidth={HAIRLINE} />
        </g>;
      }))}
      {cols.map((col, c) => (
        <g key={c}>
          {col.boxes.map((b, i) => {
            const [x, y] = pos(c, i);
            const t = EASE.out(clamp01(reveal * 2 - (k++) * 0.07));
            const on = b.t === activeBox && c === 2;
            return (
              <g key={i} opacity={t} transform={`translate(${x} ${y + (1 - t) * 12})`}>
                <rect width={bw} height={bh} rx={R.md} fill={b.derived ? C.soft : C.canvas} stroke={on ? C.accent : C.hair} strokeWidth={on ? 3 : HAIRLINE} />
                {b.edge && <rect width={6} height={bh} rx={2} fill={b.edge} />}
                <text x={26} y={40} style={{ fontSize: 28, fontWeight: WEIGHT.head, fill: C.ink }}>{b.t}</text>
                <text x={26} y={72} style={{ fontSize: 22, fill: C.muted }}>{b.sub}</text>
              </g>
            );
          })}
          <text x={pos(c, 0)[0] + bw / 2} y={H - 10} textAnchor="middle" style={{ fontSize: 22, fill: C.muted }} opacity={clamp01(reveal * 2 - 1)}>{col.title}</text>
        </g>
      ))}
    </svg>
  );
}

/** End card: GECKO (650), the long name (300), the repo URL (mono, muted), staggered rises. */
export function EndCard({ start = 0, exitAt }: { start?: number; exitAt?: number }) {
  const frame = useCurrentFrame();
  const a = presence(frame, start, exitAt, DUR.move, DUR.move);
  const b = presence(frame, start + STAGGER.line * 2, exitAt, DUR.move, DUR.move);
  const c = presence(frame, start + STAGGER.line * 4, exitAt, DUR.move, DUR.move);
  const rise = (p: number) => ({ opacity: p, transform: `translateY(${(1 - p) * 24}px)` });
  return (
    <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 28 }}>
      <div style={{ fontSize: TYPE.wordmark, fontWeight: WEIGHT.head, lineHeight: 1, ...rise(a) }}>GECKO</div>
      <div style={{ fontSize: 56, fontWeight: WEIGHT.light, ...rise(b) }}>General Engine for Custom Keyboard Optimization</div>
      <div style={{ fontSize: 32, fontFamily: MONO, color: C.muted, marginTop: 24, ...rise(c) }}>github.com/JoshuaMarkle/gecko</div>
    </div>
  );
}
