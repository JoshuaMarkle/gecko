// Search tab, "Board + layout" (web-screens.md "Search"): which board parameters move and how
// far (sidebar groups, bottom bounds table), and a 3D preview of the board with one parameter
// swept across its bounds, collisions marked as the search would count them.

import { useEffect, useMemo, useState } from "react";
import { api, type Geometry, type GeomVar } from "../../api";
import { fitChars, fmt } from "../../format";
import { GEOM_GROUPS, groupSpec, PRESET_2D, PRESET_3D, varLabel, withBounds, type GeomSettings } from "../../geom";
import { Keyboard3D, type ViewMode } from "../../three/Keyboard3D";
import { Num, Sec, Seg, Toggle } from "../Form";
import { collisionColors, stableFocus, useParamBoard } from "./GeomBoard";

/** Free parameters of `geometry` for the selected groups (default bounds), from the server. */
export function useGeomVars(geometry: Geometry, groups: string[]): { vars: GeomVar[]; error: string | null } {
  const spec = groupSpec(groups);
  const [state, setState] = useState<{ key: string; vars: GeomVar[]; error: string | null }>({ key: "", vars: [], error: null });
  const key = `${geometry.id}|${spec}`;
  useEffect(() => {
    if (!geometry.param || !spec) { setState({ key, vars: [], error: null }); return; }
    let alive = true;
    api.geometryFree(geometry.id, spec)
      .then((r) => alive && setState({ key, vars: r.vars, error: null }))
      .catch((e) => alive && setState({ key, vars: [], error: (e as Error).message }));
    return () => { alive = false; };
  }, [key, geometry.id, geometry.param, spec]);
  return state.key === key ? { vars: state.vars, error: state.error } : { vars: [], error: null };
}

/** Sidebar sections: what moves, and the search's own settings. */
export function GeomSide({ s, set }: { s: GeomSettings; set: (patch: Partial<GeomSettings>) => void }) {
  const has = (g: string) => s.groups.includes(g);
  const toggle = (g: string, on: boolean) => set({ groups: on ? [...s.groups, g] : s.groups.filter((x) => x !== g) });
  const preset = PRESET_3D.every(has) && s.groups.length === PRESET_3D.length ? "3d"
    : PRESET_2D.every(has) && s.groups.length === PRESET_2D.length ? "2d" : "custom";
  return <>
    <Sec id="geom-free" title="Board shape" note="what may move">
      <div className="row">
        <label title="2D moves keys in the plane (stagger, splay, spread, thumbs); 3D also curves, lifts, tents and tilts.">Preset</label>
        <Seg value={preset} onChange={(v) => v !== "custom" && set({ groups: v === "3d" ? PRESET_3D : PRESET_2D })}
          options={[["2d", "2D"], ["3d", "3D"], ["custom", "Custom"]]} />
        <span />
      </div>
      {GEOM_GROUPS.map(([g, label, help, three]) => (
        <Toggle key={g} label={three ? `${label} · 3D` : label} help={help} value={has(g)} onChange={(on) => toggle(g, on)} />
      ))}
    </Sec>
    <Sec id="geom-settings" title="Board search settings" note="CMA-ES over the board">
      <div className="row">
        <label title="Bilevel: every candidate board gets a short layout search. Alternate: board steps with the layout fixed, then a full layout search, for a few rounds.">Mode</label>
        <select value={s.mode} onChange={(e) => set({ mode: e.target.value as GeomSettings["mode"] })}>
          <option value="bilevel">Bilevel</option><option value="alternate">Alternate</option>
        </select>
        <span />
      </div>
      <Num label="Layout swaps / board" help="Bilevel: layout swaps tried on each candidate board, starting from the best layout so far." value={s.innerEvals} min={0} step={5000} onChange={(v) => set({ innerEvals: Math.max(0, Math.round(v)) })} />
      <Num label="Clearance" help="Extra space kept around each keycap (per side). 0 lets caps touch." value={s.margin} min={0} step={0.01} unit="u" onChange={(v) => set({ margin: Math.max(0, v) })} />
      <Num label="Collision penalty" help="Score added per u of keycap overlap, in units of the start score. Higher keeps the search away from colliding boards." value={s.penalty} min={0} step={1} onChange={(v) => set({ penalty: Math.max(0, v) })} />
      <Toggle label="Keep key efforts" help="Keep the board file's hand-entered key efforts instead of deriving them from each candidate's key positions." value={s.keepEffort} onChange={(v) => set({ keepEffort: v })} />
    </Sec>
  </>;
}

/** Main: the board with parameter `sel` set to `value` (others at their start), over the start board's outline. */
export function GeomPreview({ geometry, vars, sel, value, setValue, chars }: {
  geometry: Geometry; vars: GeomVar[]; sel: number | null; value: number; setValue: (v: number) => void; chars: string;
}) {
  const [mode, setMode] = useState<ViewMode>("3d");
  const [sweep, setSweep] = useState(false);
  const v = sel !== null ? vars[sel] : undefined;
  const names = useMemo(() => vars.map((x) => x.name), [vars]);
  const values = useMemo(() => vars.map((x, i) => (i === sel ? value : x.start)), [vars, sel, value]);
  const start = useParamBoard(geometry.param, names, vars.map((x) => x.start), geometry.id);
  const shown = useParamBoard(geometry.param, names, values, geometry.id);
  const focus = useMemo(() => stableFocus(start?.geometry.key.length ? start.geometry.key : geometry.key), [start, geometry.key]);
  // Sweep: move the value back and forth across its bounds.
  useEffect(() => {
    if (!sweep || !v) return;
    let t = 0;
    const id = setInterval(() => { t += 0.04; setValue(v.lo + (v.hi - v.lo) * (0.5 - 0.5 * Math.cos(t))); }, 40);
    return () => clearInterval(id);
  }, [sweep, v, setValue]);
  if (!geometry.param) {
    return <div className="empty"><div className="notice" style={{ maxWidth: 420 }}>{geometry.name} has no parameter file, so its shape cannot be searched. Pick a parametric board (Corne 36, Ortho 30) or the board of a finished board + layout run on the Board tab.</div></div>;
  }
  const board = shown && !shown.error ? shown.geometry : geometry;
  const n = shown?.collisions.length ?? 0;
  const [label, unit] = v ? varLabel(v.name) : ["", ""];
  return (
    <div className="viewport">
      <Keyboard3D geometry={board} chars={fitChars(chars, board)} heat={[]} arcs={[]} overlay="none" mode={mode} selected={null} onKey={() => {}}
        focus={focus} colors={shown ? collisionColors(board.key.length, shown.collisions) : undefined}
        ghost={v && value !== v.start ? start?.geometry.key : undefined} />
      <div className="viewlabel">
        <span>{v ? `${label} = ${fmt(value, 2)}${unit}` : "Start board · pick a parameter below to preview it"}</span>
        {shown?.error ? <span className="err">{shown.error}</span> : <span className={n ? "err" : "muted"}>{n ? `${n} collision${n === 1 ? "" : "s"}` : "no collisions"}</span>}
      </div>
      <div className="viewctl">
        <div className="seg inline"><button className={mode === "3d" ? "on" : ""} onClick={() => setMode(mode === "3d" ? "top" : "3d")} title="3D view (off: flat top-down)">3D</button></div>
      </div>
      {v && (
        <div className="legend geomsweep">
          <span className="mono">{fmt(v.lo, 2)}</span>
          <input type="range" min={v.lo} max={v.hi} step={(v.hi - v.lo) / 200} value={value} onChange={(e) => { setSweep(false); setValue(parseFloat(e.target.value)); }} title="Preview this parameter's value" />
          <span className="mono">{fmt(v.hi, 2)}</span>
          <button className={sweep ? "" : "off"} onClick={() => setSweep(!sweep)} title="Sweep across the bounds">Sweep</button>
          <button onClick={() => { setSweep(false); setValue(v.start); }} title="Back to the board's value">Reset</button>
          <span><i style={{ background: "transparent", border: "1px dashed #707070" }} />start</span>
        </div>
      )}
    </div>
  );
}

/** Bottom: every free parameter with its start value and editable bounds; click a row to preview it. */
export function GeomParams({ s, set, board, vars, error, sel, onSel }: {
  s: GeomSettings; set: (patch: Partial<GeomSettings>) => void; board: string; vars: GeomVar[]; error: string | null;
  sel: number | null; onSel: (i: number) => void;
}) {
  const shown = withBounds(s, board, vars);
  const setBound = (name: string, lo: number, hi: number) => set({ bounds: { ...s.bounds, [`${board}|${name}`]: [lo, hi] } });
  const edited = vars.some((v) => s.bounds[`${board}|${v.name}`]);
  const reset = () => set({ bounds: Object.fromEntries(Object.entries(s.bounds).filter(([k]) => !k.startsWith(`${board}|`))) });
  return (
    <div className="bottompad">
      <div className="paneltabs">
        <h4 style={{ margin: 0 }}>Free parameters <span className="hint">{vars.length} · click one to preview it</span></h4>
        <span className="right"><button className="btn soft small" disabled={!edited} onClick={reset}>Default bounds</button></span>
      </div>
      {error && <div className="notice error">{error}</div>}
      {!error && !vars.length && <div className="hint">Turn on a group under Board shape.</div>}
      {vars.length > 0 && (
        <table className="tbl geomparams">
          <thead><tr><th>parameter</th><th>lower</th><th>start</th><th>upper</th><th>range</th></tr></thead>
          <tbody>
            {shown.map((v, i) => {
              const [label, unit] = varLabel(v.name);
              const d = vars[i];
              return (
                <tr key={v.name} className={`click ${sel === i ? "sel" : ""}`} onClick={() => onSel(i)} title={v.name}>
                  <td className="l">{label}</td>
                  <td><input className="cell" type="number" value={round3(v.lo)} max={v.start} step={unit === "°" ? 1 : 0.05}
                    onClick={(e) => e.stopPropagation()} onChange={(e) => { const x = parseFloat(e.target.value); if (Number.isFinite(x) && x <= v.start && x < v.hi) setBound(v.name, x, v.hi); }} /></td>
                  <td>{fmt(v.start, 2)}{unit}</td>
                  <td><input className="cell" type="number" value={round3(v.hi)} min={v.start} step={unit === "°" ? 1 : 0.05}
                    onClick={(e) => e.stopPropagation()} onChange={(e) => { const x = parseFloat(e.target.value); if (Number.isFinite(x) && x >= v.start && x > v.lo) setBound(v.name, v.lo, x); }} /></td>
                  <td><RangeBar lo={v.lo} hi={v.hi} start={v.start} dlo={d.lo} dhi={d.hi} /></td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}
    </div>
  );
}

const round3 = (x: number) => Math.round(x * 1000) / 1000;

/** Bounds as a bar on the span of the default and edited bounds; tick at the start value. */
export function RangeBar({ lo, hi, start, dlo, dhi, best, w = 120 }: { lo: number; hi: number; start: number; dlo?: number; dhi?: number; best?: number; w?: number }) {
  const a = Math.min(lo, dlo ?? lo), b = Math.max(hi, dhi ?? hi);
  const x = (t: number) => 2 + ((t - a) / (b - a || 1)) * (w - 4);
  return (
    <svg width={w} height={12} style={{ verticalAlign: "middle" }}>
      <line x1={x(a)} x2={x(b)} y1={6} y2={6} stroke="#e0e0e0" strokeWidth={2} />
      <line x1={x(lo)} x2={x(hi)} y1={6} y2={6} stroke="#adadad" strokeWidth={4} />
      <line x1={x(start)} x2={x(start)} y1={1} y2={11} stroke="#141414" strokeWidth={1.5} />
      {best !== undefined && <circle cx={x(best)} cy={6} r={3.5} fill="#0066ff" />}
    </svg>
  );
}
