// Home (web-screens.md "Home"): the optimization flowchart. Inputs → physical cost and
// frequencies → scoring → the four optimize modes → saved runs and export; every box opens its
// tab and shows what the next run would use.

import type { CorpusInfo, Geometry, Meta, Profile, RunSummary } from "../../api";
import { fmt, fmtCorpusChars, fmtCount } from "../../format";
import { GEOM_DEFAULTS } from "../../geom";
import { handDims, handLength, type HandSettings } from "../../hand";
import type { SearchSettings } from "../../inputs";
import type { View } from "../../tabs";
import { FlowChart, type FlowBox } from "./FlowChart";
import { algoName } from "./LayoutView";

interface Props {
  meta: Meta;
  geometry: Geometry;
  isCustomBoard: boolean;
  hand: HandSettings;
  profile: Profile;
  profileEdited: boolean;
  corpus?: CorpusInfo;
  search: SearchSettings;
  /** Score and label of the shown layout. */
  score: number | null;
  label: string;
  /** Layout runs on exactly the current inputs. */
  matching: number;
  runs: RunSummary[];
  /** The optimize tab last used (highlighted). */
  active: View | null;
  onGo: (v: View) => void;
}

const budget = (kind: "time" | "evals" | undefined, s: number, evals: number, unit: string) => kind === "evals" ? `${fmtCount(evals)} ${unit}` : `${s}s`;

export function HomeView(p: Props) {
  const g = { ...GEOM_DEFAULTS, ...p.search.geom };
  const w = p.profile.weights;
  const weighted = Object.entries(w).filter(([id, v]) => v !== 0 && p.meta.metrics.some((m) => m.id === id && m.dir !== "info")).length;
  const handOn = ["G01", "G03", "G05", "G06"].some((id) => (w[id] ?? 0) !== 0);
  const param = !!p.geometry.param;
  const runnable = !p.isCustomBoard && !!p.corpus;
  const boxes: Partial<Record<string, FlowBox>> = {
    board: { title: "Board", state: p.isCustomBoard ? "warn" : "ok",
      lines: [`${p.geometry.name} · ${p.geometry.key.length} keys`, p.isCustomBoard ? "Custom: cannot be run yet" : param ? "Parametric: shape can move" : "Fixed shape"] },
    hand: { title: "Hand", state: p.hand.use ? "ok" : "off",
      lines: [`${p.hand.preset} · ${handLength(handDims(p.hand)).toFixed(0)} mm`, p.hand.use ? "Used in optimization" : "Preview only"] },
    profile: { title: "Profile", state: "ok", lines: [`${p.profile.name}${p.profileEdited ? " (edited)" : ""}`, `${weighted} weighted metrics`] },
    language: { title: "Language", state: p.corpus ? "ok" : "warn",
      lines: p.corpus ? [p.corpus.name, `${fmtCorpusChars(p.corpus.chars)} · ${p.corpus.charset.length} symbols`] : ["Local language", "Cannot be run yet"] },
    cost: { title: "Physical cost", lines: ["Effort per key · finger load", handOn ? "Reach · press angle · wrist" : "Hand model unweighted", "Movement · rolls · sequences"] },
    freq: { title: "Frequencies", lines: ["Characters · bigrams", "Trigrams · skipgrams"] },
    score: { title: "Scoring model", lines: ["Frequency × physical cost", "Profile weights · lower is better", p.score !== null ? `${p.label}: ${fmt(p.score, 3)}` : "No layout scored"] },
    summary: { title: "Summary", lines: ["Every input of the next run", `${weighted} / ${p.meta.metrics.length} metrics weighted`] },
    layout: { title: "Layout", state: runnable ? "ok" : "warn",
      lines: ["Layout for the board", `${algoName(p.search.algo)} · ${budget(p.search.budget, p.search.timeS, p.search.evals ?? 50_000_000, "evals")}`, `${p.matching} run${p.matching === 1 ? "" : "s"} on these inputs`] },
    shape: { title: "Board", state: runnable && param ? "ok" : "warn",
      lines: ["Board for the layout", param ? `${g.groups.length} shape groups` : "Needs a parametric board", budget(g.budget, g.timeS ?? 60, g.evals, "boards")] },
    both: { title: "Both", state: runnable && param ? "ok" : "warn",
      lines: ["Board and layout together", param ? `${g.mode} · ${g.groups.length} groups` : "Needs a parametric board", budget(g.budget, g.timeS ?? 60, g.evals, "boards")] },
    explore: { title: "Explore", state: runnable ? "ok" : "warn",
      lines: ["Map of good layouts", `${p.runs.filter((r) => r.algo === "explore").length} atlases`] },
    runs: { title: "Runs", lines: [`${p.runs.length} saved`, `${p.runs.filter((r) => r.kind === "geometry").length} board runs`] },
    export: { title: "Export", lines: [clipLabel(p.label), "Image · files"] },
  };
  return (
    <div className="homeflow">
      <header className="homeflow-head">
        <h1>Optimization flow</h1>
        <p>Inputs become a physical cost per key and symbol frequencies; the scoring model combines them, and each optimize mode searches it. Click a box to open its tab.</p>
      </header>
      <FlowChart boxes={boxes} active={p.active} onGo={p.onGo} />
    </div>
  );
}

const clipLabel = (s: string) => s || "No layout";
