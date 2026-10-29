// The promo's timeline (frames at 30 fps). STORYBOARD.md is the prose; this file wins.
// Scenes overlap by their transitions: each `from` is where the scene mounts, `dur` how long.

import { sec } from "./lib/motion";
import { APP_S } from "./scenes/App";
import { LANGUAGE_S } from "./scenes/Language";
import { ORTHO } from "./scenes/Opening";
import { PHYSICAL_S } from "./scenes/Physical";

/** Scene lengths and how far each overlaps the one before it (s). */
const PLAN = [
  /** 3D: slow macro shots of the Corne, the rise to the top view, the flatten toward the 2D drawing. */
  ["opening", ORTHO / 30 + 0.5, 0],
  /** 2D: the drawing becomes numbers, then the layout search swaps legends beside its live stats. */
  ["optimize", 8, 0.4],
  /** CSS 3D: dive into the board, fly a tunnel of the data behind it. */
  ["tunnel", 5.5, 0.5],
  /** A document of the novel; its lines chain into one, run as data through a loop and fill the letter-frequency chart. */
  ["language", LANGUAGE_S, 0.5],
  /** 3D: hands type on the final layout; the skeleton with its bone lengths; the board reshapes; the STL closes around it. */
  ["physical", PHYSICAL_S, 0.5],
  /** CSS 3D: the app, its panels lifting off. */
  ["app", APP_S, 0.5],
  /** The wordmark; the video ends on it. */
  ["end", 5, 0.5],
] as const;

type Id = (typeof PLAN)[number][0];
export const SCENES = {} as Record<Id, { from: number; dur: number }>;
let at = 0;
for (const [id, dur, overlap] of PLAN) {
  const from = at - sec(overlap);
  SCENES[id] = { from, dur: sec(dur) };
  at = from + sec(dur);
}

export const PROMO_FRAMES = SCENES.end.from + SCENES.end.dur;
