// Board tab (frontend.md "Board"): board cards or custom constraints | 3D model with
// auto-rotate and per-key editing | quick stats and a rough goodness estimate.

import { useMemo, useState } from "react";
import { FINGERS, type Finger, type Geometry, type Key } from "../../api";
import { fingerColor, INK } from "../../colors";
import { boardStats, boardScore } from "../../board";
import type { Constraints } from "../../inputs";
import { Keyboard3D, type ViewMode } from "../../three/Keyboard3D";
import { flat2d } from "../../wasm";
import { FitBoard } from "../Board2D";
import { Frame, Num, Sec, Slider, Toggle } from "../Form";

interface Props {
  geometries: Geometry[];
  customBoards: Geometry[];
  setCustomBoards: (g: Geometry[]) => void;
  geometry: Geometry;
  setGeomId: (id: string) => void;
  constraints: Constraints;
  setConstraints: (c: Constraints) => void;
  custom: boolean;
  setCustom: (on: boolean) => void;
}

/** Constraint groups for building a board: [id, title, note, fields [key, label, help, default, min, max, step, unit]]. */
type Field = [string, string, string, number, number, number, number, string];
const CONSTRAINTS: [string, string, string, Field[]][] = [
  ["grid", "Rows and columns", "main key matrix", [
    ["rows", "Rows", "Rows in the main matrix (thumb cluster excluded).", 3, 1, 6, 1, ""],
    ["cols", "Columns / hand", "Columns per hand, outer pinky column included.", 5, 3, 8, 1, ""],
  ]],
  ["stagger", "Stagger", "column offsets", [
    ["index", "Index", "Vertical offset of the index column from the pinky column.", 0.25, -1, 1, 0.05, "u"],
    ["middle", "Middle", "Vertical offset of the middle column.", 0.5, -1, 1, 0.05, "u"],
    ["ring", "Ring", "Vertical offset of the ring column.", 0.25, -1, 1, 0.05, "u"],
  ]],
  ["splay", "Splay", "column fan-out", [
    ["angle", "Max splay", "Rotation of the outermost column relative to the index column.", 8, 0, 30, 1, "°"],
  ]],
  ["thumb", "Thumb cluster", "keys under the thumb", [
    ["keys", "Keys / hand", "Thumb keys per hand.", 3, 0, 6, 1, ""],
    ["arc", "Arc radius", "Radius of the arc the thumb keys sit on.", 4, 1, 10, 0.5, "u"],
  ]],
  ["spacing", "Key spacing", "centre to centre", [
    ["pitch", "Pitch", "Distance between key centres (1u = 19.05 mm).", 19.05, 16, 22, 0.05, "mm"],
  ]],
  ["fingers", "Keys per finger", "workload limits", [
    ["pinky", "Pinky", "Most keys a pinky may own.", 6, 1, 12, 1, ""],
    ["index", "Index", "Most keys an index finger may own.", 8, 1, 12, 1, ""],
  ]],
  ["size", "Size limits", "physical footprint", [
    ["width", "Max width", "Widest the board (both halves) may be.", 300, 150, 500, 5, "mm"],
    ["depth", "Max depth", "Deepest the board may be.", 120, 60, 200, 5, "mm"],
    ["keys", "Max keys", "Total key budget.", 42, 30, 100, 1, ""],
  ]],
];

function ConstraintForm({ c, set }: { c: Constraints; set: (c: Constraints) => void }) {
  return (
    <>
      <div className="pad"><div className="notice">Constraints for building a board, used when optimizing the board itself (planned). They are saved but not applied yet.</div></div>
      {CONSTRAINTS.map(([id, title, note, fields]) => {
        const g = c[id] ?? { on: false };
        const put = (patch: Record<string, number | boolean>) => set({ ...c, [id]: { ...g, ...patch } as Constraints[string] });
        return (
          <Sec key={id} id={`con-${id}`} title={title} note={note}>
            <Toggle label="Enabled" help={`Apply the ${title.toLowerCase()} constraint.`} value={!!g.on} onChange={(on) => put({ on })} />
            {fields.map(([k, label, help, def, min, max, step, unit]) => {
              const v = typeof g[k] === "number" ? (g[k] as number) : def;
              return max - min <= 40 && step < 1
                ? <Slider key={k} label={label} help={help} value={v} min={min} max={max} step={step} unit={`${v}${unit}`} disabled={!g.on} onChange={(x) => put({ [k]: x })} />
                : <Num key={k} label={label} help={help} value={v} min={min} max={max} step={step} unit={unit} disabled={!g.on} onChange={(x) => put({ [k]: Math.min(max, Math.max(min, x)) })} />;
            })}
          </Sec>
        );
      })}
    </>
  );
}

function Cards({ boards, sel, onPick }: { boards: Geometry[]; sel: string; onPick: (id: string) => void }) {
  return (
    <div className="boardcards">
      {boards.map((g) => (
        <button key={g.id} className={`boardcard ${g.id === sel ? "sel" : ""}`} onClick={() => onPick(g.id)} title={g.description}>
          <div className="boardcard-img">
            <FitBoard geometry={g} max={14} pad={2} cells={(i) => ({ fill: g.key[i].home ? "#e8e8e8" : "#ffffff", text: INK, main: "" })} />
          </div>
          <span className="boardcard-cap">{g.name} · {g.key.length}</span>
        </button>
      ))}
    </div>
  );
}

/** Key editor popup: position, rotation and finger of one key. */
function KeyPopup({ k, i, at, onChange, onClose }: { k: Key; i: number; at: { x: number; y: number }; onChange: (k: Key) => void; onClose: () => void }) {
  const num = (label: string, v: number, set: (v: number) => void, unit: string, help: string) => (
    <Num label={label} help={help} value={+v.toFixed(3)} step={unit === "°" ? 1 : 0.05} unit={unit} onChange={set} />
  );
  const p = k.pos, r = k.rot;
  return (
    <div className="keypop" style={{ left: at.x, top: at.y }} onPointerDown={(e) => e.stopPropagation()}>
      <div className="keypop-head"><b>Key {i + 1}</b><span className="hint">row {k.row} · col {k.col}</span><button className="btn soft small" onClick={onClose}>Done</button></div>
      {num("x", p[0], (v) => onChange({ ...k, pos: [v, p[1], p[2]] }), "u", "Left edge, in key units (1u = 19.05 mm).")}
      {num("y", p[1], (v) => onChange({ ...k, pos: [p[0], v, p[2]] }), "u", "Top edge, toward the user.")}
      {num("z", p[2], (v) => onChange({ ...k, pos: [p[0], p[1], v] }), "u", "Height above the base plane.")}
      {num("rx", r[0], (v) => onChange({ ...k, rot: [v, r[1], r[2]] }), "°", "Tilt about the x axis (tenting the row).")}
      {num("ry", r[1], (v) => onChange({ ...k, rot: [r[0], v, r[2]] }), "°", "Tilt about the y axis (tenting).")}
      {num("rz", r[2], (v) => onChange({ ...k, rot: [r[0], r[1], v] }), "°", "Rotation in the plane (splay).")}
      <div className="row">
        <label title="Finger that presses this key.">Finger</label>
        <select value={k.finger} onChange={(e) => onChange({ ...k, finger: e.target.value as Finger })}>
          {FINGERS.map((f) => <option key={f} value={f}>{f}</option>)}
        </select>
        <span className="unit"><i className="swatch" style={{ background: fingerColor(k.finger) }} /></span>
      </div>
    </div>
  );
}

function StatsPanel({ g, all }: { g: Geometry; all: Geometry[] }) {
  const s = boardStats(g);
  const score = boardScore(s);
  const ranked = useMemo(() => all.map((x) => ({ g: x, s: boardScore(boardStats(x)) })).sort((a, b) => b.s.total - a.s.total), [all]);
  const kv = (k: string, v: string) => <div className="kv"><span>{k}</span><span>{v}</span></div>;
  return (
    <div className="bottomgrid">
      <div>
        <h4>Quick stats</h4>
        {kv("Keys", `${s.keys}`)}
        {kv("Form", s.split ? "split" : "unibody")}
        {kv("Rows × columns", `${s.rows} × ${s.cols}`)}
        {kv("Thumb keys", `${s.thumbs}`)}
        {kv("Stagger", s.stagger)}
        {kv("Size", `${s.widthMm.toFixed(0)} × ${s.depthMm.toFixed(0)} mm`)}
      </div>
      <div>
        <h4>Estimate <span className="hint">rough, from geometry only</span></h4>
        <div className="score"><b>{score.total.toFixed(0)}</b><span className="muted">/ 100</span></div>
        <ScoreBar label="Reach and comfort" v={score.comfort} help={`Mean base effort ${s.meanEffort.toFixed(2)} over the non-thumb keys; ${s.homeKeys} home keys.`} />
        <ScoreBar label="Even spread" v={score.spread} help={`How evenly keys divide over the fingers (normalized entropy ${s.evenness.toFixed(2)}).`} />
        <ScoreBar label="Thumb zone" v={score.thumb} help={`${s.thumbs} thumb keys; 2–3 per hand is ideal.`} />
        <ScoreBar label="Compactness" v={score.compact} help={`${s.stretchKeys} keys need a stretch.`} />
      </div>
      <div>
        <h4>Versus the other boards</h4>
        <table className="tbl">
          <thead><tr><th>board</th><th>keys</th><th>estimate</th></tr></thead>
          <tbody>
            {ranked.map((r) => (
              <tr key={r.g.id} className={r.g.id === g.id ? "sel" : ""}><td>{r.g.name}</td><td>{r.g.key.length}</td><td>{r.s.total.toFixed(0)}</td></tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function ScoreBar({ label, v, help }: { label: string; v: number; help: string }) {
  return (
    <div className="kv" title={help}><span>{label}</span><span className="bar"><i style={{ width: `${Math.round(100 * v)}%` }} /><em>{Math.round(100 * v)}</em></span></div>
  );
}

export function BoardView(p: Props) {
  const [spin, setSpin] = useState(1);
  const [mode, setMode] = useState<ViewMode>("3d");
  const [fingers, setFingers] = useState(true);
  const [edit, setEdit] = useState<{ i: number; at: { x: number; y: number } } | null>(null);
  const g = p.geometry;
  const all = [...p.geometries, ...p.customBoards];

  const pick = (id: string) => { p.setGeomId(id); setSpin((s) => s + 1); setEdit(null); };
  /** First edit of a bundled board forks it to `<name> (custom)`; later edits change the fork. */
  const editKey = (i: number, k: Key) => {
    const keys = g.key.map((x, j) => (j === i ? k : x));
    const isCustom = p.customBoards.some((c) => c.id === g.id);
    const base: Geometry = isCustom ? g : { ...g, id: `custom-${g.id}-${Date.now().toString(36)}`, name: `${g.name} (custom)` };
    let next: Geometry = { ...base, key: keys };
    try { next = { ...next, flat: flat2d(next) }; } catch { /* keep the old drawing */ }
    p.setCustomBoards(isCustom ? p.customBoards.map((c) => (c.id === g.id ? next : c)) : [...p.customBoards, next]);
    if (!isCustom) p.setGeomId(next.id);
  };
  const removeCustom = () => {
    p.setCustomBoards(p.customBoards.filter((c) => c.id !== g.id));
    p.setGeomId(p.geometries[0].id);
  };
  const isCustom = p.customBoards.some((c) => c.id === g.id);

  return (
    <Frame
      title="Boards"
      control={<label className="switchlabel" title="Show the physical constraints for building a board instead of the bundled boards.">Custom <input type="checkbox" className="switch" checked={p.custom} onChange={(e) => p.setCustom(e.target.checked)} /></label>}
      side={p.custom ? <ConstraintForm c={p.constraints} set={p.setConstraints} /> : <Cards boards={all} sel={g.id} onPick={pick} />}
      footer={isCustom && !p.custom ? <button className="btn soft small" onClick={removeCustom}>Delete {g.name}</button> : undefined}
      main={
        <div className="viewport" onPointerDown={() => setEdit(null)}>
          <Keyboard3D geometry={g} chars={""} heat={[]} arcs={[]} overlay={fingers ? "fingers" : "none"} mode={mode} spin={spin}
            selected={edit?.i ?? null}
            onKey={(i, at) => {
              if (i < 0 || !at) { setEdit(null); return; }
              const box = document.querySelector(".frame-main")!.getBoundingClientRect();
              setEdit({ i, at: { x: Math.min(at.x - box.left + 12, box.width - 250), y: Math.max(8, Math.min(at.y - box.top - 20, box.height - 330)) } });
            }} />
          <div className="viewlabel"><span>{g.name}</span><span className="muted">{g.key.length} keys</span></div>
          <div className="viewctl" onPointerDown={(e) => e.stopPropagation()}>
            <div className="seg inline"><button className={mode === "3d" ? "on" : ""} onClick={() => setMode(mode === "3d" ? "top" : "3d")} title="3D view (off: flat top-down)">3D</button></div>
            <div className="seg inline"><button className={fingers ? "on" : ""} onClick={() => setFingers(!fingers)} title="Color keys by finger">Fingers</button></div>
          </div>
          <div className="viewhint">{isCustom ? "Custom board (this browser only)." : "Click a key to edit it; the first edit makes a custom copy."}</div>
          {edit && g.key[edit.i] && <KeyPopup k={g.key[edit.i]} i={edit.i} at={edit.at} onChange={(k) => editKey(edit.i, k)} onClose={() => setEdit(null)} />}
        </div>
      }
      bottom={<div className="bottompad"><StatsPanel g={g} all={all} /></div>}
    />
  );
}
