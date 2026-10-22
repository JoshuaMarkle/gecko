// One store for the inputs every tab shares (board, hand, language, profile, start layout,
// search, working model). The input tabs edit it and Play reads it, so a run started from any
// tab uses the same inputs.

import { useCallback, useMemo, useState } from "react";
import type { Geometry, Profile } from "./api";
import { withFingerEffort } from "./fingerEffort";
import { HAND_WEIGHTS, withHand, type HandSettings } from "./hand";

export interface SearchSettings {
  algo: string; timeS: number; threads: number; seed: number; saSteps: number; gaPop: number;
  memeticPop?: number; localSearch?: "hc" | "tabu"; localSteps?: number; islands?: number; migrationInterval?: number; migrants?: number;
  /** Starting layout: "auto" (whatever is loaded) or a layout id. */
  start?: string;
  /** Budget kind: wall-clock seconds (`timeS`) or evaluations (`evals`). */
  budget?: "time" | "evals";
  evals?: number;
  /** Stop once the estimated chance of having the optimum passes this (0..1); null = off. */
  stopConf?: number | null;
  /** Corpus id the results are also scored on but never searched on (L06); "" = none. */
  holdout?: string;
}

/** Physical constraints for building a board (Board tab, Custom on). Each group can be enabled
 *  on its own; nothing consumes them yet (board generation is planned, see frontend.md). */
export interface Constraint { on: boolean; [field: string]: number | boolean }
export type Constraints = Record<string, Constraint>;

/** The selected layout after an optimization (navbar right, Home). */
export interface WorkingModel { chars: string; label: string; score: number | null; run?: string }

/** `useState` persisted in `localStorage` under `gecko.<key>`. */
export function usePref<T>(key: string, init: T): [T, (v: T) => void] {
  const [v, setV] = useState<T>(() => {
    try {
      const s = localStorage.getItem(`gecko.${key}`);
      return s ? (JSON.parse(s) as T) : init;
    } catch {
      return init;
    }
  });
  const set = useCallback((x: T) => {
    setV(x);
    try { localStorage.setItem(`gecko.${key}`, JSON.stringify(x)); } catch { /* storage unavailable */ }
  }, [key]);
  return [v, set];
}

export function useInputs() {
  const [geomId, setGeomId] = usePref("geometry", "");
  const [corpusId, setCorpusId] = usePref("corpus", "");
  const [profileId, setProfileId] = usePref("profile", "");
  /** The live (possibly edited) profile; returned with the hand merged in (`withHand`) and sent with each run as `profile_override`. */
  const [profile, setProfileRaw] = useState<Profile | null>(null);
  // Profiles come back from the views with the hand merged in; keep the stored one hand-free, and
  // drop G01 / G05 weights `withHand` added unless they were edited.
  const setProfile = (p: Profile | null) => setProfileRaw((cur) => {
    if (!p) return null;
    const { hand: _h, ...rest } = withFingerEffort(p);
    const weights = { ...rest.weights };
    for (const [id, [w]] of Object.entries(HAND_WEIGHTS)) if (!(id in (cur?.weights ?? {})) && weights[id] === w) delete weights[id];
    return { ...rest, weights };
  });
  const [refId, setRefId] = usePref("reference", "");
  const [chars, setChars] = usePref("chars", "");
  const [label, setLabel] = usePref("label", "");
  const [startChars, setStartChars] = useState("");
  const [pins, setPins] = usePref("pins", "");
  const [search, setSearch] = usePref<SearchSettings>("search", { algo: "sa", timeS: 10, threads: 8, seed: 1, saSteps: 0, gaPop: 128 });
  /** Boards forked from a bundled one by editing a key (kept in this browser only). */
  const [customBoards, setCustomBoards] = usePref<Geometry[]>("customBoards", []);
  const [constraints, setConstraints] = usePref<Constraints>("constraints", {});
  const [hand, setHand] = usePref<HandSettings>("hand", { use: false, preset: "average", custom: null });
  const [model, setModel] = usePref<WorkingModel | null>("model", null);
  // What every score and run gets: the edited profile with the Hand tab's hand merged in.
  const scoring = useMemo(() => (profile ? withHand(profile, hand) : null), [profile, hand]);
  return {
    geomId, setGeomId, corpusId, setCorpusId, profileId, setProfileId, profile: scoring, setProfile, refId, setRefId,
    chars, setChars, label, setLabel, startChars, setStartChars, pins, setPins, search, setSearch,
    customBoards, setCustomBoards, constraints, setConstraints, hand, setHand, model, setModel,
  };
}

export type Inputs = ReturnType<typeof useInputs>;
