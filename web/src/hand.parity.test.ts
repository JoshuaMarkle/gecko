// The TypeScript hand model must match its Rust port (`gecko_core::hand`), which the scorer uses:
// the Hand tab should show the hand the scorer measures. The fixture is written by Rust
// (`GECKO_BLESS=1 cargo test --release -p gecko-core hand_parity`) and checked by both sides.

import { describe, expect, it } from "vitest";
import fixture from "./fixtures/hand-parity.json";
import { fingerChain, FOUR, handLayout, naturalTip, placeHand, Thumb, type FingerName, type HandDims, type V3 } from "./hand";

const TOL = 1e-6;

function near(actual: number[] | number, expected: number[] | number, label: string) {
  const a = Array.isArray(actual) ? actual : [actual];
  const e = Array.isArray(expected) ? expected : [expected];
  expect(a.length, label).toBe(e.length);
  a.forEach((x, i) => expect(Math.abs(x - e[i]), `${label}[${i}]: ${x} vs Rust ${e[i]}`).toBeLessThanOrEqual(TOL * (1 + Math.abs(e[i]))));
}

type Seg = [number, number, number];

for (const c of fixture.cases) {
  const d: HandDims = {
    thumb: c.dims.thumb as Seg,
    index: c.dims.index as Seg,
    middle: c.dims.middle as Seg,
    ring: c.dims.ring as Seg,
    pinky: c.dims.pinky as Seg,
    palmWidth: c.dims.palm_width,
    palmLength: c.dims.palm_length,
    span: c.dims.span,
    reach: c.dims.reach,
  };

  describe(`hand parity with Rust: ${c.name}`, () => {
    it("skeleton", () => {
      const lay = handLayout(d);
      lay.knuckles.forEach((k, i) => near(k, c.knuckles[i], `knuckle ${i}`));
      near(lay.thumb, c.thumb_base, "thumb base");
      near(lay.thumbRest, c.thumb_rest, "thumb rest");
    });

    it("natural curl", () => {
      FOUR.forEach((f, i) => near(naturalTip(d[f]), c.natural_tip[i], `${f} natural tip`));
    });

    it("finger IK", () => {
      const l = d.middle;
      for (const ch of c.middle_chains) {
        // Rust returns joint flexions; rebuild the joints in the finger plane (u forward = −z, v = y).
        const abs = [ch.angles[0], ch.angles[0] + ch.angles[1], ch.angles[0] + ch.angles[1] + ch.angles[2]];
        const want: V3[] = [[0, 0, 0]];
        let u = 0, v = 0;
        for (let i = 0; i < 3; i++) {
          u += l[i] * Math.cos(abs[i]);
          v -= l[i] * Math.sin(abs[i]);
          want.push([0, v, -u]);
        }
        const got = fingerChain([0, 0, 0], [0, ch.tv, -ch.tu], l);
        got.forEach((p, i) => near(p, want[i], `chain (${ch.tu}, ${ch.tv}) joint ${i}`));
      }
    });

    it("hand placement", () => {
      for (const p of c.placements) {
        const home = { thumb: [0, 0, 0] } as Record<FingerName, V3>;
        FOUR.forEach((f, i) => (home[f] = p.targets[i] as V3));
        const got = placeHand("R", home, d, 1);
        near(got.wrist, p.wrist, "wrist");
        near(got.yaw, p.yaw, "yaw");
        near(got.roll ?? 0, p.roll, "roll");
      }
    });

    it("thumb IK", () => {
      const t = new Thumb(d);
      near(t.neutral, c.thumb.neutral, "thumb neutral");
      t.joints(t.neutral).forEach((j, i) => near(j, c.thumb.neutral_joints[i], `thumb joint ${i}`));
      for (const r of c.thumb.reach) {
        const s = t.solve(r.target as V3);
        near(s.q, r.q, `thumb q → ${r.target}`);
        near(s.miss, r.miss, `thumb miss → ${r.target}`);
      }
    });
  });
}
