// Profile tab (frontend.md "Profile"): every scoring parameter in sections with scroll-spy |
// a visual of what the focused section optimizes | what it measures and how weights move the score.

import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { FINGERS, type Analysis, type Geometry, type LayoutInfo, type Meta, type MetricDef, type PairKind, type Profile } from "../../api";
import { fingerColor, heatColor, heatText, INK, PAIR_KINDS } from "../../colors";
import { fmt, LAYER_KEYS, ONESHOT_KEYS, TOGGLE_KEYS, undirected } from "../../format";
import { FitBoard, type Cell, type KeyArc } from "../Board2D";
import { FingerLoad } from "../Charts";
import { Frame, Num } from "../Form";

interface Props {
  meta: Meta;
  profiles: Profile[];
  profileId: string;
  setProfileId: (id: string) => void;
  profile: Profile;
  setProfile: (p: Profile) => void;
  geometry: Geometry;
  chars: string;
  layouts: LayoutInfo[];
  a: Analysis | null;
}

type Visual = "load" | "reach" | "sfb" | "stretch" | "rolls" | "redirects" | "comfort" | "layers" | "shortcuts" | "precision";
interface Section { id: Visual; title: string; ids: string[]; what: string; why: string; how: string }

const SECTIONS: Section[] = [
  { id: "load", title: "Finger load", ids: ["M02", "M03", "M07", "M04", "M09"],
    what: "How keystrokes divide over fingers and hands, against the target share per finger below.",
    why: "Overloaded weak fingers (pinkies) tire first; an even, strength-matched load is the base of comfort.",
    how: "Each weight scales its metric's spread over the reference layouts; raising Finger load error pulls loads toward the targets, Left hand toward 50/50." },
  { id: "reach", title: "Rows and reach", ids: ["M01", "M05", "M06", "B10", "B11"],
    what: "Where frequent keys sit: per-key effort, home-row use, the stretch columns, row jumps and total finger travel.",
    why: "The home row costs nothing to reach; every row change or centre-column stretch is motion.",
    how: "Higher Effort and Movement weights pull frequent symbols toward cheap keys; Home row is a reward and lowers the score as it rises." },
  { id: "sfb", title: "Same-finger", ids: ["B01", "B02", "B03", "S01", "S02", "S03", "T02", "T08"],
    what: "Consecutive (SFB) and one-apart (SFS) keystrokes on the same finger, and how far that finger travels.",
    why: "A finger cannot start the next key until it lifts from the last; same-finger pairs are the biggest single speed cost.",
    how: "SFB carries the largest default weight. Distance shapes how travel counts (linear, squared, Fitts); finger factors make pinky SFBs costlier." },
  { id: "stretch", title: "Stretches and scissors", ids: ["B04", "B05", "B06", "B09", "B14", "S04", "S05", "T09"],
    what: "Pairs that spread or cross adjacent fingers: lateral stretches (LSB), scissors (adjacent fingers on rows two apart), pinky–ring pairs.",
    why: "Adjacent fingers share tendons; forcing them apart vertically or sideways strains the hand.",
    how: "Stretch (B14) measures the geometry directly and replaces LSB and Scissors by default; weight it up for a compact feel." },
  { id: "rolls", title: "Rolls and alternation", ids: ["B07", "B08", "T01", "T03", "T04", "T05", "T12", "T13"],
    what: "Trigrams that roll across one hand (inward or outward) or alternate between hands.",
    why: "Rolls and alternation are the fast, fluid patterns; good layouts trade between them.",
    how: "These are rewards: a larger weight lowers the score more for each % of rolls or alternates." },
  { id: "redirects", title: "Redirects", ids: ["T06", "T07", "T10"],
    what: "One-hand trigrams that change direction (e.g. ring → index → middle); bad redirects avoid the index finger entirely.",
    why: "Changing direction mid-roll breaks rhythm and needs fine finger control.",
    how: "Raise Redirects to discourage direction changes; Bad redirects separately penalises the worst ones." },
  { id: "comfort", title: "Measured comfort", ids: ["E04", "E05"],
    what: "Key-pair discomfort from Engram's formula and its measured typing data.",
    why: "Measured comfort catches pairs the geometric metrics miss.",
    how: "A light weight nudges ties toward measured-comfortable pairs; it rarely changes the optimum on its own." },
  { id: "layers", title: "Layers", ids: ["Y01", "Y02", "Y03", "Y04"],
    what: "Layer switches, symbol bigrams across layers, paired brackets and home-row-mod misfires.",
    why: "On small boards symbols live on layers; each switch is an extra key press.",
    how: "Only layered layouts score here. Raise Layer switches to keep frequent symbols on the base." },
  { id: "shortcuts", title: "Shortcuts and familiarity", ids: ["L01", "L02"],
    what: "Whether Z X C V (and other shortcut letters) stay reachable by the left hand, and how far the layout moves from QWERTY.",
    why: "Shortcuts are muscle memory; a familiar layout is quicker to learn.",
    how: "Zero by default. A small weight trades a little typing score for an easier switch." },
  { id: "precision", title: "Precision", ids: [],
    what: "How many trigrams the scorer keeps: the most frequent ones covering this share of trigram volume.",
    why: "Rare trigrams barely change the score but cost search time.",
    how: "1 is exact. 0.95 is several times faster with a negligible score difference." },
];

const sectionOf = (id: string): Visual => SECTIONS.find((s) => s.ids.includes(id))?.id
  ?? ({ M: "reach", B: "stretch", S: "sfb", T: "rolls", E: "comfort", Y: "layers", L: "shortcuts" } as Record<string, Visual>)[id[0]] ?? "precision";

function Visualize({ id, geometry, chars, a, layouts }: { id: Visual; geometry: Geometry; chars: string; a: Analysis | null; layouts: LayoutInfo[] }) {
  const sym = [...chars];
  const n = geometry.key.length;
  const slotOf = new Map(sym.slice(0, n).map((c, i) => [c, i]));
  const label = (i: number) => (sym[i] ?? "").toUpperCase();
  const plain = (i: number): Cell => ({ fill: "#ffffff", text: INK, main: label(i) });
  const arcsOf = (pairs: { ngram: string; freq: number; kind: PairKind }[], kinds: PairKind[], top = 30): KeyArc[] => {
    const shown = undirected(pairs).filter((p) => kinds.includes(p.kind)).slice(0, top);
    const max = Math.max(...shown.map((p) => p.freq), 1e-9);
    return shown.flatMap((p) => {
      const [i, j] = [slotOf.get(p.ngram[0]), slotOf.get(p.ngram[1])];
      return i === undefined || j === undefined ? [] : [{ i, j, color: PAIR_KINDS[p.kind][1], w: Math.sqrt(p.freq / max) }];
    });
  };
  const legend = (items: [string, string][]) => <div className="legend static">{items.map(([n, c]) => <span key={n}><i style={{ background: c }} />{n}</span>)}</div>;
  const board = (cells: (i: number) => Cell, arcs?: KeyArc[], caption?: ReactNode) => (
    <div className="visual">
      <FitBoard geometry={geometry} cells={cells} arcs={arcs} max={60} pad={16} />
      {caption}
    </div>
  );
  if (!a && id !== "precision") return <div className="empty">Scoring…</div>;
  const maxHeat = Math.max(...(a?.heat ?? [1]), 1e-9);
  switch (id) {
    case "load":
      return (
        <div className="visual split">
          <FitBoard geometry={geometry} max={56} pad={16} cells={(i) => {
            const t = (a!.heat[i] ?? 0) / maxHeat;
            return { fill: mix(fingerColor(geometry.key[i].finger), t), text: INK, main: label(i), sub: fmt(a!.heat[i] ?? 0, 1) };
          }} />
          <div className="visual-chart"><FingerLoad a={a!} refName="" /></div>
        </div>
      );
    case "reach": {
      const es = geometry.key.map((k) => k.effort), lo = Math.min(...es), hi = Math.max(...es);
      return board((i) => {
        const t = (es[i] - lo) / Math.max(hi - lo, 1e-9);
        return { fill: heatColor(t * 0.85), text: heatText(t * 0.85), main: label(i), sub: es[i].toFixed(1), ring: geometry.key[i].home };
      }, undefined, <div className="legend static"><span>darker = more effort · ringed = home keys</span></div>);
    }
    case "sfb":
      return board((i) => ({ ...plain(i), fill: mix(fingerColor(geometry.key[i].finger), 0.35) }), [...arcsOf(a!.bigrams, ["sfb"]), ...arcsOf(a!.skipgrams, ["sfs"], 15).map((x) => ({ ...x, w: x.w * 0.5 }))],
        legend([["SFB (strong) and SFS (faint) on one finger", PAIR_KINDS.sfb[1]]]));
    case "stretch":
      return board((i) => ({ ...plain(i), fill: mix(fingerColor(geometry.key[i].finger), 0.35) }), arcsOf(a!.bigrams, ["lsb", "scissor"]),
        legend([["Lateral stretch", PAIR_KINDS.lsb[1]], ["Scissors", PAIR_KINDS.scissor[1]]]));
    case "rolls":
      return board((i) => ({ ...plain(i), fill: mix(fingerColor(geometry.key[i].finger), 0.35) }), arcsOf(a!.bigrams, ["same_hand"], 24),
        legend([["Same-hand pairs (rolls)", INK], ["alternation: the other hand next", "#adadad"]]));
    case "redirects": {
      // Schematic on the board: ring → index → middle on the left home row.
      const home = (f: string) => geometry.key.findIndex((k) => k.finger === f && k.home);
      const [r, ix, m] = [home("LR"), home("LI"), home("LM")];
      const arcs: KeyArc[] = r >= 0 && ix >= 0 && m >= 0 ? [{ i: r, j: ix, color: INK, w: 1 }, { i: ix, j: m, color: PAIR_KINDS.sfb[1], w: 1 }] : [];
      return board((i) => ({ ...plain(i), ring: i === r || i === ix || i === m, mark: i === r ? "1" : i === ix ? "2" : i === m ? "3" : undefined }), arcs,
        <div className="legend static"><span>example: {label(r)} → {label(ix)} → {label(m)} rolls inward, then turns back</span></div>);
    }
    case "comfort":
      return board((i) => {
        const t = (a!.heat[i] ?? 0) / maxHeat;
        return { fill: heatColor(t), text: heatText(t), main: label(i) };
      }, arcsOf(a!.bigrams, ["sfb", "lsb", "scissor"], 12), <div className="legend static"><span>heat = keystrokes · arcs = the least comfortable frequent pairs</span></div>);
    case "layers": {
      const glyphs = [...LAYER_KEYS, ...ONESHOT_KEYS, ...TOGGLE_KEYS];
      const has = sym.slice(0, n).some((c) => glyphs.includes(c));
      return board((i) => ({ ...plain(i), ring: glyphs.includes(sym[i]) }), undefined,
        <div className="legend static"><span>{has ? "ringed = layer keys" : "This layout has no layers, so layer metrics are 0."}</span></div>);
    }
    case "shortcuts": {
      const qwerty = layouts.find((l) => l.id === "qwerty")?.chars ?? "";
      const q = [...qwerty];
      return board((i) => ({
        fill: q.length && q[i] !== sym[i] ? "#ececec" : "#ffffff", text: INK, main: label(i), ring: "zxcv".includes(sym[i] ?? "_"),
      }), undefined, <div className="legend static"><span>ringed = Z X C V · shaded = moved from QWERTY</span></div>);
    }
    case "precision":
      return <div className="empty">Precision changes speed, not what a good layout is.</div>;
  }
}

/** Finger color blended toward white by (1 − t). */
function mix(hex: string, t: number): string {
  const c = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16));
  const k = 0.25 + 0.75 * Math.max(0, Math.min(1, t));
  return `rgb(${c.map((v) => Math.round(255 - (255 - v) * k)).join(",")})`;
}

export function ProfileView(p: Props) {
  const pr = p.profile;
  const [focus, setFocus] = useState<Visual>("load");
  const scroller = useRef<HTMLDivElement>(null);
  const base = p.profiles.find((x) => x.id === p.profileId);
  const changed = base && JSON.stringify({ ...base, id: undefined }) !== JSON.stringify({ ...pr, id: undefined });
  const bySection = useMemo(() => {
    const m = new Map<Visual, MetricDef[]>();
    for (const d of p.meta.metrics) {
      if (d.dir === "info") continue;
      const s = sectionOf(d.id);
      m.set(s, [...(m.get(s) ?? []), d]);
    }
    return m;
  }, [p.meta.metrics]);
  const setW = (id: string, v: number) => p.setProfile({ ...pr, weights: { ...pr.weights, [id]: Math.max(0, v) } });

  // Scroll-spy: the last section whose top has passed the top of the sidebar.
  const spy = () => {
    const el = scroller.current;
    if (!el) return;
    const top = el.getBoundingClientRect().top;
    let cur: Visual = SECTIONS[0].id;
    for (const s of SECTIONS) {
      const h = el.querySelector(`[data-sec="${s.id}"]`);
      if (h && h.getBoundingClientRect().top - top <= 32) cur = s.id;
    }
    if (el.scrollTop + el.clientHeight >= el.scrollHeight - 2) cur = SECTIONS[SECTIONS.length - 1].id;
    setFocus(cur);
  };
  useEffect(spy, []);
  const jump = (id: Visual) => {
    const el = scroller.current?.querySelector(`[data-sec="${id}"]`) as HTMLElement | null;
    if (el && scroller.current) scroller.current.scrollTo({ top: el.offsetTop - 4, behavior: "smooth" });
    setFocus(id);
  };
  const sec = SECTIONS.find((s) => s.id === focus)!;
  const metrics = p.a?.breakdown.metrics.filter((m) => sectionOf(m.id) === focus && m.dir !== "info") ?? [];

  return (
    <Frame
      title="Profile"
      control={<span className="hint">{pr.name}{changed ? " (edited)" : ""}</span>}
      sideRef={scroller}
      onSideScroll={spy}
      side={
        <>
          <div className="pad">
            <div className="row">
              <label title="Metric weights and targets (data/profiles). Edits re-score live and are sent with each run.">Profile</label>
              <select value={p.profileId} onChange={(e) => p.setProfileId(e.target.value)}>
                {p.profiles.map((x) => <option key={x.id} value={x.id}>{x.name}</option>)}
              </select>
              <span />
            </div>
            <div className="hint">{pr.description}</div>
            <div className="btns"><button className="btn soft small" disabled={!changed} onClick={() => base && p.setProfile(base)}>Reset to {base?.name ?? "saved"}</button></div>
          </div>
          {SECTIONS.map((s) => (
            <div key={s.id} className={`psec ${focus === s.id ? "focus" : ""}`} data-sec={s.id}>
              <button className="psec-head" onClick={() => jump(s.id)} title={s.what}>{s.title}</button>
              <div className="psec-body">
                {(bySection.get(s.id) ?? []).map((m) => (
                  <Num key={m.id} label={`${m.name}${m.dir === "max" ? " +" : ""}`} help={`${m.id} · ${m.desc}${m.dir === "max" ? " (reward: a larger weight lowers the score)" : ""}`}
                    value={pr.weights[m.id] ?? 0} step={0.05} min={0} onChange={(v) => setW(m.id, v)} unit={m.id} />
                ))}
                {s.id === "load" && <>
                  {pr.finger_target.map((t, i) => (i === 4 || i === 5 ? null : (
                    <Num key={i} label={`Target ${FINGERS[i]}`} help={`Target share of keystrokes for ${FINGERS[i]} (Finger load error, M02).`} value={t} step={0.5} unit="%"
                      onChange={(v) => { const ft = [...pr.finger_target]; ft[i] = v; p.setProfile({ ...pr, finger_target: ft }); }} />
                  )))}
                  <Num label="Thumb target" help="Target thumb share % for Thumb load (M09)." value={pr.thumb_target ?? 18} step={0.5} unit="%" onChange={(v) => p.setProfile({ ...pr, thumb_target: v })} />
                </>}
                {s.id === "sfb" && pr.sfb && <>
                  <div className="row">
                    <label title="How same-finger travel distance counts toward Finger speed.">Distance</label>
                    <select value={pr.sfb.distance} onChange={(e) => p.setProfile({ ...pr, sfb: { ...pr.sfb!, distance: e.target.value as "linear" | "squared" | "fitts" } })}>
                      <option value="linear">Linear</option><option value="squared">Squared</option><option value="fitts">Fitts</option>
                    </select><span />
                  </div>
                  <Num label="Lateral factor" help="Multiplier on sideways same-finger travel." value={pr.sfb.lateral} step={0.1} onChange={(v) => p.setProfile({ ...pr, sfb: { ...pr.sfb!, lateral: v } })} />
                  <Num label="Skip decay" help="Weight of an SFS relative to an SFB for each key between." value={pr.sfb.skip_decay} step={0.05} onChange={(v) => p.setProfile({ ...pr, sfb: { ...pr.sfb!, skip_decay: v } })} />
                </>}
                {s.id === "precision" && (
                  <Num label="Trigram coverage" help="Keep the most frequent trigrams covering this share of trigram volume. 1 = exact, lower = faster search." value={pr.trigram_coverage} step={0.005} min={0.5} max={1}
                    onChange={(v) => p.setProfile({ ...pr, trigram_coverage: Math.min(1, Math.max(0.5, v)) })} />
                )}
              </div>
            </div>
          ))}
        </>
      }
      main={
        <div className="viewport light">
          <Visualize id={focus} geometry={p.geometry} chars={p.chars} a={p.a} layouts={p.layouts} />
          <div className="viewlabel"><span>{sec.title}</span><span className="muted">on the current layout</span></div>
        </div>
      }
      bottom={
        <div className="bottompad describe">
          <div>
            <h4>{sec.title}</h4>
            <p><b>Measures.</b> {sec.what}</p>
            <p><b>Why it matters.</b> {sec.why}</p>
            <p><b>Weights.</b> {sec.how}</p>
          </div>
          {metrics.length > 0 && (
            <table className="tbl">
              <thead><tr><th>metric</th><th>value</th><th>weight</th><th>contribution</th></tr></thead>
              <tbody>
                {metrics.map((m) => (
                  <tr key={m.id} className={m.weight === 0 ? "off" : ""}><td title={m.id}>{m.name}</td><td>{fmt(m.value)}{m.unit === "%" ? "%" : ""}</td><td>{fmt(pr.weights[m.id] ?? 0, 3)}</td><td>{m.weight === 0 ? "–" : fmt(m.weighted, 3)}</td></tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      }
    />
  );
}
