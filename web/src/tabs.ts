// Navbar tabs (web-screens.md "Shell"): inputs, then results. Old stored tab names map to new ones.

export type InputTabName = "board" | "hand" | "language" | "profile" | "search" | "summary";
export type ResultTabName = "home" | "explore" | "analysis" | "saved" | "export";
export type View = InputTabName | ResultTabName;

export const INPUT_TABS: [InputTabName, string][] = [["board", "Board"], ["hand", "Hand"], ["language", "Language"], ["profile", "Profile"], ["search", "Search"], ["summary", "Summary"]];
export const RESULT_TABS: [ResultTabName, string][] = [["home", "Home"], ["explore", "Explore"], ["analysis", "Analysis"], ["saved", "Saved"], ["export", "Export"]];

/** Renames: Corpus → Language, Stats → Analysis, Runs → Saved, Optimize → Home (older names too). */
const OLD: Record<string, View> = { corpus: "language", stats: "analysis", runs: "saved", optimize: "home", results: "saved" };

export function asView(v: string): View {
  if (OLD[v]) return OLD[v];
  return [...INPUT_TABS, ...RESULT_TABS].some(([x]) => x === v) ? (v as View) : "home";
}
