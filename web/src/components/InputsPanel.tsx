// Left panel: inputs grouped by a segmented control (Layout, Board, Corpus, Profile, Search).

import { useState, type ReactNode } from "react";
import type { CorpusInfo, Geometry, LayoutInfo, Meta, Profile } from "../api";
import { checkLayout, fmtCount, rowsOf } from "../format";

export type Group = "layout" | "board" | "corpus" | "profile" | "search";
const GROUPS: [Group, string][] = [["layout", "Layout"], ["board", "Board"], ["corpus", "Corpus"], ["profile", "Profile"], ["search", "Search"]];

const ALGOS: [string, string][] = [["sa", "Simulated annealing"], ["hc", "Hill climbing"], ["ga", "Genetic algorithm"]];

export interface SearchSettings { algo: string; timeS: number; threads: number; seed: number; saSteps: number; gaPop: number }

interface Props {
  group: Group;
  setGroup: (g: Group) => void;
  meta: Meta;
  layouts: LayoutInfo[];
  geometries: Geometry[];
  corpora: CorpusInfo[];
  profiles: Profile[];
  geometry: Geometry;
  corpus: CorpusInfo | undefined;
  chars: string;
  setChars: (c: string, label: string) => void;
  startChars: string;
  pins: string;
  setPins: (p: string) => void;
  refId: string;
  setRefId: (id: string) => void;
  setGeomId: (id: string) => void;
  setCorpusId: (id: string) => void;
  profileId: string;
  setProfileId: (id: string) => void;
  profile: Profile;
  setProfile: (p: Profile) => void;
  search: SearchSettings;
  setSearch: (s: SearchSettings) => void;
  running: boolean;
}

function Sec({ id, title, note, children }: { id: string; title: string; note?: string; children: ReactNode }) {
  const key = `gecko.sec.${id}`;
  const [open, setOpen] = useState(() => {
    try { return localStorage.getItem(key) !== "0"; } catch { return true; }
  });
  return (
    <details className="sec" open={open} onToggle={(e) => {
      const o = (e.target as HTMLDetailsElement).open;
      setOpen(o);
      try { localStorage.setItem(key, o ? "1" : "0"); } catch { /* storage unavailable */ }
    }}>
      <summary>{title}{note && <span className="note">{note}</span>}</summary>
      <div className="body">{children}</div>
    </details>
  );
}

function Num({ label, help, value, onChange, unit, step = 1, min, max }: {
  label: string; help: string; value: number; onChange: (v: number) => void; unit?: string; step?: number; min?: number; max?: number;
}) {
  return (
    <div className="row">
      <label title={help}>{label}</label>
      <input type="number" value={Number.isFinite(value) ? value : ""} step={step} min={min} max={max}
        onChange={(e) => { const v = parseFloat(e.target.value); if (Number.isFinite(v)) onChange(v); }} />
      <span className="unit">{unit}</span>
    </div>
  );
}

function LayoutGroup(p: Props) {
  const [text, setText] = useState<string | null>(null);
  const rowsText = rowsOf(p.chars, p.geometry).join("\n");
  const shown = text ?? rowsText;
  const error = text === null ? null : checkLayout(text.replace(/\s/g, ""), p.geometry.key.length, p.corpus?.charset ?? "");
  return (
    <>
      <Sec id="lay-load" title="Start layout" note="the editor and runs start here">
        <div className="row">
          <label title="Load a reference layout into the editor.">Load</label>
          <select value="" onChange={(e) => { const l = p.layouts.find((x) => x.id === e.target.value); if (l) p.setChars(l.chars, l.name); }}>
            <option value="">choose a layout…</option>
            {p.layouts.map((l) => <option key={l.id} value={l.id}>{l.name}</option>)}
          </select>
          <span />
        </div>
        <div className="row">
          <label title="Every layout is compared with this one in the Result panel and charts.">Compare with</label>
          <select value={p.refId} onChange={(e) => p.setRefId(e.target.value)}>
            {p.layouts.map((l) => <option key={l.id} value={l.id}>{l.name}</option>)}
          </select>
          <span />
        </div>
      </Sec>
      <Sec id="lay-edit" title="Edit" note="click two keys to swap them">
        <div className="row wide">
          <textarea className={`mono ${error ? "invalid" : ""}`} rows={3} spellCheck={false} value={shown}
            title="Slot-ordered symbols, row by row. Whitespace is ignored."
            onChange={(e) => setText(e.target.value)}
            onBlur={() => { if (text !== null && !error) { p.setChars(text.replace(/\s/g, ""), "edited"); } setText(null); }} />
        </div>
        {error && <div className="hint" style={{ color: "var(--error)" }}>{error}</div>}
        <div className="btns">
          <button className="btn soft small" onClick={() => p.setChars(p.startChars, "start")} disabled={p.chars === p.startChars}>Undo edits</button>
        </div>
      </Sec>
      <Sec id="lay-pins" title="Pins" note="symbols that never move">
        <div className="row">
          <label title="Symbols listed here keep their current key during optimization, e.g. ,./; or zxcv.">Pinned</label>
          <input type="text" className="mono" value={p.pins} placeholder="e.g. ,./;" onChange={(e) => p.setPins(e.target.value)} />
          <span className="unit">{[...p.pins.replace(/\s/g, "")].length || ""}</span>
        </div>
      </Sec>
    </>
  );
}

function BoardGroup(p: Props) {
  const g = p.geometry;
  const rows = new Set(g.key.map((k) => k.row)).size;
  return (
    <Sec id="board" title="Geometry" note={g.name}>
      <div className="row">
        <label title="Physical board: key positions, fingers and base effort (data/geometries).">Board</label>
        <select value={g.id} onChange={(e) => p.setGeomId(e.target.value)}>
          {p.geometries.map((x) => <option key={x.id} value={x.id}>{x.name}</option>)}
        </select>
        <span />
      </div>
      <div className="kv"><span>Keys</span><span>{g.key.length} in {rows} rows</span></div>
      <div className="kv"><span>Home keys</span><span>{g.key.filter((k) => k.home).length}</span></div>
      <div className="hint">{g.description}</div>
    </Sec>
  );
}

function CorpusGroup(p: Props) {
  return (
    <Sec id="corpus" title="Corpus" note="n-gram statistics">
      <div className="row">
        <label title="Compiled n-gram tables (data/corpora, built with `gecko corpus build`).">Corpus</label>
        <select value={p.corpus?.id ?? ""} onChange={(e) => p.setCorpusId(e.target.value)}>
          {p.corpora.map((c) => <option key={c.id} value={c.id}>{c.name}</option>)}
        </select>
        <span />
      </div>
      <div className="kv"><span>Characters</span><span>{fmtCount(p.corpus?.chars ?? 0)}</span></div>
      <div className="kv"><span>Charset</span><span className="mono">{p.corpus?.charset}</span></div>
    </Sec>
  );
}

function ProfileGroup(p: Props) {
  const pr = p.profile;
  const setW = (id: string, v: number) => p.setProfile({ ...pr, weights: { ...pr.weights, [id]: Math.max(0, v) } });
  const base = p.profiles.find((x) => x.id === p.profileId);
  const changed = base && JSON.stringify({ ...base, id: undefined }) !== JSON.stringify({ ...pr, id: undefined });
  return (
    <>
      <Sec id="prof" title="Profile" note={pr.name + (changed ? " (edited)" : "")}>
        <div className="row">
          <label title="Metric weights and targets (data/profiles). Edits here re-score live and are saved with each run.">Profile</label>
          <select value={p.profileId} onChange={(e) => p.setProfileId(e.target.value)}>
            {p.profiles.map((x) => <option key={x.id} value={x.id}>{x.name}</option>)}
          </select>
          <span />
        </div>
        <div className="hint">{pr.description}</div>
        <div className="btns"><button className="btn soft small" disabled={!changed} onClick={() => base && p.setProfile(base)}>Reset weights</button></div>
      </Sec>
      <Sec id="prof-w" title="Weights" note="larger = care more">
        {p.meta.metrics.filter((m) => m.dir !== "info").map((m) => (
          <Num key={m.id} label={`${m.name}${m.dir === "max" ? " +" : ""}`} help={`${m.id} · ${m.desc}${m.dir === "max" ? " (reward)" : ""}`}
            value={pr.weights[m.id] ?? 0} step={0.05} min={0} onChange={(v) => setW(m.id, v)} unit={m.id} />
        ))}
      </Sec>
      <Sec id="prof-t" title="Targets" note="finger load and trigram fidelity">
        {pr.finger_target.map((t, i) => (i === 4 || i === 5 ? null : (
          <Num key={i} label={`Finger ${["LP", "LR", "LM", "LI", "LT", "RT", "RI", "RM", "RR", "RP"][i]}`} help="Target share of keystrokes for this finger (M02)." value={t} step={0.5} unit="%"
            onChange={(v) => { const ft = [...pr.finger_target]; ft[i] = v; p.setProfile({ ...pr, finger_target: ft }); }} />
        )))}
        <Num label="Trigram coverage" help="Keep the most frequent trigrams covering this share of trigram volume. 1 = exact, lower = faster search." value={pr.trigram_coverage} step={0.005} min={0.5} max={1}
          onChange={(v) => p.setProfile({ ...pr, trigram_coverage: Math.min(1, Math.max(0.5, v)) })} />
      </Sec>
    </>
  );
}

function SearchGroup(p: Props) {
  const s = p.search;
  const set = (patch: Partial<SearchSettings>) => p.setSearch({ ...s, ...patch });
  return (
    <>
      <Sec id="search-algo" title="Algorithm" note="how the search moves">
        <div className="row">
          <label title="Search algorithm. Its own settings appear below. Locked while a run is going.">Algorithm</label>
          <select value={s.algo} onChange={(e) => set({ algo: e.target.value })} disabled={p.running}>
            {ALGOS.filter(([a]) => p.meta.algos.includes(a)).map(([a, n]) => <option key={a} value={a}>{n}</option>)}
          </select>
          <span />
        </div>
        {s.algo === "sa" && (
          <Num label="Steps / restart" help="Swap attempts in one cooling schedule. 0 = auto (about 4 restarts per thread within the time budget)." value={s.saSteps} step={100000} min={0} unit={s.saSteps === 0 ? "auto" : ""} onChange={(v) => set({ saSteps: Math.max(0, Math.round(v)) })} />
        )}
        {s.algo === "ga" && (
          <Num label="Population" help="Layouts per generation." value={s.gaPop} step={16} min={4} onChange={(v) => set({ gaPop: Math.max(4, Math.round(v)) })} />
        )}
        {s.algo === "hc" && <div className="hint">Steepest descent with random restarts. No extra settings.</div>}
      </Sec>
      <Sec id="search" title="Budget" note="time and threads">
        <Num label="Time" help="Wall-clock budget for the run." value={s.timeS} min={1} step={1} unit="s" onChange={(v) => set({ timeS: Math.max(1, v) })} />
        <Num label="Threads" help="Independent workers (restarts / islands) in parallel." value={s.threads} min={1} max={p.meta.threads} unit={`/${p.meta.threads}`} onChange={(v) => set({ threads: Math.min(p.meta.threads, Math.max(1, Math.round(v))) })} />
        <Num label="Seed" help="Runs are reproducible from the seed (exactly so with an evaluation budget)." value={s.seed} min={0} onChange={(v) => set({ seed: Math.max(0, Math.round(v)) })} />
      </Sec>
    </>
  );
}

export function InputsPanel(p: Props) {
  return (
    <>
      <div className="panehead"><b>Inputs</b><span className="hint">hover a label for help</span></div>
      <div className="seg groups">
        {GROUPS.map(([g, label]) => <button key={g} className={p.group === g ? "on" : ""} onClick={() => p.setGroup(g)}>{label}</button>)}
      </div>
      <div className="scroll">
        {p.group === "layout" && <LayoutGroup {...p} />}
        {p.group === "board" && <BoardGroup {...p} />}
        {p.group === "corpus" && <CorpusGroup {...p} />}
        {p.group === "profile" && <ProfileGroup {...p} />}
        {p.group === "search" && <SearchGroup {...p} />}
      </div>
    </>
  );
}
