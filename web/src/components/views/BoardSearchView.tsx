// Board and Both tabs (web-screens.md "Board · Both"): search a parametric board's shape.
// Board keeps the layout fixed (geometry run mode "board"); Both searches board and layout
// together (bilevel or alternate). Shape groups and settings | the board previewed with one
// parameter swept, or the live best board during a run | the free parameters and their bounds.

import { useEffect, useState } from "react";
import type { Geometry, GeomVar, Key, LayoutInfo, Meta } from "../../api";
import { fitChars, fmtCount } from "../../format";
import { GEOM_DEFAULTS, type GeomSettings } from "../../geom";
import type { SearchSettings } from "../../inputs";
import { Keyboard3D, type Focus } from "../../three/Keyboard3D";
import { Frame, Num, Sec, Seg } from "../Form";
import { GeomParams, GeomPreview, GeomSide } from "../geom/GeomSearch";
import { StartSec, ThreadsSeed } from "./LayoutView";

/** The live best board of a running board search, as App builds it. */
export interface LiveBoard { geometry: Geometry; ghost?: Key[]; colors?: (string | undefined)[]; focus?: Focus; caption: string; collisions: number }

interface Props {
  target: "board" | "both";
  meta: Meta;
  layouts: LayoutInfo[];
  geometry: Geometry;
  search: SearchSettings;
  setSearch: (s: SearchSettings) => void;
  /** A run of any kind is live: settings are locked. */
  locked: boolean;
  /** Free parameters of the board for the selected groups (`useGeomVars`). */
  geomVars: { vars: GeomVar[]; error: string | null };
  /** The profile weights a hand-model metric (G01 / G03 / G05): without one, board shape barely matters. */
  handWeighted: boolean;
  /** The layout the run starts from (Board: keeps). */
  chars: string;
  label: string;
  pins: string;
  onLoad: (chars: string, label: string) => void;
  /** A board search of this tab's kind is running. */
  liveBoard?: LiveBoard;
}

export function BoardSearchView(p: Props) {
  const s = p.search;
  const set = (patch: Partial<SearchSettings>) => p.setSearch({ ...s, ...patch });
  const g = { ...GEOM_DEFAULTS, ...s.geom };
  const setGeom = (patch: Partial<GeomSettings>) => set({ geom: { ...g, ...patch } });
  const budget = g.budget ?? "time";
  const board = p.target === "board";
  // Board preview: the selected free parameter and its shown value.
  const [sel, setSel] = useState<number | null>(null);
  const [pv, setPv] = useState(0);
  const vars = p.geomVars.vars;
  useEffect(() => { setSel(null); }, [p.geometry.id, vars.length]);
  const pick = (i: number) => { setSel(i); setPv(vars[i].start); };
  const lb = p.liveBoard;

  return (
    <Frame
      title={board ? "Optimize board" : "Optimize board + layout"}
      control={<span className="hint">{p.locked ? "locked while running" : board ? "for the chosen layout" : "together"}</span>}
      side={
        <fieldset className="plain" disabled={p.locked}>
          {(!p.geometry.param || !p.handWeighted) && (
            <div className="sidenotes">
              {!p.geometry.param && <div className="notice">{p.geometry.name} has no parameter file: pick a parametric board on the Board tab to run.</div>}
              {!p.handWeighted && <div className="notice">The profile weights no hand-model metric, so the board's shape only moves distance metrics. Turn on "Use in optimization" on the Hand tab.</div>}
            </div>
          )}
          <StartSec layouts={p.layouts} start={s.start ?? "auto"} setStart={(v) => set({ start: v })} label={p.label} pins={board ? "" : p.pins} onLoad={p.onLoad}
            title={board ? "Layout" : "Starting layout"} note={board ? "kept fixed" : "where runs begin"} />
          <GeomSide s={g} set={setGeom} target={p.target} />
          <Sec id="geom-budget" title="Budget" note="how long, how wide">
            <div className="row">
              <label title="Stop after a wall-clock time or after a number of candidate boards (exactly reproducible).">Budget</label>
              <Seg value={budget} onChange={(b) => setGeom({ budget: b })} options={[["time", "Time"], ["evals", "Boards"]]} />
              <span />
            </div>
            {budget === "time"
              ? <Num label="Time" help={board ? "Wall-clock budget for the run." : "Wall-clock budget for the run, final layout polish of both boards included (a minute or more is useful)."} value={g.timeS ?? 60} min={1} step={10} unit="s" onChange={(v) => setGeom({ timeS: Math.max(1, v) })} />
              : <Num label="Boards" help={board ? "Candidate boards to score with the layout fixed." : "Candidate boards to score; each gets its own layout search."} value={g.evals} min={10} step={50} unit={fmtCount(g.evals)} onChange={(v) => setGeom({ evals: Math.max(10, Math.round(v)) })} />}
            <ThreadsSeed s={s} set={set} cores={p.meta.threads} />
          </Sec>
        </fieldset>
      }
      main={lb ? (
        <div className="viewport">
          <Keyboard3D geometry={lb.geometry} chars={fitChars(p.chars, lb.geometry)} heat={[]} arcs={[]} overlay="none" mode="3d" selected={null} onKey={() => {}}
            colors={lb.colors} ghost={lb.ghost} focus={lb.focus} />
          <div className="viewlabel">
            <span>{lb.caption}</span>
            <span className={lb.collisions ? "err" : "muted"}>{lb.collisions ? `${lb.collisions} collisions` : "no collisions"}</span>
          </div>
        </div>
      ) : <GeomPreview geometry={p.geometry} vars={vars} sel={sel} value={pv} setValue={setPv} chars={p.chars} />}
      bottom={<GeomParams s={g} set={setGeom} board={p.geometry.id} vars={vars} error={p.geomVars.error} sel={sel} onSel={pick} />}
    />
  );
}
