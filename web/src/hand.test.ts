import { describe, expect, it } from "vitest";
import {
  adjustPlacement, AVERAGE, dimsVector, WRIST0, fingerAt, fingerChain, fingerKey, FOUR, keysDown, handLength, normalCdf, placeHand, planTyping,
  population, poseHand, scaleDims, similarity, sizeBand, type FingerName, type Side, type Stroke, type V3,
  HAND_WEIGHTS, handSpec, withHand, type HandSettings, Thumb, handLayout, toLocal, toWorld, bodyOf, BODY,
} from "./hand";
import type { Profile } from "./api";

const dist = (a: V3, b: V3) => Math.hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2]);
const U = 1 / 19.05;

describe("fingerChain", () => {
  const l: [number, number, number] = [3, 2, 1.5];
  it("keeps segment lengths and reaches a target within range", () => {
    const base: V3 = [0, 3, 3], target: V3 = [0.5, 0, 0];
    const pts = fingerChain(base, target, l);
    expect(pts).toHaveLength(4);
    pts.slice(1).forEach((p, i) => expect(dist(pts[i], p)).toBeCloseTo(l[i], 6));
    expect(dist(pts[3], target)).toBeLessThan(1e-3);
  });
  it("points straight at an unreachable target", () => {
    const pts = fingerChain([0, 0, 0], [0, 0, -20], l);
    expect(pts[3][2]).toBeCloseTo(-6.5, 6);
  });
  it("curls at the knuckle for a near, low key instead of lifting the first segment", () => {
    const base: V3 = [0, 2, 0], target: V3 = [0, 0, -3];
    const pts = fingerChain(base, target, l);
    expect(dist(pts[3], target)).toBeLessThan(1e-3);
    expect(pts[1][1]).toBeLessThanOrEqual(base[1] + l[0] * Math.sin(0.35) + 1e-9);
  });
});

/** Home keys of a symmetric board: index … pinky at x = ±(1.5 … 4.5), thumbs below. */
function homes(): Record<Side, Record<FingerName, V3>> {
  const side = (s: 1 | -1) => {
    const h = {} as Record<FingerName, V3>;
    FOUR.forEach((f, i) => { h[f] = [s * (1.5 + i), 0.4, i === 3 ? 0.3 : i === 1 ? -0.2 : 0]; });
    h.thumb = [s * 1, 0.4, 2.5];
    return h;
  };
  return { L: side(-1), R: side(1) };
}

describe("hand placement", () => {
  const h = homes();
  it("mirrors exactly for mirrored home keys", () => {
    const l = placeHand("L", h.L, AVERAGE, U), r = placeHand("R", h.R, AVERAGE, U);
    expect(l.wrist[0]).toBeCloseTo(-r.wrist[0], 9);
    expect(l.wrist[1]).toBeCloseTo(r.wrist[1], 9);
    expect(l.wrist[2]).toBeCloseTo(r.wrist[2], 9);
    expect(l.yaw).toBeCloseTo(-r.yaw, 9);
    const pl = poseHand("L", l, h.L, AVERAGE, U), pr = poseHand("R", r, h.R, AVERAGE, U);
    pl.fingers.flat().forEach((p, i) => {
      const q = pr.fingers.flat()[i];
      expect(p[0]).toBeCloseTo(-q[0], 6);
      expect(p[1]).toBeCloseTo(q[1], 6);
      expect(p[2]).toBeCloseTo(q[2], 6);
    });
  });
  it("puts the wrist toward the user and the fingertips on the home keys", () => {
    const r = placeHand("R", h.R, AVERAGE, U);
    expect(r.wrist[2]).toBeGreaterThan(h.R.middle[2] + 4);
    const pose = poseHand("R", r, h.R, AVERAGE, U);
    FOUR.forEach((f, i) => expect(dist(pose.fingers[i + 1][3], h.R[f])).toBeLessThan(1e-3));
  });
});

describe("wrist adjustments", () => {
  const h = homes();
  it("stay mirrored and keep the fingertips on their keys", () => {
    const w = { height: 5, distance: 5, spread: 8, angle: 8, tilt: 10, follow: 0.3 };
    const l = adjustPlacement("L", placeHand("L", h.L, AVERAGE, U), w, U), r = adjustPlacement("R", placeHand("R", h.R, AVERAGE, U), w, U);
    const pl = poseHand("L", l, h.L, AVERAGE, U), pr = poseHand("R", r, h.R, AVERAGE, U);
    pl.palm.forEach((p, i) => {
      expect(p[0]).toBeCloseTo(-pr.palm[i][0], 6);
      expect(p[1]).toBeCloseTo(pr.palm[i][1], 6);
      expect(p[2]).toBeCloseTo(pr.palm[i][2], 6);
    });
    const small = poseHand("R", adjustPlacement("R", placeHand("R", h.R, AVERAGE, U), { ...WRIST0, height: 5, tilt: 5 }, U), h.R, AVERAGE, U);
    FOUR.forEach((f, i) => expect(dist(small.fingers[i + 1][3], h.R[f])).toBeLessThan(1e-3));
  });
  it("tilt lifts the thumb side", () => {
    const base = placeHand("R", h.R, AVERAGE, U);
    const flat = poseHand("R", base, h.R, AVERAGE, U), tilted = poseHand("R", adjustPlacement("R", base, { ...WRIST0, tilt: 20 }, U), h.R, AVERAGE, U);
    expect(tilted.fingers[1][0][1]).toBeGreaterThan(flat.fingers[1][0][1]); // index knuckle up
    expect(tilted.fingers[4][0][1]).toBeLessThan(flat.fingers[4][0][1]); // pinky knuckle down
  });
  it("rolls the fingers' flexion planes with the hand", () => {
    const base = placeHand("R", h.R, AVERAGE, U);
    const flat = poseHand("R", base, h.R, AVERAGE, U), tilted = poseHand("R", adjustPlacement("R", base, { ...WRIST0, tilt: 20 }, U), h.R, AVERAGE, U);
    // Each flexion axis (toward the pinky) dips by about the 20° roll (sin 20° ≈ 0.34).
    FOUR.forEach((_, i) => expect(tilted.axes[i][1]).toBeLessThan(flat.axes[i][1] - 0.25));
  });
  it("leans the pinky in toward the thumb side, the index barely", () => {
    for (const side of ["L", "R"] as const) {
      const pose = poseHand(side, placeHand(side, h[side], AVERAGE, U), h[side], AVERAGE, U);
      expect(pose.axes[3][1]).toBeLessThan(-0.2); // pinky plane leaned ~15°
      expect(pose.axes[0][1]).toBeGreaterThan(-0.1);
      // The pinky's PIP bows out to the pinky side of its knuckle → tip chord.
      const [k, , pip, tip] = pose.fingers[4];
      const c = tip.map((v, j) => v - k[j]), q = pip.map((v, j) => v - k[j]);
      const t = (q[0] * c[0] + q[1] * c[1] + q[2] * c[2]) / (c[0] ** 2 + c[1] ** 2 + c[2] ** 2);
      const out = side === "R" ? 1 : -1;
      expect(out * (q[0] - t * c[0])).toBeGreaterThan(0);
    }
  });
});

describe("typing plan", () => {
  const h = homes();
  const home = (s: Side, f: FingerName) => h[s][f];
  // Alternating hands, a different finger each time: nothing but the rhythm should limit it.
  const alt: Stroke[] = Array.from({ length: 20 }, (_, i) => {
    const side: Side = i % 2 ? "R" : "L";
    const f = FOUR[i % 4];
    const p = h[side][f];
    return { side, finger: f, pos: [p[0], p[1], p[2] + (i % 8 < 4 ? -1 : 1)], ch: "x" };
  });
  it("types on the set rhythm, rushing fingers when the overlap is too small", () => {
    const par = planTyping(alt, home, 100, 1, U, true), ser = planTyping(alt, home, 100, 0, U, true);
    expect(par.stats.wpm).toBeCloseTo(100, 6);
    expect(ser.stats.wpm).toBeCloseTo(100, 6);
    expect(ser.stats.naturalWpm).toBeLessThan(par.stats.naturalWpm * 0.8);
    expect(ser.stats.rushed).toBeGreaterThan(par.stats.rushed);
    expect(ser.stats.serialWaits).toBeGreaterThan(0);
    expect(par.stats.early).toBeGreaterThan(0.9);
    expect(ser.stats.early).toBe(0);
  });
  it("waits instead of rushing by default", () => {
    const loose = planTyping(alt, home, 100, 0, U);
    expect(loose.stats.rushed).toBe(0);
    for (const p of [0, 0.3, 0.8, 1.5, 3]) expect(planTyping(alt, home, 150, p, U).stats.maxRush).toBe(1);
    expect(loose.stats.wpm).toBeCloseTo(loose.stats.naturalWpm, 6);
    expect(loose.stats.wpm).toBeLessThan(80);
  });
  it("at 300% every finger heads for its next key as soon as it is free", () => {
    const plan = planTyping(alt, home, 40, 3, U);
    for (const list of Object.values(plan.byFinger)) {
      list.slice(1).forEach((p, i) => {
        const prev = list[i];
        const free = Number.isFinite(prev.back) ? prev.back : prev.up;
        expect(p.depart - free).toBeLessThan(0.06);
      });
    }
  });
  it("rolls keys over above 100%: presses overlap and fingers get ready earlier", () => {
    const one = planTyping(alt, home, 150, 1, U), two = planTyping(alt, home, 150, 2, U);
    const overlaps = (pl: typeof one) => pl.presses.filter((p, i) => i && p.arrive < pl.presses[i - 1].up).length;
    expect(overlaps(one)).toBe(0);
    expect(overlaps(two)).toBeGreaterThan(0);
    // At a slow pace the next finger waits over its key for longer.
    const wait = (pl: typeof one) => pl.presses.reduce((t, p) => t + (p.arrive - p.ready), 0);
    expect(wait(planTyping(alt, home, 30, 2, U))).toBeGreaterThan(wait(planTyping(alt, home, 30, 1, U)));
  });
  it("marks a key down exactly while it is pressed", () => {
    const strokes: Stroke[] = alt.map((s, i) => ({ ...s, key: i % 3 }));
    const plan = planTyping(strokes, home, 60, 0.8, U);
    const out = new Float32Array(3);
    const pr = plan.presses[4];
    keysDown(plan, (pr.arrive + pr.up) / 2, out);
    expect(out[pr.key!]).toBeCloseTo(1, 6);
    keysDown(plan, -1, out);
    expect([...out]).toEqual([0, 0, 0]);
  });
  it("presses along a tilted key's normal (3D boards)", () => {
    const n: [number, number, number] = [-Math.sin(0.5), Math.cos(0.5), 0];
    const strokes: Stroke[] = alt.map((s, i) => (i === 4 ? { ...s, n } : s));
    const plan = planTyping(strokes, home, 60, 0.8, U);
    const pr = plan.presses[4];
    const tip = fingerAt(plan.byFinger[fingerKey(pr.side, pr.finger)], home(pr.side, pr.finger), (pr.arrive + pr.up) / 2, U).pos;
    const d = [tip[0] - pr.pos[0], tip[1] - pr.pos[1], tip[2] - pr.pos[2]];
    const depth = Math.hypot(d[0], d[1], d[2]);
    expect(depth).toBeGreaterThan(0);
    // The dip is exactly opposite the normal.
    for (let k = 0; k < 3; k++) expect(d[k] / depth).toBeCloseTo(-n[k], 6);
  });
  it("ends once every finger is back home", () => {
    const plan = planTyping(alt, home, 80, 0.8, U);
    expect(Number.isFinite(plan.end)).toBe(true);
    expect(plan.end).toBeGreaterThanOrEqual(plan.presses[plan.presses.length - 1].up);
  });
  it("orders each press: depart ≤ ready ≤ arrive < up, presses in sequence", () => {
    const plan = planTyping(alt, home, 60, 0.7, U);
    plan.presses.forEach((p, i) => {
      expect(p.depart).toBeLessThanOrEqual(p.ready + 1e-9);
      expect(p.ready).toBeLessThanOrEqual(p.arrive + 1e-9);
      expect(p.arrive).toBeLessThan(p.up);
      if (i) expect(p.arrive).toBeGreaterThan(plan.presses[i - 1].arrive);
    });
  });
  it("counts same-finger waits on repeated far keys", () => {
    const p = h.R.index;
    const sf: Stroke[] = [0, 1, 2, 3].map((i) => ({ side: "R", finger: "index", pos: [p[0], p[1], p[2] + (i % 2 ? -1 : 1)], ch: "x" }));
    const plan = planTyping(sf, home, 150, 1, U);
    expect(plan.stats.fingerWaits).toBeGreaterThan(0);
    expect(planTyping(sf, home, 150, 1, U, true).stats.fingerWaits).toBeGreaterThan(0);
  });
  it("moves each fingertip continuously", () => {
    const plan = planTyping(alt, home, 80, 0.8, U);
    for (const s of ["L", "R"] as Side[]) for (const f of FOUR) {
      const list = plan.byFinger[fingerKey(s, f)];
      let prev = fingerAt(list, h[s][f], 0, U).pos;
      for (let t = 0.002; t < plan.end + 0.2; t += 0.002) {
        const cur = fingerAt(list, h[s][f], t, U).pos;
        expect(dist(prev, cur)).toBeLessThan(0.08);
        prev = cur;
      }
      expect(dist(prev, h[s][f])).toBeLessThan(1e-9);
    }
  });
});

describe("hand sizes", () => {
  it("bands the presets", () => {
    expect(sizeBand(handLength(AVERAGE))).toBe("Average");
    expect(sizeBand(handLength(scaleDims(AVERAGE, 0.88)))).toBe("Small");
    expect(sizeBand(handLength(scaleDims(AVERAGE, 1.12)))).toBe("Large");
  });
  it("normal CDF is symmetric", () => {
    expect(normalCdf(0)).toBeCloseTo(0.5, 6);
    expect(normalCdf(1.96)).toBeCloseTo(0.975, 3);
  });
  it("similarity is 1 for identical hands and falls with distance", () => {
    const v = dimsVector(AVERAGE);
    expect(similarity(v, v)).toBe(1);
    expect(similarity(v, dimsVector(scaleDims(AVERAGE, 1.1)))).toBeLessThan(similarity(v, dimsVector(scaleDims(AVERAGE, 1.02))));
  });
  it("population is deterministic", () => {
    expect(population(5, 3)).toEqual(population(5, 3));
  });
});

describe("hand in the scored profile", () => {
  const base: Profile = { name: "p", description: "", weights: { B01: 1 }, finger_target: [], trigram_coverage: 0.99, scale: { B01: 2 } };
  const on: HandSettings = { use: true, preset: "large", custom: null };

  it("adds the hand and G01 / G03 / G05 / G06 at the hand profile's weights when on", () => {
    const p = withHand(base, on);
    expect(p.hand).toEqual({ size: "large" });
    expect(p.weights).toEqual({ B01: 1, G01: HAND_WEIGHTS.G01[0], G03: HAND_WEIGHTS.G03[0], G05: HAND_WEIGHTS.G05[0], G06: HAND_WEIGHTS.G06[0] });
    expect(p.scale?.G01).toBe(HAND_WEIGHTS.G01[1]);
    expect(base.weights.G01).toBeUndefined();
  });

  it("keeps weights the profile sets, 0 included, and drops the hand when off", () => {
    const set = { ...base, weights: { B01: 1, G01: 0 } };
    expect(withHand(set, on).weights.G01).toBe(0);
    const off = withHand({ ...base, hand: { size: "small" } }, { ...on, use: false });
    expect(off.hand).toBeUndefined();
    expect(off.weights).toEqual(base.weights);
  });

  it("sends custom measurements in the scorer's snake_case", () => {
    const spec = handSpec({ use: true, preset: "custom", custom: { ...AVERAGE, palmWidth: 90 } });
    expect(spec.dims?.palm_width).toBe(90);
    expect(spec.dims?.middle).toEqual(AVERAGE.middle);
    expect(handSpec({ use: true, preset: "custom", custom: null })).toEqual({ size: "average" });
  });

  it("sends arms only when set, and scales default arms with the hand", () => {
    const arms = { elbowWidth: 500, forearm: 260 };
    expect(handSpec({ ...on, body: arms }).body).toEqual({ elbow_width: 500, forearm: 260 });
    expect(bodyOf({ ...on, preset: "average" })).toEqual(BODY);
    expect(bodyOf(on).elbowWidth).toBeGreaterThan(BODY.elbowWidth);
  });
});

describe("thumb", () => {
  const t = new Thumb(AVERAGE);
  const rest = handLayout(AVERAGE).thumbRest;

  it("rests on its rest point and reaches nearby targets within its limits", () => {
    const r = t.solve(rest);
    expect(r.miss).toBeLessThan(0.5);
    r.q.forEach((v, j) => expect(Math.abs(v - t.neutral[j])).toBeLessThan(1e-3));
    const near = t.solve([rest[0] + 12, rest[1], rest[2] + 6]);
    expect(near.miss).toBeLessThan(0.5);
    expect(dist(t.joints(near.q)[3], [rest[0] + 12, rest[1], rest[2] + 6])).toBeLessThan(0.5);
  });

  it("falls short of far targets, keeping its segment lengths", () => {
    const far = t.solve([rest[0] - 60, rest[1], rest[2] - 60]);
    expect(far.miss).toBeGreaterThan(20);
    const j = t.joints(far.q);
    AVERAGE.thumb.forEach((l, i) => expect(dist(j[i], j[i + 1])).toBeCloseTo(l, 6));
  });

  it("toLocal inverts toWorld for both hands", () => {
    const pl = { wrist: [3, 1, 2] as V3, yaw: 0.3, roll: -0.2 };
    for (const side of ["L", "R"] as Side[]) {
      const p: V3 = [12, -7, -80];
      const back = toLocal(side, pl, U, toWorld(side, pl, U, p));
      expect(dist(back, p)).toBeLessThan(1e-9);
    }
  });
});
