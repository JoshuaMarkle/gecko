import type { ReactNode } from "react";
import { FINGERS, type CorpusInfo, type Geometry, type LayoutInfo, type Meta, type Profile } from "../../api";
import { fingerTargets } from "../../fingerEffort";
import { handDims, wristOf } from "../../hand";
import type { Inputs } from "../../inputs";
import { CONSTRAINTS } from "./BoardView";
import { algoName } from "./LayoutView";

const number = (v: number) => Number(v.toPrecision(5)).toLocaleString("en-US", { maximumFractionDigits: 5 });
const value = (v: unknown): string => Array.isArray(v) ? v.map(value).join(" / ") : typeof v === "number" ? number(v) : typeof v === "boolean" ? v ? "On" : "Off" : String(v ?? "—");
type Row = [string, unknown, boolean, string];
function Sheet({ title, children, wide = false }: { title: string; children: ReactNode; wide?: boolean }) {
  return <section className={`input-sheet${wide ? " wide" : ""}`}><h2>{title}</h2><div className="input-sheet-table">{children}</div></section>;
}
function Settings({ rows }: { rows: Row[] }) {
  return <table className="tbl summary-table"><thead><tr><th scope="col">Parameter</th><th scope="col">Value</th><th scope="col">Use / condition</th></tr></thead><tbody>
    {rows.map(([name, v, on, note]) => <tr key={name} className={on ? "" : "off"}><th scope="row">{name}</th><td>{value(v)}</td><td>{on ? "Included" : "Inactive"} · {note}</td></tr>)}
  </tbody></table>;
}

export function SummaryView({ inputs: i, meta, geometry: g, corpus, profile: p, layouts }: {
  inputs: Inputs; meta: Meta; geometry: Geometry; corpus?: CorpusInfo; profile: Profile; layouts: LayoutInfo[];
}) {
  const s = i.search, e = p.finger_effort;
  const weighted = (...ids: string[]) => ids.some(id => (p.weights[id] ?? 0) !== 0);
  const targets = e ? fingerTargets(e, g) : p.finger_target;
  const start = layouts.find(l => l.id === s.start);
  const memetic = s.algo === "memetic";
  const custom = i.customBoards.some(b => b.id === g.id);
  const skipDecay = p.sfb?.skip_decay ?? 0.5;
  const active = meta.metrics.filter(m => m.dir !== "info" && weighted(m.id)).length;
  const searchRows: Row[] = [
    ["Algorithm", algoName(s.algo), true, "Search strategy"],
    ["Starting layout", start?.name ?? (i.label || "Auto"), true, s.start && s.start !== "auto" ? "Named layout" : "Loaded layout / current edits"],
    ["Starting symbols", start?.chars ?? i.chars, true, "Slot order"],
    ["Pinned symbols", i.pins || "None", !!i.pins, "Held in their starting slots"],
    ["Budget", s.budget ?? "time", true, "Stopping rule"],
    ["Time", `${s.timeS} s`, s.budget !== "evals", "Wall-clock budget"],
    ["Evaluations", s.evals ?? 50_000_000, s.budget === "evals", "Evaluation budget"],
    ["Threads", s.threads, true, `${meta.threads} available cores`],
    ["Seed", s.seed, true, "Random seed"],
    ["SA steps / restart", s.saSteps || "Auto", ["sa", "rwsa"].includes(s.algo), "Simulated / random-weight annealing"],
    ["GA population", s.gaPop, s.algo === "ga", "Genetic algorithm"],
    ["Population / island", s.memeticPop ?? 24, memetic, "Memetic islands"],
    ["Local search", s.localSearch ?? "hc", memetic && (s.localSteps ?? 12) > 0, "Child refinement"],
    ["Local steps", s.localSteps ?? 12, memetic, "Refinement cap; 0 disables"],
    ["Islands", s.islands || `Auto (${s.threads})`, memetic, "Memetic islands"],
    ["Migration interval", s.migrationInterval ?? 5, memetic && (s.migrationInterval ?? 5) > 0, "Generations; 0 disables"],
    ["Migrants", s.migrants ?? 2, memetic && (s.migrationInterval ?? 5) > 0 && (s.migrants ?? 2) > 0, "Layouts exchanged"],
    ["Stop confidence", s.stopConf == null ? "Off" : `${number(s.stopConf * 100)}%`, false, "Saved only; not implemented"],
  ];
  const profileRows: Row[] = [
    ["Profile", p.name, true, "Current edits included"],
    ["Trigram coverage", `${number(p.trigram_coverage * 100)}%`, true, "Most frequent trigrams retained"],
    ["Skip decay", skipDecay, weighted("sfs_far"), "Gap 2 × decay; gap 3 × decay²"],
    ["Symmetric finger effort", e?.symmetric ?? false, !!e, "Left costs mirrored to right when on"],
    ["Thumb target", `${number(e ? targets[4] + targets[5] : p.thumb_target ?? 18)}%`, weighted("thumb_load"), e ? "Derived from typing effort" : "Profile target"],
    ["Familiarity reference", p.familiar_to ?? "None", weighted("familiar") && !!p.familiar_to, "Familiarity needs a QWERTY reference"],
    ...["Pinky", "Ring", "Middle", "Index"].flatMap((f, n) => ["Above", "Below"].map((dir, k): Row => [`${f} · ${dir.toLowerCase()} home`, p.row_offset?.[n]?.[k] ?? 0, weighted("effort") && (p.row_offset?.[n]?.[k] ?? 0) !== 0, "Added row effort"])),
  ];
  return <div className="input-summary">
    <header className="input-summary-heading"><div><h1>Input summary</h1><p>Optimizer reference sheet · current inputs for your next layout search</p></div><span>{active} / {meta.metrics.length} metrics weighted</span></header>
    <p className="hint">Gray rows are inactive, unweighted, or preview-only. Weighted metrics can still contribute zero when their pattern is absent. Algorithm-specific defaults not exposed on the Layout tab remain defined by the core.</p>
    {(custom || !corpus) && <div className="notice">{custom ? "Custom boards can be previewed but cannot be optimized yet. " : ""}{!corpus ? "Select a bundled language to run optimization." : ""}</div>}
    <div className="input-summary-grid">
      <Sheet title="Layout search & stopping"><Settings rows={searchRows} /></Sheet>
      <div className="input-summary-intro">
      <Sheet title="Board & language"><Settings rows={[
        ["Board", g.name, true, `${g.key.length} physical keys`],
        ["Language", corpus?.name ?? i.corpusId, !!corpus, corpus ? "Corpus frequencies" : "Local language; runs not supported"],
        ["Corpus symbols", corpus?.charset ?? "—", !!corpus, "Symbols represented in the corpus"],
        ["Frequency channels", "Characters / bigrams / skipgrams / trigrams", true, "Normalized over layout symbols"],
        ...["engram_formula", "engram"].map((id): Row => [`${id} comfort table`, g.comfort?.[id] ?? "None", weighted(id) && !!g.comfort_tables?.[id]?.length, "Geometry-specific key-pair comfort"]),
      ]} /></Sheet>
      </div>
      <Sheet title="Scoring parameters"><Settings rows={profileRows} /></Sheet>
      <Sheet title="Finger effort & load targets"><table className="tbl summary-table"><thead><tr><th>Finger</th><th>Typing</th><th>Sideways</th><th>Vertical</th><th>Target %</th></tr></thead><tbody>
        {FINGERS.map((f, n) => { const k = e?.symmetric && n >= 5 ? 9 - n : n; const present = g.key.some(key => key.finger === f); return <tr key={f} className={present ? "" : "off"}><th scope="row">{f}{!present && " · absent"}</th>
          <td className={!weighted("finger_target", "finger_peak", "thumb_load") ? "inactive" : ""}>{value(e?.typing[k])}</td>
          <td className={!weighted("effort") ? "inactive" : ""}>{value(e?.lateral[k])}</td><td className={!weighted("effort") ? "inactive" : ""}>{value(e?.vertical[k])}</td>
          <td className={!weighted("finger_target", "finger_peak") ? "inactive" : ""}>{number(targets[n])}</td></tr>; })}
      </tbody></table><p className="hint">Typing effort generates load targets (finger_target, finger_peak, thumb_load). Sideways and vertical effort add to key effort. LP → RP: left pinky through right pinky.</p></Sheet>
      <Sheet title="Every scoring factor" wide><table className="tbl summary-table metrics-summary"><thead><tr><th>ID</th><th>Factor</th><th>Status</th><th>Weight</th><th>Scale</th><th>Weight / scale</th><th>Direction</th><th>Measures</th></tr></thead><tbody>
        {meta.metrics.map(m => { const w = p.weights[m.id] ?? 0, scale = p.scale?.[m.id] ?? 1; const missing = m.id === "familiar" && !p.familiar_to ? "No reference" : ["engram_formula", "engram"].includes(m.id) && !g.comfort_tables?.[m.id]?.length ? "No comfort table" : null; const on = w !== 0 && m.dir !== "info" && !missing; return <tr key={m.id} className={on ? "" : "off"}>
          <th scope="row">{m.id}</th><td>{m.name}</td><td>{m.dir === "info" ? "Report only" : !w ? "Off" : !on ? missing : "Weighted"}</td><td>{number(w)}</td><td>{number(scale)}</td><td>{number(w / scale)}</td><td>{m.dir === "max" ? "Reward" : m.dir === "min" ? "Penalize" : "Report"}</td><td>{m.desc} {m.unit && `(${m.unit})`}</td>
        </tr>; })}
      </tbody></table></Sheet>
      <Sheet title="Hand & simulation"><Settings rows={[
        ["Use hand in optimization", i.hand.use, i.hand.use, i.hand.use ? "Profile hand for reach / wrist / press_angle / thumb_arc" : "Off: the average hand, at the profile's weights"],
        ["Hand-model metrics", weighted("reach", "wrist", "press_angle", "thumb_arc") ? "Weighted" : "Unweighted", weighted("reach", "wrist", "press_angle", "thumb_arc"), "reach · wrist · press_angle · thumb_arc"],
        ["Hand preset", i.hand.preset, i.hand.use, i.hand.use ? "Sent with every score and run" : "Preview only"],
        ...Object.entries(handDims(i.hand)).map(([k, v]): Row => [k, `${value(v)} mm`, i.hand.use, Array.isArray(v) ? "Segments, base → tip" : "Measurement"]),
        ...Object.entries(wristOf(i.hand)).map(([k, v]): Row => [`Wrist ${k}`, v, false, "Simulation only"]),
      ]} /></Sheet>
      <Sheet title="Physical constraints · saved only"><Settings rows={CONSTRAINTS.flatMap(([id, title, , fields]) => fields.map(([key, label, , fallback, , , , unit]): Row => [
        `${title} · ${label}`, `${value(i.constraints[id]?.[key] ?? fallback)} ${unit}`.trim(), false,
        `${i.constraints[id]?.on ? "Enabled" : "Disabled"} in Board; generation not implemented`,
      ]))} /></Sheet>
      <Sheet title="Per-key geometry" wide><p className="hint">Fixed board inputs: position in key units, rotation in degrees, size in key units. Home / stretch flags and logical rows / columns feed the metric definitions; size and rotation also describe the physical board.</p><table className="tbl summary-table"><thead><tr><th>Key</th><th>Finger</th><th>Position x / y / z</th><th>Rotation x / y / z</th><th>Size</th><th>Row / col</th><th>Home</th><th>Stretch</th><th>Effort</th><th>Home-row mod</th><th>Fill</th></tr></thead><tbody>
        {g.key.map((k, n) => <tr key={n}><th scope="row">{n + 1}</th><td>{k.finger}</td><td>{value(k.pos)}</td><td>{value(k.rot)}</td><td>{value(k.size)}</td><td>{k.row} / {k.col}</td><td>{value(k.home)}</td><td>{value(k.stretch)}</td><td>{number(k.effort)}</td><td>{value(k.hrm ?? (k.home && !k.stretch && !["LT", "RT"].includes(k.finger)))}</td><td>{k.fill ?? "—"}</td></tr>)}
      </tbody></table></Sheet>
    </div>
  </div>;
}
