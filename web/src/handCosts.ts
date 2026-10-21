// Per-key hand-model costs (G01 reach, G05 press angle, G03 wrist posture) for a board and
// hand, from the WASM scorer, for the Hand and Profile tabs. Null while loading or when WASM is
// unavailable.

import { useEffect, useState } from "react";
import type { Geometry } from "./api";
import type { HandSpec } from "./hand";
import { handCosts, type HandCosts } from "./wasm";

export function useHandCosts(g: Geometry, spec: HandSpec): HandCosts | null {
  const [c, setC] = useState<HandCosts | null>(null);
  const key = JSON.stringify(spec);
  useEffect(() => {
    let alive = true;
    handCosts(g, spec).then((x) => alive && setC(x), () => alive && setC(null));
    return () => { alive = false; };
  }, [g, key]);
  return c;
}
