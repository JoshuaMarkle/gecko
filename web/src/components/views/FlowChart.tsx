// Home's flowchart (web-screens.md "Home"): how the inputs become a score and how each optimize
// mode searches it. Every tab is a box that opens it and shows its current value; the derived
// steps (physical cost, frequencies, scoring) are tinted boxes without a tab.

import { useId, type KeyboardEvent } from "react";
import type { View } from "../../tabs";

/** One box: its title, up to three short lines and a state mark (left edge). */
export interface FlowBox { title: string; lines: string[]; state?: "ok" | "warn" | "off" }

type Id = Exclude<View, "home" | "bench"> | "cost" | "freq" | "score";
const W = 1120, H = 470;

/** Box positions [x, y, w, h]; the column order is the data flow. */
const POS: Record<Id, [number, number, number, number]> = {
  board: [16, 16, 196, 84], hand: [16, 116, 196, 84], profile: [16, 216, 196, 84], language: [16, 364, 196, 84],
  cost: [268, 102, 200, 112], freq: [268, 364, 200, 84],
  score: [520, 216, 190, 112], summary: [520, 380, 190, 68],
  layout: [764, 16, 176, 84], shape: [764, 116, 176, 84], both: [764, 216, 176, 84], explore: [764, 316, 176, 84],
  runs: [984, 102, 120, 84], export: [984, 242, 120, 84],
};

const OPT: Id[] = ["layout", "shape", "both", "explore"];
const cy = (id: Id) => POS[id][1] + POS[id][3] / 2;
const right = (id: Id) => POS[id][0] + POS[id][2];

/** Characters that fit a box line (11px Inter ≈ 6.1px per character). */
const clip = (s: string, w: number) => { const n = Math.floor((w - 24) / 6.1); return s.length > n ? `${s.slice(0, n - 1)}…` : s; };

export function FlowChart({ boxes, active, onGo }: { boxes: Partial<Record<Id, FlowBox>>; active: View | null; onGo: (v: View) => void }) {
  const id = useId();
  const box = (k: Id, tab: boolean) => {
    const b = boxes[k];
    if (!b) return null;
    const [x, y, w, h] = POS[k];
    const go = () => tab && onGo(k as View);
    const key = (e: KeyboardEvent) => { if (tab && (e.key === "Enter" || e.key === " ")) { e.preventDefault(); go(); } };
    return (
      <g key={k} className={`flow-node${tab ? " link" : " derived"}${active === k ? " on" : ""}`} onClick={go} onKeyDown={key}
        role={tab ? "link" : undefined} tabIndex={tab ? 0 : undefined} aria-label={tab ? `${b.title}: ${b.lines.join(", ")}` : undefined}>
        <rect x={x} y={y} width={w} height={h} rx={6} />
        {b.state && <rect className={`flow-state ${b.state}`} x={x} y={y + 6} width={3} height={h - 12} />}
        <text x={x + 12} y={y + 21} className="flow-title">{b.title}</text>
        {b.lines.slice(0, (h - 30) / 17).map((l, i) => <text key={i} x={x + 12} y={y + 40 + i * 17}>{clip(l, w)}</text>)}
      </g>
    );
  };
  const bus1 = 737, bus2 = 962;
  return (
    <div className="flowchart-scroll">
      <svg className="flowchart" viewBox={`0 0 ${W} ${H}`} role="img" aria-label="How GECKO optimizes: inputs, physical cost and frequencies, scoring, the four optimize modes, saved runs">
        <defs><marker id={`${id}-arrow`} viewBox="0 0 8 8" refX="7" refY="4" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
          <path d="M1 1 L7 4 L1 7" />
        </marker></defs>
        <g className="flow-links">
          {/* Board, hand and profile make the physical cost; language the frequencies. */}
          <path d={`M${right("board")} ${cy("board")} H240 V140 H268`} markerEnd={`url(#${id}-arrow)`} />
          <path d={`M${right("hand")} ${cy("hand")} H268`} markerEnd={`url(#${id}-arrow)`} />
          <path d={`M${right("profile")} ${cy("profile")} H240 V188 H268`} markerEnd={`url(#${id}-arrow)`} />
          <path d={`M${right("language")} ${cy("language")} H268`} markerEnd={`url(#${id}-arrow)`} />
          <path d={`M${right("cost")} ${cy("cost")} H494 V256 H520`} markerEnd={`url(#${id}-arrow)`} />
          <path d={`M${right("freq")} ${cy("freq")} H494 V292 H520`} markerEnd={`url(#${id}-arrow)`} />
          {/* Scoring feeds every optimize mode; every mode saves runs. */}
          <path d={`M${right("score")} ${cy("score")} H${bus1} M${bus1} ${cy("layout")} V${cy("explore")}`} />
          {OPT.map((o) => <path key={o} className={active === o ? "on" : ""} d={`M${bus1} ${cy(o)} H${POS[o][0]}`} markerEnd={`url(#${id}-arrow)`} />)}
          {OPT.map((o) => <path key={`${o}-out`} d={`M${right(o)} ${cy(o)} H${bus2}`} />)}
          <path d={`M${bus2} ${cy("layout")} V${cy("explore")} M${bus2} ${cy("runs")} H${POS.runs[0]}`} markerEnd={`url(#${id}-arrow)`} />
          <path d={`M${POS.runs[0] + 60} ${POS.runs[1] + POS.runs[3]} V${POS.export[1]}`} markerEnd={`url(#${id}-arrow)`} />
          <path className="dashed" d={`M${POS.score[0] + 95} ${POS.score[1] + POS.score[3]} V${POS.summary[1]}`} />
        </g>
        <g className="flow-groups">
          <text x={16} y={H - 4}>Inputs</text><text x={268} y={H - 4}>Compiled</text><text x={520} y={H - 4}>Score</text>
          <text x={764} y={H - 4}>Optimize</text><text x={984} y={H - 4}>Saved</text>
        </g>
        {(["board", "hand", "profile", "language", "summary", ...OPT, "runs", "export"] as Id[]).map((k) => box(k, true))}
        {(["cost", "freq", "score"] as Id[]).map((k) => box(k, false))}
      </svg>
    </div>
  );
}
