// Profile tab (web-screens.md "Profile"): every scoring parameter in sections with scroll-spy |
// a visual of what the focused section optimizes | what it measures and how weights move the score.

import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { type Analysis, type Geometry, type LayoutInfo, type Meta, type MetricDef, type PairKind, type Profile } from "../../api";
import { fingerColor, heatColor, heatText, INK, PAIR_KINDS } from "../../colors";
import { fmt, LAYER_KEYS, ONESHOT_KEYS, TOGGLE_KEYS, undirected } from "../../format";
import { FitBoard, type Cell, type KeyArc } from "../Board2D";
import { FingerLoad } from "../Charts";
import { defaultFingerEffort, editFingerEffort, fingerTargets, symmetricEffort, withFingerEffort } from "../../fingerEffort";
import { Frame, Num, Seg, Toggle } from "../Form";
import { withHand, type HandSettings, type HandSpec } from "../../hand";
import { useHandCosts } from "../../handCosts";

interface Props {
  meta: Meta;
  profiles: Profile[];
  profileId: string;
  setProfileId: (id: string) => void;
  profile: Profile;
  setProfile: (p: Profile) => void;
  /** The Hand tab's settings: merged into the saved profile for the "edited" check and reset. */
  hand: HandSettings;
  geometry: Geometry;
  chars: string;
  layouts: LayoutInfo[];
  a: Analysis | null;
}

type Visual = "load" | "reach" | "hand" | "sfb" | "stretch" | "rolls" | "redirects" | "comfort" | "layers" | "shortcuts" | "precision";
interface Section { id: Visual; title: string; ids: string[]; what: string; why: string; how: string }

const SECTIONS: Section[] = [
  { id: "load", title: "Finger load", ids: ["finger_target", "finger_evenness", "finger_peak", "hand_balance", "thumb_load"],
    what: "How keystrokes divide over fingers and hands, against the target share per finger below.",
    why: "Overloaded weak fingers (pinkies) tire first; an even, strength-matched load is the base of comfort.",
    how: "Each weight scales its metric's spread over the reference layouts; raising Finger load error pulls loads toward the targets, Left hand toward 50/50." },
  { id: "reach", title: "Rows and reach", ids: ["effort", "home_row", "center_cols", "row_jump", "movement"],
    what: "Where frequent keys sit: per-key effort, home-row use, the stretch columns, row jumps and total finger travel.",
    why: "The home row costs nothing to reach; every row change or centre-column stretch is motion.",
    how: "Higher Effort and Movement weights pull frequent symbols toward cheap keys; Home row is a reward and lowers the score as it rises." },
  { id: "hand", title: "Hand model", ids: ["reach", "press_angle", "wrist", "thumb_arc"],
    what: "Reach, press angle, wrist posture and thumb arc fit from the hand model: each key's cost for this hand, with the hand resting on the home keys (set the hand and arms on the Hand tab).",
    why: "Keys the fingertips reach with little joint deviation, and press head-on, are faster and kinder; straight, less-pronated wrists strain less. The costs follow the board's shape and the hand's size.",
    how: "Reach pulls frequent symbols onto keys this hand reaches easily; Press angle, Wrist posture and Thumb arc fit mostly separate boards (keywells, splits, tents and thumb keys along the thumb's sweep win). Turning on Use hand in optimization adds all four at the hand profile's weights." },
  { id: "sfb", title: "Same-finger", ids: ["sfb", "repeat", "sfs", "sfs_far", "alt_sfs", "sft"],
    what: "Consecutive (SFB) and one-apart (SFS) keystrokes on the same finger.",
    why: "A finger cannot start the next key until it lifts from the last; same-finger pairs are the biggest single speed cost.",
    how: "SFB carries the largest default weight: in keystroke timing an SFB costs about half a keystroke more, and how far the finger travels adds nothing measurable. SFS has no measured timing cost; its weight is a comfort assumption. Skip decay sets how much SFS two or three keys apart count." },
  { id: "stretch", title: "Stretches and scissors", ids: ["lsb", "scissor", "half_scissor", "pinky_ring", "stretch", "skip_stretch", "contortion"],
    what: "Pairs that spread or cross adjacent fingers: lateral stretches (LSB), scissors (adjacent fingers on rows two apart), pinky–ring pairs.",
    why: "Adjacent fingers share tendons; forcing them apart vertically or sideways strains the hand.",
    how: "Stretch measures the geometry directly and replaces LSB and Scissors by default; weight it up for a compact feel." },
  { id: "rolls", title: "Rolls and alternation", ids: ["bigram_inroll", "hand_switch", "alternate", "inroll", "outroll", "onehand", "thumb_roll", "thumb_alt"],
    what: "Trigrams that roll across one hand (inward or outward) or alternate between hands.",
    why: "Rolls and alternation are the fast, fluid patterns; good layouts trade between them.",
    how: "These are rewards: a larger weight lowers the score more for each % of rolls or alternates." },
  { id: "redirects", title: "Redirects", ids: ["redirect", "bad_redirect", "irregular"],
    what: "One-hand trigrams that change direction (e.g. ring → index → middle); bad redirects avoid the index finger entirely.",
    why: "Changing direction mid-roll breaks rhythm and needs fine finger control.",
    how: "Raise Redirects to discourage direction changes; Bad redirects separately penalises the worst ones." },
  { id: "comfort", title: "Measured comfort and timing", ids: ["typing_time", "engram_formula", "engram"],
    what: "Key-pair discomfort from Engram's formula and its measured typing data, and typing time predicted from 136M keystrokes of fast touch typists.",
    why: "Measured comfort and timing catch pairs the geometric metrics miss, and check that the other weights point the right way.",
    how: "A light weight nudges ties toward measured-comfortable pairs. Typing time is a judge, unweighted by default: weighting it makes the score chase the model that checks it." },
  { id: "layers", title: "Layers", ids: ["layer_switch", "symbol_bigram", "brackets", "hrm_misfire"],
    what: "Layer switches, symbol bigrams across layers, paired brackets and home-row-mod misfires.",
    why: "On small boards symbols live on layers; each switch is an extra key press.",
    how: "Only layered layouts score here. Raise Layer switches to keep frequent symbols on the base." },
  { id: "shortcuts", title: "Shortcuts and familiarity", ids: ["shortcuts", "familiar", "vim"],
    what: "Whether Z X C V (and other shortcut letters) stay reachable by the left hand, how far the layout moves from QWERTY, and whether Vim's h j k l keep a usable shape.",
    why: "Shortcuts are muscle memory; a familiar layout is quicker to learn.",
    how: "Zero by default. A small weight trades a little typing score for an easier switch." },
  { id: "precision", title: "Precision", ids: [],
    what: "How many trigrams the scorer keeps: the most frequent ones covering this share of trigram volume.",
    why: "Rare trigrams barely change the score but cost search time.",
    how: "1 is exact. 0.95 is several times faster with a negligible score difference." },
];

const sectionOf = (id: string): Visual => SECTIONS.find((s) => s.ids.includes(id))?.id
  ?? ({ M: "reach", G: "hand", B: "stretch", S: "sfb", T: "rolls", E: "comfort", Y: "layers", L: "shortcuts" } as Record<string, Visual>)[id[0]] ?? "precision";

function Visualize({ id, geometry, chars, a, layouts, hand }: { id: Visual; geometry: Geometry; chars: string; a: Analysis | null; layouts: LayoutInfo[]; hand: HandSpec }) {
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
  if (!a && id !== "precision" && id !== "hand") return <div className="empty">Scoring…</div>;
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
      }, undefined, <div className="legend static"><span>darker = geometry base effort (before profile costs) · ringed = home keys</span></div>);
    }
    case "hand":
      return <HandVisual geometry={geometry} hand={hand} label={label} />;
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

/** `reach` cost per key for the profile's hand (average when the profile has none). */
function HandVisual({ geometry, hand, label }: { geometry: Geometry; hand: HandSpec; label: (i: number) => string }) {
  const c = useHandCosts(geometry, hand);
  if (!c) return <div className="empty">Solving the hand…</div>;
  const hi = Math.max(...c.reach, 1e-9);
  return (
    <div className="visual">
      <FitBoard geometry={geometry} max={60} pad={16} cells={(i) => {
        const t = Math.min(1, c.reach[i] / Math.min(hi, 4)) * 0.85;
        return { fill: heatColor(t), text: heatText(t), main: label(i), sub: c.reach[i].toFixed(1), ring: c.miss[i] > 0 };
      }} />
      <div className="legend static"><span>darker = higher reach cost · ringed = the fingertip falls short with the hand on the home keys</span></div>
    </div>
  );
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
  const saved = p.profiles.find((x) => x.id === p.profileId);
  const base = saved ? withHand(withFingerEffort(saved), p.hand) : undefined;
  const effort = pr.finger_effort ?? defaultFingerEffort();
  const targets = fingerTargets(effort, p.geometry);
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
  const shown = new Set(p.meta.metrics.map((d) => d.id));
  const metrics = p.a?.breakdown.metrics.filter((m) => sectionOf(m.id) === focus && m.dir !== "info" && shown.has(m.id)) ?? [];

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
                    value={pr.weights[m.id] ?? 0} step={0.05} min={0} onChange={(v) => setW(m.id, v)} />
                ))}
                {s.id === "load" && <>
                  <Toggle label="Symmetric hands" help="Use the left-hand costs for both hands. Turning this on copies the left values to the right." value={effort.symmetric}
                    onChange={v => p.setProfile({ ...pr, finger_effort: symmetricEffort(effort, v) })} />
                  <p className="hint">Higher effort = harder. Targets use inverse typing effort, normalized over fingers on this board. Movement costs add to Effort per column or row from home.</p>
                  {(effort.symmetric ? [3, 2, 1, 0, 4] : [3, 2, 1, 0, 4, 6, 7, 8, 9, 5]).map(i => {
                    const name = ["Pinky", "Ring", "Middle", "Index", "Thumb"][i < 5 ? i : 9 - i];
                    const label = effort.symmetric ? name : `${i < 5 ? "Left" : "Right"} ${name.toLowerCase()}`;
                    return <div key={i} className="finger-effort">
                      <div className="hint">{label} · target {targets[i].toFixed(1)}%{effort.symmetric ? ` left / ${targets[9 - i].toFixed(1)}% right` : ""}</div>
                      {([['typing', 'Typing effort'], ['lateral', 'Sideways effort'], ['vertical', 'Vertical effort']] as const).map(([field, title]) => (
                        <Num key={field} label={title} help={field === 'typing' ? "Relative typing effort; twice the effort gives half the target share." : "Extra key effort per logical column or row away from this finger’s nearest home key. No home key means no added movement cost."}
                          value={effort[field][i]} step={0.1} min={field === 'typing' ? 0.01 : 0}
                          onChange={v => p.setProfile({ ...pr, finger_effort: editFingerEffort(effort, field, i, v) })} />
                      ))}
                    </div>;
                  })}
                  <div className="hint">Combined thumb target: {(targets[4] + targets[5]).toFixed(1)}%</div>
                </>}
                {s.id === "sfb" && pr.sfb && <>
                  <Num label="Skip decay" help="Weight of an SFS relative to an SFB for each key between." value={pr.sfb.skip_decay} step={0.05} onChange={(v) => p.setProfile({ ...pr, sfb: { ...pr.sfb!, skip_decay: v } })} />
                </>}
                {s.id === "shortcuts" && <>
                  <div className="row">
                    <label title="Shape Vim motions must keep. Off: the vim metric scores 0. Row: h j k l in order on neighbouring keys of one row. Pairs: h left of l and j left of k, each on neighbours. Fingers: four different fingers of one hand.">Vim h j k l</label>
                    <Seg value={(pr.vim ?? "off") as "off" | "row" | "pairs" | "fingers"} options={[["off", "Off"], ["row", "Row"], ["pairs", "Pairs"], ["fingers", "Fingers"]]}
                      onChange={(v) => p.setProfile({ ...pr, vim: v === "off" ? undefined : v })} />
                    <span />
                  </div>
                  {pr.vim && !pr.weights.vim && <p className="hint">Give Vim motions a weight for the policy to count; 100 or more makes it a constraint.</p>}
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
          <Visualize id={focus} geometry={p.geometry} chars={p.chars} a={p.a} layouts={p.layouts} hand={pr.hand ?? { size: "average" }} />
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
