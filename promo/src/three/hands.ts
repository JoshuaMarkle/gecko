// The product's hand model on the promo Corne: rigs on the home keys, a typing plan for a text, and
// both hands posed at any time of it. All of it is web/src/hand.ts (placement, finger and thumb IK,
// typing schedule) at 1 world unit per mm; this file only finds the keys, as the Hand tab's
// components/hand/rigs.ts does on its board. Deterministic: a time gives the same pose every render.

import * as THREE from "three";
import {
  AVERAGE, dist3, FINGER_NAMES, fingerAt, fingerKey, FOUR, handLayout, handLength, handShift, keysDown, LENGTH_MEAN, placeHand,
  planTyping, poseHand, sub3, toWorld, WRIST0,
  type FingerName, type FingerState, type HandDims, type HandPose, type Placement, type Plan, type Press, type Side, type Stroke, type V3,
} from "../../../web/src/hand";
import { data } from "../lib/data";
import { keyTransform } from "./Corne3D";
import { CAP_TOP } from "./Switch";

export type { FingerName, FingerState, HandPose, Press, Side, V3 };
export { FINGER_NAMES };

const KEYS = data.board.keys;
const FINGER_OF: Record<string, FingerName> = { P: "pinky", R: "ring", M: "middle", I: "index", T: "thumb" };
const CODE_OF: Record<FingerName, string> = { pinky: "P", ring: "R", middle: "M", index: "I", thumb: "T" };
/** Height of the mesh's tip joint above the pad touching the key, average hand, mm (web HandMesh.tsx `PAD`). */
const PAD: Record<FingerName, number> = { thumb: 11, index: 16, middle: 17, ring: 17, pinky: 14 };

export const DIMS: HandDims = AVERAGE;

/** Cap-top centre (mm, scene) and unit normal of key i on the flat Corne (Corne3D defaults). */
export function capTop(i: number, spread = 36, lift = 0): { p: V3; n: V3 } {
  const { pos, rot } = keyTransform(i, KEYS[i].pose, spread, lift);
  const n = new THREE.Vector3(0, 1, 0).applyEuler(rot);
  const p = pos.clone().addScaledVector(n, CAP_TOP);
  return { p: [p.x, p.y, p.z], n: [n.x, n.y, n.z] };
}

/** One hand on the board: its solved placement and the fingertip home of each finger. */
export interface Rig { side: Side; place: Placement; home: Record<FingerName, V3> }

/** Both hands with the four fingers on their home keys and the thumb on the thumb key nearest its natural rest (web `buildRigs`). */
export function buildRigs(d: HandDims = DIMS, spread = 36): Rig[] {
  return (["L", "R"] as const).map((side) => {
    const home = {} as Record<FingerName, V3>;
    for (const f of FOUR) {
      const ks = KEYS.map((k, i) => ({ k, i })).filter((x) => x.k.finger === `${side}${CODE_OF[f]}`);
      const pick = ks.find((x) => x.k.home && !x.k.stretch) ?? ks.find((x) => x.k.home) ?? ks[0];
      home[f] = capTop(pick.i, spread).p;
    }
    const place = placeHand(side, home, d, 1);
    const rest = toWorld(side, place, 1, handLayout(d).thumbRest);
    const near = (p: V3) => Math.hypot(p[0] - rest[0], p[2] - rest[2]);
    const thumbs = KEYS.map((k, i) => ({ k, i })).filter((x) => x.k.finger === `${side}T`).map((x) => capTop(x.i, spread).p);
    home.thumb = thumbs.reduce((a, p) => (near(p) < near(a) ? p : a));
    return { side, place, home };
  });
}

/** Keystrokes for `text` on a layout (one symbol per key); unmapped characters are skipped, capitals typed as their letter. */
export function strokesFor(text: string, chars: string, spread = 36): Stroke[] {
  const slot = new Map<string, number>();
  [...chars].forEach((c, i) => { if (i < KEYS.length && !slot.has(c)) slot.set(c, i); });
  const out: Stroke[] = [];
  for (const ch of text) {
    const i = slot.get(ch === " " ? "␣" : ch) ?? slot.get(ch.toLowerCase());
    if (i === undefined) continue;
    const f = KEYS[i].finger;
    const { p, n } = capTop(i, spread);
    out.push({ side: f[0] as Side, finger: FINGER_OF[f[1]], pos: p, ch, key: i, n });
  }
  return out;
}

export interface Frame3 { side: Side; pose: HandPose; states: Record<FingerName, FingerState> }

/** A typing session: the rigs, the plan, and a sampler for any time. */
export interface Typing {
  rigs: Rig[];
  plan: Plan;
  /** Both hands at time `t` (s; < 0: resting on the home keys). */
  frames: (t: number) => Frame3[];
  /** Press depth per key (0..1) at time `t`. */
  keys: (t: number) => number[];
  /** The latest press that has reached its key by time `t`, or null. */
  pressAt: (t: number) => Press | null;
}

/**
 * Plan `text` on `chars` at `wpm` with finger overlap `parallel` (web planTyping, natural pace),
 * hands of size `d`. Fingertip targets are raised by the mesh's pad height (`pad`), as the Hand
 * tab does for the skinned hands, so the pads (not the tip joints) touch the caps.
 */
export function typing(text: string, chars: string, { wpm = 90, parallel = 1, d = DIMS, spread = 36, pad = true } = {}): Typing {
  const rigs = buildRigs(d, spread);
  const homeOf = (s: Side, f: FingerName) => rigs.find((r) => r.side === s)!.home[f];
  const plan = planTyping(strokesFor(text, chars, spread), homeOf, wpm, parallel, 1);
  const k = handLength(d) / LENGTH_MEAN;
  const lift = Object.fromEntries(FINGER_NAMES.map((f) => [f, pad ? PAD[f] * k : 0])) as Record<FingerName, number>;
  const frames = (t: number): Frame3[] => rigs.map((r) => {
    const states = {} as Record<FingerName, FingerState>;
    const pos = {} as Record<FingerName, V3>;
    const ex: [FingerName, V3][] = [];
    for (const f of FINGER_NAMES) {
      const a = t >= 0 ? fingerAt(plan.byFinger[fingerKey(r.side, f)], r.home[f], t, 1) : { pos: r.home[f], state: "rest" as const };
      states[f] = a.state; pos[f] = a.pos;
      ex.push([f, sub3(a.pos, r.home[f])]);
    }
    const off = handShift(ex, WRIST0.follow);
    const targets = {} as Record<FingerName, V3>;
    for (const f of FINGER_NAMES) {
      const p: V3 = states[f] === "rest" ? [r.home[f][0] + off[0], r.home[f][1], r.home[f][2] + off[2]] : pos[f];
      targets[f] = [p[0], p[1] + lift[f], p[2]];
    }
    return { side: r.side, pose: poseHand(r.side, r.place, targets, d, 1, off), states };
  });
  const buf = new Float32Array(KEYS.length);
  const keys = (t: number) => { keysDown(plan, t, buf); return Array.from(buf); };
  const pressAt = (t: number) => {
    let last: Press | null = null;
    for (const pr of plan.presses) if (pr.arrive <= t) last = pr; else break;
    return last;
  };
  return { rigs, plan, frames, keys, pressAt };
}

/** Fingertip travel for a press (mm): from where the finger set off to the key. */
export const travelMm = (pr: Press) => dist3(pr.from, pr.pos);

/**
 * Press angle (degrees) of a posed finger: between its distal segment and the key's normal
 * (0 = pressing straight down the key's axis).
 */
export function pressAngle(pose: HandPose, finger: FingerName, n: V3 = [0, 1, 0]): number {
  const ch = pose.fingers[FINGER_NAMES.indexOf(finger)];
  const a = ch[ch.length - 2], b = ch[ch.length - 1];
  const s = sub3(a, b), l = Math.hypot(...s) || 1;
  return (Math.acos(Math.max(-1, Math.min(1, (s[0] * n[0] + s[1] * n[1] + s[2] * n[2]) / l))) * 180) / Math.PI;
}
