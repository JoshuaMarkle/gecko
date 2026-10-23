// Hands on a board for the Hand tab: each hand placed over its home keys (`Rig`), the typing
// strokes for a sample text, and both hands posed at a time of the typing plan.

import type { Geometry } from "../../api";
import {
  FINGER_NAMES, fingerAt, fingerKey, FOUR, handLayout, handShift, placeHand, adjustPlacement, poseHand, sub3, toWorld,
  type FingerName, type FingerState, type HandDims, type HandPose, type Placement, type Plan, type Side, type Stroke, type V3,
  type Wrist,
} from "../../hand";
import { boardBounds, keyTop as capTop } from "../../three/Keyboard3D";

export const MM = 1 / 19.05;
const FINGER_OF: Record<string, FingerName> = { P: "pinky", R: "ring", M: "middle", I: "index", T: "thumb" };
export const CODE_OF: Record<FingerName, string> = { pinky: "P", ring: "R", middle: "M", index: "I", thumb: "T" };
/** Pause at the end of the sample before it loops, seconds. */
export const LOOP_GAP = 0.8;

// ---------- rigs and strokes ----------

/**
 * One hand on the board: its solved placement (`base`), the placement shown after the wrist
 * adjustments, how much the hand follows a reaching finger, and the fingertip home of each
 * finger (board-centred three coordinates).
 */
export interface Rig { side: Side; base: Placement; place: Placement; follow: number; home: Record<FingerName, V3> }

/**
 * Hands for a board: the four fingers rest on their home keys (non-stretch first), the hand is
 * placed by `placeHand`, and the thumb rests on the thumb key nearest its natural rest point
 * (or on that point at key height when the board has no thumb keys), so both thumbs pick
 * mirrored keys on a symmetric board. The wrist adjustments move the shown hand only; the
 * fingertips keep their homes.
 */
export function buildRigs(g: Geometry, d: HandDims, w: Wrist): { rigs: Rig[]; keyTop: (slot: number) => V3; keyNormal: (slot: number) => V3 } {
  const b = boardBounds(g.key);
  const keyTop = (i: number): V3 => capTop(g.key[i], b.cx, b.cy).p;
  const keyNormal = (i: number): V3 => capTop(g.key[i], b.cx, b.cy).n;
  const rigs: Rig[] = [];
  for (const side of ["L", "R"] as const) {
    const home = {} as Record<FingerName, V3>;
    for (const f of FOUR) {
      const ks = g.key.map((k, i) => ({ k, i })).filter((x) => x.k.finger === `${side}${CODE_OF[f]}`);
      const pick = ks.find((x) => x.k.home && !x.k.stretch) ?? ks.find((x) => x.k.home) ?? ks[0];
      if (pick) home[f] = keyTop(pick.i);
    }
    if (FOUR.some((f) => !home[f])) continue;
    const pl = placeHand(side, home, d, MM);
    const rest = toWorld(side, pl, MM, handLayout(d).thumbRest);
    const thumbs = g.key.map((k, i) => ({ k, i })).filter((x) => x.k.finger === `${side}T`).map((x) => keyTop(x.i));
    const near = (p: V3) => Math.hypot(p[0] - rest[0], p[2] - rest[2]);
    home.thumb = thumbs.length ? thumbs.reduce((a, p) => (near(p) < near(a) ? p : a)) : [rest[0], Math.min(...FOUR.map((f) => home[f][1])), rest[2]];
    rigs.push({ side, base: pl, place: adjustPlacement(side, pl, w, MM), follow: w.follow, home });
  }
  return { rigs, keyTop, keyNormal };
}

/** Keystrokes for a text on the loaded layout. Unmapped spaces go to a thumb, alternating away from the last hand. */
export function strokesFor(text: string, g: Geometry, chars: string, rigs: Rig[], keyTop: (i: number) => V3, keyNormal: (i: number) => V3): Stroke[] {
  const slotOf = new Map<string, number>();
  [...chars].forEach((c, i) => { if (i < g.key.length && !slotOf.has(c)) slotOf.set(c, i); });
  const sides = new Set(rigs.map((r) => r.side));
  const homeOf = (s: Side) => rigs.find((r) => r.side === s)!.home;
  const out: Stroke[] = [];
  for (const ch of text) {
    const slot = slotOf.get(ch === " " ? "␣" : ch) ?? slotOf.get(ch.toLowerCase());
    if (slot !== undefined) {
      const f = g.key[slot].finger;
      const side = f[0] as Side;
      if (sides.has(side)) out.push({ side, finger: FINGER_OF[f[1]], pos: keyTop(slot), ch, key: slot, n: keyNormal(slot) });
    } else if (ch === " " && sides.size) {
      const prev = out[out.length - 1]?.side;
      let side: Side = prev === "R" ? "L" : "R";
      if (!sides.has(side)) side = side === "L" ? "R" : "L";
      out.push({ side, finger: "thumb", pos: homeOf(side).thumb, ch });
    }
  }
  return out;
}


export interface Frame3 { pose: HandPose; states: Record<FingerName, FingerState> }

/**
 * Both hands at time `t` of the plan (`t` < 0: resting). The hand follows its fingers' excursions;
 * resting fingers ride along. `lift` (world units) raises each fingertip target, for a mesh whose
 * tip joint sits above its pad.
 */
export function framesAt(rigs: Rig[], plan: Plan | null, d: HandDims, t: number, lift?: Record<FingerName, number>): Frame3[] {
  return rigs.map((r) => {
    const states = {} as Record<FingerName, FingerState>;
    const pos = {} as Record<FingerName, V3>;
    const ex: [FingerName, V3][] = [];
    for (const f of FINGER_NAMES) {
      const a = plan && t >= 0 ? fingerAt(plan.byFinger[fingerKey(r.side, f)], r.home[f], t, MM) : { pos: r.home[f], state: "rest" as const };
      states[f] = a.state;
      pos[f] = a.pos;
      ex.push([f, sub3(a.pos, r.home[f])]);
    }
    const off = handShift(ex, r.follow);
    const targets = {} as Record<FingerName, V3>;
    for (const f of FINGER_NAMES) {
      const p: V3 = states[f] === "rest" ? [r.home[f][0] + off[0], r.home[f][1], r.home[f][2] + off[2]] : pos[f];
      targets[f] = lift ? [p[0], p[1] + lift[f], p[2]] : p;
    }
    return { pose: poseHand(r.side, r.place, targets, d, MM, off), states };
  });
}
