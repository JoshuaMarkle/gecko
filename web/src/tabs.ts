// Navbar tabs (web-screens.md "Shell"): Home, then the Inputs, Optimize and Saved groups.
// Old stored tab names map to new ones.

export type InputTabName = "board" | "hand" | "language" | "profile" | "summary";
/** layout: the layout for a fixed board; shape: the board for a fixed layout; both: board and layout together. */
export type OptimizeTabName = "layout" | "shape" | "both" | "explore";
export type SavedTabName = "runs" | "bench" | "export";
export type View = "home" | InputTabName | OptimizeTabName | SavedTabName;

export const INPUT_TABS: [InputTabName, string][] = [["board", "Board"], ["hand", "Hand"], ["language", "Language"], ["profile", "Profile"], ["summary", "Summary"]];
export const OPTIMIZE_TABS: [OptimizeTabName, string][] = [
  ["layout", "Optimize the layout for the board"], ["shape", "Optimize the board for the layout"],
  ["both", "Optimize board and layout together"], ["explore", "Explore the search space"],
];
export const SAVED_TABS: [SavedTabName, string][] = [["runs", "Saved runs"], ["bench", "Algorithm bench"], ["export", "Export"]];

/** Navbar groups in order; the first has no label. */
export const TAB_GROUPS: [string, [View, string][]][] = [["", [["home", "Home"]]], ["Inputs", INPUT_TABS], ["Optimize", OPTIMIZE_TABS], ["Saved", SAVED_TABS]];

export const isOptimizeTab = (v: View): v is OptimizeTabName => OPTIMIZE_TABS.some(([t]) => t === v);

/** Renames: Search / Analysis / Optimize / Stats → Layout, Saved / Runs / Results → Runs, Corpus → Language. */
const OLD: Record<string, View> = { corpus: "language", search: "layout", analysis: "layout", stats: "layout", optimize: "layout", saved: "runs", results: "runs" };

export function asView(v: string): View {
  if (OLD[v]) return OLD[v];
  return TAB_GROUPS.some(([, tabs]) => tabs.some(([x]) => x === v)) ? (v as View) : "home";
}
