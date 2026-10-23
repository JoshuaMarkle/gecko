// Hand tab (web-screens.md "Hand"): hand settings | 3D hands placed over the board's home keys
// (mirrored skeletons, finger IK), with a typing simulation whose fingers move in parallel |
// reach (the scorer's per-key hand costs), measurements, typing timeline and stats, size
// distribution, similarity. "Use hand in optimization" sends this hand to the scorer (G01, G03, G05,
// B14 spread); the wrist and typing simulation stay view-only.

import { useMemo, useRef, useState } from "react";
import type { Geometry } from "../../api";
import {
  dimsVector, FINGER_NAMES, handDims, handLength, planTyping, wristOf, WRIST0, bodyOf, handSpec, sizeBand,
  type FingerName, type HandDims, type HandSettings, type Preset, type Side, type Wrist,
} from "../../hand";
import { Keyboard3D, type ViewMode } from "../../three/Keyboard3D";
import { Frame, Num, PanelTabs, Sec, Seg, Slider, Toggle } from "../Form";
import { cap } from "../../format";
import { useHandCosts } from "../../handCosts";
import { Hands, handFocus } from "../hand/Hands3D";
import { Measurements, Reach, Similarity, SizeDistribution, Typing } from "../hand/Panels";
import { buildRigs, framesAt, MM, reachFor, strokesFor } from "../hand/rigs";


interface Props {
  geometry: Geometry;
  chars: string;
  hand: HandSettings;
  setHand: (h: HandSettings) => void;
}

const SAMPLES: [string, string][] = [
  ["pangram", "the quick brown fox jumps over the lazy dog"],
  ["prose", "it was the best of times, it was the worst of times, it was the age of wisdom"],
  ["code", "for (let i = 0; i < n; i++) { total += values[i]; }"],
];
const SEG_NAMES: Record<FingerName, [string, string, string]> = {
  thumb: ["Metacarpal", "Proximal", "Distal"], index: ["Proximal", "Middle", "Distal"], middle: ["Proximal", "Middle", "Distal"],
  ring: ["Proximal", "Middle", "Distal"], pinky: ["Proximal", "Middle", "Distal"],
};

// ---------- view ----------

export function HandView(p: Props) {
  const h = p.hand;
  const d = handDims(h);
  const off = !h.use;
  const [tab, setTab] = useState<"reach" | "measure" | "typing" | "size" | "similar">("reach");
  const [reachHeat, setReachHeat] = useState(true);
  const costs = useHandCosts(p.geometry, handSpec(h));
  const [sideTab, setSideTab] = useState<"hand" | "simulation">("hand");
  const [sample, setSample] = useState("pangram");
  const [wpm, setWpm] = useState(120);
  const [parallel, setParallel] = useState(1);
  const [playing, setPlaying] = useState(false);
  const [loop, setLoop] = useState(true);
  const [labels, setLabels] = useState(false);
  const [skin, setSkin] = useState(true);
  const [mode, setMode] = useState<ViewMode>("3d");
  /** Key under the pointer: its finger reaches for it (-1: hands at rest). */
  const [hover, setHover] = useState(-1);
  const clock = useRef(-1);
  /** Press state per board key, written by the typing animation, read by the keycaps. */
  const keys = useRef(new Float32Array(0));
  if (keys.current.length !== p.geometry.key.length) keys.current = new Float32Array(p.geometry.key.length);
  const setDims = (next: HandDims) => p.setHand({ ...h, preset: "custom", custom: next });
  const setSeg = (f: FingerName, i: number, v: number) => { const s = [...d[f]] as [number, number, number]; s[i] = Math.max(1, v); setDims({ ...d, [f]: s }); };

  // `d` is rebuilt every render; key the memos on its values.
  const dimsKey = dimsVector(d).join(",");
  const w = wristOf(h);
  const arms = bodyOf(h);
  const wristKey = Object.values(w).join(",");
  const setWrist = (patch: Partial<Wrist>) => p.setHand({ ...h, wrist: { ...w, ...patch } });
  const { rigs, keyTop, keyNormal } = useMemo(() => buildRigs(p.geometry, d, w), [p.geometry, dimsKey, wristKey]);
  const reach = useMemo(() => (hover >= 0 ? reachFor(p.geometry, rigs, keyTop, hover) : null), [hover, p.geometry, rigs, keyTop]);
  const focus = useMemo(() => handFocus(p.geometry, rigs, d, mode), [p.geometry, rigs, mode]);
  // Fingers that cannot reach their home key in the resting pose (wrist adjusted too far), with the gap in mm.
  const unreachable = useMemo(() => framesAt(rigs, null, d, -1).flatMap(({ pose }, hi) => FINGER_NAMES.map((f, fi) => {
    const tip = pose.fingers[fi][3], t = rigs[hi].home[f];
    return { name: `${rigs[hi].side} ${f}`, gap: Math.hypot(tip[0] - t[0], tip[1] - t[1], tip[2] - t[2]) / MM };
  })).filter((x) => x.gap > 1), [rigs, dimsKey]);
  const text = SAMPLES.find(([k]) => k === sample)?.[1] ?? "";
  const plan = useMemo(() => {
    if (!rigs.length) return null;
    const strokes = strokesFor(text, p.geometry, p.chars, rigs, keyTop, keyNormal);
    const homeOf = (s: Side, f: FingerName) => rigs.find((r) => r.side === s)!.home[f];
    return planTyping(strokes, homeOf, wpm, parallel, MM);
  }, [rigs, keyTop, keyNormal, text, p.geometry, p.chars, wpm, parallel]);

  return (
    <Frame
      title={<Seg inline value={sideTab} onChange={setSideTab} options={[["hand", "Hand"], ["simulation", "Simulation"]]} />}
      side={
        sideTab === "hand" ? <>
          <div className="pad">
            <Toggle label="Use in optimization" help="Score and optimize with this hand: its reach (G01), press angle (G05) and wrist posture (G03) per key, and B14's stretch allowance scaled by its span. Adds G01 / G03 / G05 to the profile at the hand profile's weights unless the profile sets them." value={h.use} onChange={(use) => p.setHand({ ...h, use })} />
            {off ? <div className="hint">Off: the scorer uses the average hand for G01 / G03 / G05, at the profile's weights (0 in the default). The views on the right still preview this hand.</div>
              : <div className="hint">On: runs and scores use this hand. Reach, Press angle and Wrist posture are in the Profile tab's Hand model section.</div>}
          </div>
          <fieldset className="plain" disabled={off}>
            <div className="pad">
              <div className="row">
                <label title="Preset hand sizes. Editing a value below switches to Custom.">Preset</label>
                <Seg value={h.preset} disabled={off} onChange={(preset: Preset) => p.setHand({ ...h, preset })}
                  options={[["average", "Average"], ["large", "Large"], ["small", "Small"], ...(h.custom ? [["custom", "Custom"] as [Preset, string]] : [])]} />
                <span />
              </div>
              <div className="kv"><span>Hand length</span><span>{handLength(d).toFixed(0)} mm · {sizeBand(handLength(d))}</span></div>
            </div>
            <Sec id="hand-adv" title="Advanced" note="custom values, mm" defaultOpen={false}>
              {FINGER_NAMES.map((f) => (
                <div key={f} className="subgroup">
                  <div className="subhead">{cap(f)}</div>
                  {d[f].map((v, i) => (
                    <Num key={i} label={SEG_NAMES[f][i]} help={`${cap(f)} ${SEG_NAMES[f][i].toLowerCase()} segment length.`} value={+v.toFixed(1)} step={0.5} unit="mm" disabled={off} onChange={(x) => setSeg(f, i, x)} />
                  ))}
                </div>
              ))}
              <div className="subgroup">
                <div className="subhead">Palm and reach</div>
                <Num label="Palm width" help="Across the index–pinky knuckles." value={+d.palmWidth.toFixed(1)} step={0.5} unit="mm" disabled={off} onChange={(x) => setDims({ ...d, palmWidth: Math.max(30, x) })} />
                <Num label="Palm length" help="Wrist crease to the middle knuckle." value={+d.palmLength.toFixed(1)} step={0.5} unit="mm" disabled={off} onChange={(x) => setDims({ ...d, palmLength: Math.max(40, x) })} />
                <Num label="Span" help="Thumb tip to pinky tip, spread flat." value={+d.span.toFixed(1)} step={1} unit="mm" disabled={off} onChange={(x) => setDims({ ...d, span: Math.max(80, x) })} />
                <Num label="Reach" help="Comfortable fingertip travel from the home key." value={+d.reach.toFixed(1)} step={1} unit="mm" disabled={off} onChange={(x) => setDims({ ...d, reach: Math.max(5, x) })} />
              </div>
              <div className="subgroup">
                <div className="subhead">Arms (wrist posture, G03)</div>
                <Num label="Elbow width" help="Elbow to elbow while typing. Wider elbows angle the forearms inward and bend the wrists more toward the pinky on a one-piece board." value={+arms.elbowWidth.toFixed(0)} step={5} unit="mm" disabled={off} onChange={(x) => p.setHand({ ...h, body: { ...arms, elbowWidth: Math.min(1500, Math.max(100, x)) } })} />
                <Num label="Forearm" help="Elbow to wrist crease." value={+arms.forearm.toFixed(0)} step={5} unit="mm" disabled={off} onChange={(x) => p.setHand({ ...h, body: { ...arms, forearm: Math.min(600, Math.max(100, x)) } })} />
              </div>
              <div className="btns">
                <button className="btn soft small" disabled={off || h.preset !== "custom"} onClick={() => p.setHand({ ...h, preset: "average", custom: null })}>Reset to Average</button>
                <button className="btn soft small" disabled={off || !h.body} onClick={() => p.setHand({ ...h, body: undefined })}>Arms from hand size</button>
              </div>
            </Sec>
          </fieldset>
        </> : (
          <Sec id="hand-wrist" title="Wrist" note="view only">
            <div className="wrist">
              <Slider label="Height" help="Raise (+) or lower (−) the wrist and palm from where the fingers rest naturally; the fingers still reach the home keys." value={w.height} min={-30} max={40} step={1} unit={`${w.height} mm`} onChange={(height) => setWrist({ height })} />
              <Slider label="Distance" help="Move the wrists toward you (+) or toward the keyboard (−)." value={w.distance} min={-40} max={60} step={1} unit={`${w.distance} mm`} onChange={(distance) => setWrist({ distance })} />
              <Slider label="Spread" help="Move each hand away from (+) or toward (−) the middle of the board." value={w.spread} min={-40} max={60} step={1} unit={`${w.spread} mm`} onChange={(spread) => setWrist({ spread })} />
              <Slider label="Angle" help="Turn each hand about the wrist: + points the fingers more toward the middle of the board, − straighter ahead or outward." value={w.angle} min={-30} max={30} step={1} unit={`${w.angle}°`} onChange={(angle) => setWrist({ angle })} />
              <Slider label="Tilt" help="Roll each hand about the forearm: + lifts the thumb side so the pinky side sits closer to the keyboard (the index knuckle rises toward the middle); − the reverse." value={w.tilt} min={-30} max={45} step={1} unit={`${w.tilt}°`} onChange={(tilt) => setWrist({ tilt })} />
              <Slider label="Wrist movement" help="How much the whole hand follows a reaching finger while typing: 0 keeps the wrist planted (fingers stretch), 100% moves the hand with the finger." value={w.follow} min={0} max={1} step={0.05} unit={`${(100 * w.follow).toFixed(0)}%`} onChange={(follow) => setWrist({ follow })} />
              <div className="btns"><button className="btn soft small" disabled={Object.entries(WRIST0).every(([k, v]) => w[k as keyof Wrist] === v)} onClick={() => p.setHand({ ...h, wrist: WRIST0 })}>Reset wrist</button></div>
            </div>
          </Sec>
        )
      }
      main={
        <div className="viewport hand-viewport">
          <Keyboard3D geometry={p.geometry} chars={p.chars} heat={reachHeat && costs ? costs.reach.map((r) => Math.min(r, 4)) : []} arcs={[]} overlay={reachHeat && costs ? "heat" : "none"} mode={mode} selected={null} onKey={() => {}} onHover={(i, on) => setHover((h) => (on ? i : h === i ? -1 : h))} focus={focus} pressed={keys}>
            <Hands rigs={rigs} dims={d} plan={plan} clock={clock} keys={keys} playing={playing} loop={loop} onEnd={() => setPlaying(false)} labels={labels} skin={skin} reach={reach} />
          </Keyboard3D>
          <div className="viewlabel">
            <span>Hands on {p.geometry.name}</span>
            {unreachable.length > 0 && <span className="err" title="These fingertips cannot reach their home key from this wrist position; they point at it, fully extended.">out of reach: {unreachable.map((x) => `${x.name} ${x.gap.toFixed(0)} mm`).join(", ")}</span>}
            <span className="muted">{playing && plan ? `typing · ${plan.stats.wpm.toFixed(0)} of ${wpm} wpm reached · ${(100 * plan.stats.early).toFixed(0)}% of keys started early` : reach ? `${reach.side === "L" ? "left" : "right"} ${reach.finger} reaching (IK)` : "fingertips on the home keys (IK) · hover a key to reach it"}</span>
          </div>
          <div className="viewctl">
            <div className="seg inline"><button className={reachHeat ? "on" : ""} onClick={() => setReachHeat(!reachHeat)} title="Color keys by this hand's reach cost (G01): darker = harder to reach from the home keys">Reach</button></div>
            <div className="seg inline"><button className={skin ? "on" : ""} onClick={() => setSkin(!skin)} title="Skinned hand meshes (off: joint skeletons)">Skin</button></div>
            <div className="seg inline"><button className={labels ? "on" : ""} onClick={() => setLabels(!labels)} title="Segment lengths (mm) on the resting hands">Lengths</button></div>
            <div className="seg inline"><button className={mode === "3d" ? "on" : ""} onClick={() => setMode(mode === "3d" ? "top" : "3d")} title="3D view (off: flat top-down)">3D</button></div>
          </div>
          <div className="typingbar">
            <button className="btn primary small" onClick={() => setPlaying(!playing)}>{playing ? "Stop" : "Play"}</button>
            <select value={sample} onChange={(e) => setSample(e.target.value)} title="Sample text">
              {SAMPLES.map(([k, t]) => <option key={k} value={k}>{t.length > 34 ? `${t.slice(0, 34)}…` : t}</option>)}
            </select>
            <div className="seg inline"><button className={loop ? "on" : ""} onClick={() => setLoop(!loop)} title="Repeat the sample (off: type it once, then stop)">Loop</button></div>
            <input type="range" min={5} max={200} step={5} value={wpm} onChange={(e) => setWpm(+e.target.value)} title="Target words per minute" />
            <span className="muted num">{wpm} wpm</span>
            <span className="sep" />
            <label className="muted" title="How far the next finger moves while the previous key is still down. 0: one finger at a time (each starts after the previous release); 100%: the next finger travels during the previous press and waits over its key; above 100%: keys roll over (each held into the next press) and fingers get ready ever earlier; 300%: every finger heads for its next key as soon as it is free.">Parallel</label>
            <input type="range" min={0} max={3} step={0.05} value={parallel} onChange={(e) => setParallel(+e.target.value)} title="Finger overlap" />
            <span className="muted num">{(100 * parallel).toFixed(0)}%</span>
          </div>
        </div>
      }
      bottom={
        <div className="bottomtabs">
          <PanelTabs value={tab} onChange={setTab} options={[["reach", "Reach"], ["measure", "Measurements"], ["typing", "Typing"], ["size", "Size distribution"], ["similar", "Similarity"]]}
            right={<span className="hint">{h.preset === "custom" ? "Custom" : cap(h.preset)} hand</span>} />
          <div className="bottombody">
            {tab === "reach" && <Reach geometry={p.geometry} chars={p.chars} costs={costs} body={arms} />}
            {tab === "measure" && <Measurements d={d} />}
            {tab === "typing" && <Typing plan={plan} clock={clock} playing={playing} />}
            {tab === "size" && <SizeDistribution d={d} />}
            {tab === "similar" && <Similarity d={d} />}
          </div>
        </div>
      }
    />
  );
}
