// The promo: scenes on one timeline (storyboard.ts), black stage.

import type { ReactNode } from "react";
import { Sequence } from "remotion";
import { Stage } from "./scenes/common";
import { Opening } from "./scenes/Opening";
import { Optimize } from "./scenes/Optimize";
import { Tunnel } from "./scenes/Tunnel";
import { Language } from "./scenes/Language";
import { Physical } from "./scenes/Physical";
import { App } from "./scenes/App";
import { End } from "./scenes/End";
import { SCENES } from "./storyboard";

const SCENE_VIEWS: Partial<Record<keyof typeof SCENES, () => ReactNode>> = {
  opening: Opening,
  optimize: Optimize,
  tunnel: Tunnel,
  language: Language,
  physical: Physical,
  app: App,
  end: End,
};

export function Promo() {
  return (
    <Stage>
      {(Object.keys(SCENES) as (keyof typeof SCENES)[]).map((id) => {
        const View = SCENE_VIEWS[id];
        const { from, dur } = SCENES[id];
        return View ? <Sequence key={id} name={id} from={from} durationInFrames={dur}><View /></Sequence> : null;
      })}
    </Stage>
  );
}
