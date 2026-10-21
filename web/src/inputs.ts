// One store for the inputs every tab shares (board, corpus, profile, start layout, search).
// The input tabs edit it and Play reads it, so a run started from any tab uses the same inputs.

import { useCallback, useState } from "react";
import type { Profile } from "./api";

export interface SearchSettings { algo: string; timeS: number; threads: number; seed: number; saSteps: number; gaPop: number }

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
  /** The live (possibly edited) profile; sent with each run as `profile_override`. */
  const [profile, setProfile] = useState<Profile | null>(null);
  const [refId, setRefId] = usePref("reference", "");
  const [chars, setChars] = usePref("chars", "");
  const [label, setLabel] = usePref("label", "");
  const [startChars, setStartChars] = useState("");
  const [pins, setPins] = usePref("pins", "");
  const [search, setSearch] = usePref<SearchSettings>("search", { algo: "sa", timeS: 10, threads: 8, seed: 1, saSteps: 0, gaPop: 128 });
  return {
    geomId, setGeomId, corpusId, setCorpusId, profileId, setProfileId, profile, setProfile, refId, setRefId,
    chars, setChars, label, setLabel, startChars, setStartChars, pins, setPins, search, setSearch,
  };
}

export type Inputs = ReturnType<typeof useInputs>;
