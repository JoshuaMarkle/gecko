//! Hand model (#33, #40, #52, #63): a skeleton from measurements, each hand placed over its
//! home keys, planar finger IK and a 4-DoF thumb within joint limits, and per-key costs for the
//! scorer: G01 reach (joint deviation from rest, sideways splay, and how far the fingertip falls
//! short), G05 press alignment (keycap normal vs the fingertip's pressing direction) and G03
//! wrist posture (the resting hand against the forearm, plus sideways reaches).
//!
//! Port of `web/src/hand.ts` with the same constants, so the Hand tab shows the hand the scorer
//! measures; the `parity` test below and `web/src/hand.parity.test.ts` keep the two in step. Lengths in mm. Coordinates here follow the web viewer: x right, y up, z toward the
//! user (board space is x right, y toward the user, z up; [`target`] converts). The right hand is
//! modelled; the left hand is its exact mirror. See `context/hand-model.md`.

use serde::{Deserialize, Serialize};

use crate::geometry::flat2d::{cross, dot, norm, sub};
use crate::geometry::pose::{self, add, Frame};
use crate::geometry::{Finger, Geometry, Hand};
use crate::{Error, Result};

/// Millimetres per key unit.
pub const MM_PER_U: f64 = 19.05;

pub type V3 = [f64; 3];

/// Segment lengths base → tip (thumb: metacarpal, proximal, distal; fingers: proximal, middle,
/// distal) and palm measurements, mm. Same fields as the web's `HandDims`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct HandDims {
    pub thumb: [f32; 3],
    pub index: [f32; 3],
    pub middle: [f32; 3],
    pub ring: [f32; 3],
    pub pinky: [f32; 3],
    /// Across the index–pinky knuckles.
    pub palm_width: f32,
    /// Wrist crease to the middle knuckle.
    pub palm_length: f32,
    /// Thumb tip to pinky tip, spread flat. Scales B14's relaxed finger spread.
    pub span: f32,
    /// Comfortable fingertip travel from the home key (shown in the web; not used in costs).
    pub reach: f32,
}

/// Average adult hand (pooled sexes), rounded from published phalanx and palm measurements.
pub const AVERAGE: HandDims = HandDims {
    thumb: [46.0, 32.0, 23.0],
    index: [40.0, 23.0, 18.0],
    middle: [45.0, 27.0, 19.0],
    ring: [42.0, 26.0, 19.0],
    pinky: [33.0, 18.0, 17.0],
    palm_width: 84.0,
    palm_length: 105.0,
    span: 200.0,
    reach: 40.0,
};

/// Preset sizes: the average scaled to about the 8th / 92nd percentile of hand length.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HandSize {
    Small,
    #[default]
    Average,
    Large,
}

impl HandSize {
    pub fn scale(self) -> f32 {
        match self {
            HandSize::Small => 0.92,
            HandSize::Average => 1.0,
            HandSize::Large => 1.08,
        }
    }
}

/// Profile `[hand]`: a preset size, or explicit measurements (which win), and optionally the
/// arms G03 measures the wrist against.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct HandSpec {
    #[serde(default)]
    pub size: HandSize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dims: Option<HandDims>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<Body>,
}

/// Arms for G03, mm: elbow-to-elbow distance while typing and forearm length (elbow to wrist
/// crease). The elbows sit at keyboard height, so the forearms are level.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Body {
    pub elbow_width: f32,
    pub forearm: f32,
}

/// Average arms. The elbow width is set so a flat board gives the ≈ 12° mean ulnar deviation
/// measured on conventional keyboards (`research/sources/split-keyboard-ergonomics.md`; ANSI
/// 11.9°/12.8°, ortho 12.3°; ±10 mm moves it ≈ 1.3°); forearm ≈ 0.15 × stature.
pub const BODY: Body = Body { elbow_width: 465.0, forearm: 255.0 };

impl Body {
    /// [`BODY`] scaled with hand length.
    pub fn for_hand(d: &HandDims) -> Body {
        let s = d.length() / AVERAGE.length();
        Body { elbow_width: BODY.elbow_width * s, forearm: BODY.forearm * s }
    }
}

impl HandSpec {
    pub fn dims(&self) -> HandDims {
        self.dims.clone().unwrap_or_else(|| AVERAGE.scaled(self.size.scale()))
    }

    pub fn body(&self) -> Body {
        self.body.unwrap_or_else(|| Body::for_hand(&self.dims()))
    }

    pub fn validate(&self) -> Result<()> {
        let d = self.dims();
        let all = d.thumb.iter().chain(&d.index).chain(&d.middle).chain(&d.ring).chain(&d.pinky);
        let ok = all.chain([&d.palm_width, &d.palm_length, &d.span, &d.reach]).all(|v| v.is_finite() && (1.0..=500.0).contains(v));
        if !ok {
            return Err(Error::invalid("profile", "hand: every measurement must be in [1, 500] mm"));
        }
        let b = self.body();
        if !(100.0..=1500.0).contains(&b.elbow_width) || !(100.0..=600.0).contains(&b.forearm) {
            return Err(Error::invalid("profile", "hand.body: elbow_width in [100, 1500] mm, forearm in [100, 600] mm"));
        }
        Ok(())
    }
}

impl HandDims {
    pub fn scaled(&self, s: f32) -> HandDims {
        let seg = |x: [f32; 3]| x.map(|v| v * s);
        HandDims {
            thumb: seg(self.thumb),
            index: seg(self.index),
            middle: seg(self.middle),
            ring: seg(self.ring),
            pinky: seg(self.pinky),
            palm_width: self.palm_width * s,
            palm_length: self.palm_length * s,
            span: self.span * s,
            reach: self.reach * s,
        }
    }

    /// Segments of a finger by `Finger::ord` (pinky 0 … index 3, thumb 4).
    pub fn segments(&self, ord: u8) -> [f64; 3] {
        let s = match ord {
            0 => self.pinky,
            1 => self.ring,
            2 => self.middle,
            3 => self.index,
            _ => self.thumb,
        };
        s.map(|v| v as f64)
    }

    /// Wrist crease to middle fingertip.
    pub fn length(&self) -> f32 {
        self.palm_length + self.middle.iter().sum::<f32>()
    }
}

// ---------- skeleton (right hand: x toward the pinky, y up, z toward the wrist) ----------

/// Knuckle arch: drop below the middle knuckle (y) and setback toward the wrist (z), index → pinky,
/// as fractions of palm length.
const ARCH_Y: [f64; 4] = [-0.03, 0.0, -0.025, -0.07];
const ARCH_Z: [f64; 4] = [0.02, 0.0, 0.04, 0.13];

/// Knuckles (MCP joints) index, middle, ring, pinky; thumb base (CMC); where the thumb tip
/// rests in the typing pose (below and behind the index fingertip, about key height).
struct Skeleton {
    knuckles: [V3; 4],
    thumb: V3,
    thumb_rest: V3,
}

fn skeleton(d: &HandDims) -> Skeleton {
    let kw = 0.76 * d.palm_width as f64;
    let pl = d.palm_length as f64;
    let pw = d.palm_width as f64;
    let knuckles: [V3; 4] = std::array::from_fn(|i| [kw * (i as f64 / 3.0 - 0.5), ARCH_Y[i] * pl, -pl * (1.0 - ARCH_Z[i])]);
    let thumb_len: f64 = d.thumb.iter().map(|&v| v as f64).sum();
    Skeleton {
        knuckles,
        thumb: [-0.42 * pw, -0.12 * pl, -0.24 * pl],
        thumb_rest: [knuckles[0][0] - 0.1 * pw, -0.35 * pl, -(0.72 * pl + 0.4 * thumb_len)],
    }
}

/// Natural typing curl: PIP flexed 54°, DIP 0.7 of that (tendon coupling), proximal phalanx
/// pitched 11° down (MCP flexion `-PITCH`).
const CURL: f64 = 0.95;
const COUPLE: f64 = 0.7;
const PITCH: f64 = -0.2;

/// Joint limits (radians, flexion positive): MCP from 20° hyperextension, PIP, DIP.
const MCP: [f64; 2] = [-0.35, 1.5];
const PIP: [f64; 2] = [-0.05, 1.9];
const DIP: [f64; 2] = [0.0, 1.4];

/// Tip of a planar chain with joint flexions `a`, `b`, `g` (2D: u forward, v up).
fn tip2(l: &[f64; 3], a: f64, b: f64, g: f64) -> [f64; 2] {
    let abs = [a, a + b, a + b + g];
    let (mut u, mut v) = (0.0, 0.0);
    for i in 0..3 {
        u += l[i] * abs[i].cos();
        v -= l[i] * abs[i].sin();
    }
    [u, v]
}

/// Fingertip offset from the knuckle in the natural curl: [forward, drop] (same units as `l`).
pub fn natural_tip(l: &[f64; 3]) -> [f64; 2] {
    let t = tip2(l, -PITCH, CURL, COUPLE * CURL);
    [t[0], -t[1]]
}

/// A solved finger: joint flexions (MCP, PIP, DIP; radians) and how far the tip misses (mm).
#[derive(Clone, Copy, Debug)]
pub struct Chain {
    pub angles: [f64; 3],
    pub miss: f64,
}

/// Planar IK toward a target `tu` ahead of and `tv` above the base, in the finger's vertical
/// plane (the web's `fingerChain`). For each MCP flexion the last two segments close the gap
/// when they can, every joint clamped to its limits; the pose kept misses least, then sits
/// closest to DIP ≈ `ratio`·PIP with the MCP near `rest`. Out of reach: the finger points
/// straight at the target.
pub fn solve(tu: f64, tv: f64, l: &[f64; 3], ratio: f64, rest: f64) -> Chain {
    let total = l[0] + l[1] + l[2];
    let dist = tu.hypot(tv);
    if dist >= total {
        return Chain { angles: [(-tv).atan2(tu), 0.0, 0.0], miss: dist - total };
    }
    let clamp = |x: f64, r: [f64; 2]| x.clamp(r[0], r[1]);
    let pose = |a: f64| -> (f64, Chain) {
        let du = tu - l[0] * a.cos();
        let dv = tv + l[0] * a.sin();
        let d = du.hypot(dv);
        let cg = (d * d - l[1] * l[1] - l[2] * l[2]) / (2.0 * l[1] * l[2]);
        let g = clamp(cg.clamp(-1.0, 1.0).acos(), DIP);
        let b = clamp((-dv).atan2(du) - (l[2] * g.sin()).atan2(l[1] + l[2] * g.cos()) - a, PIP);
        let t = tip2(l, a, b, g);
        let miss = (t[0] - tu).hypot(t[1] - tv);
        let m = miss / total;
        (1e4 * m * m + (g - ratio * b).powi(2) + 0.15 * (a - rest).powi(2), Chain { angles: [a, b, g], miss })
    };
    let n = 64;
    let step = (MCP[1] - MCP[0]) / n as f64;
    let mut best = pose(MCP[0]);
    for i in 1..=n {
        let s = pose(MCP[0] + i as f64 * step);
        if s.0 < best.0 {
            best = s;
        }
    }
    // Refine around the best grid point (golden-section search).
    let (mut lo, mut hi) = ((best.1.angles[0] - step).max(MCP[0]), (best.1.angles[0] + step).min(MCP[1]));
    for _ in 0..24 {
        let (m1, m2) = (hi - 0.618 * (hi - lo), lo + 0.618 * (hi - lo));
        if pose(m1).0 < pose(m2).0 {
            hi = m2;
        } else {
            lo = m1;
        }
    }
    let r = pose((lo + hi) / 2.0);
    if r.0 < best.0 {
        best = r;
    }
    best.1
}

// ---------- thumb ----------

/// A 4-DoF thumb (right-hand frame): CMC flexion (the tip sweeps across the palm toward the
/// fingers) and palmar abduction (the tip moves away from the palm, down when palm-down) about
/// fixed axes at the CMC, then MCP and IP flexion curling the thumb toward its pad. Angles in
/// radians, `[cmc flexion, cmc abduction, mcp, ip]`; the CMC angles are relative to the rest
/// pose, MCP and IP absolute.
pub struct Thumb {
    base: V3,
    /// Metacarpal direction, pad direction and the two CMC axes at zero CMC angles.
    d0: V3,
    pad0: V3,
    flex: V3,
    abd: V3,
    l: [f64; 3],
    /// Rest pose: the tip on the rest point, MCP and IP near their light curl.
    pub neutral: [f64; 4],
}

/// Rest flexion of the thumb's MCP and IP joints (≈ 17°).
const THUMB_CURL: f64 = 0.3;
/// Thumb joint limits: CMC flexion and abduction about rest (≈ ±34°, −29°…+40°: active ROM
/// CMC flexion 49°), MCP −11°…60° and IP −17°…80° absolute (`research/sources/hand-models.md`).
const THUMB_LIMIT: [[f64; 2]; 4] = [[-0.6, 0.6], [-0.5, 0.7], [-0.2, 1.05], [-0.3, 1.4]];
/// Deviation from rest that costs 1 per joint (radians): CMC 26°, MCP 34°, IP 46°.
const THUMB_SCALE: [f64; 4] = [0.45, 0.45, 0.6, 0.8];

/// `v` rotated by `a` radians about unit `axis` (Rodrigues).
fn rotate(axis: V3, a: f64, v: V3) -> V3 {
    let (s, c) = a.sin_cos();
    let k = cross(axis, v);
    let d = dot(axis, v) * (1.0 - c);
    [v[0] * c + k[0] * s + axis[0] * d, v[1] * c + k[1] * s + axis[1] * d, v[2] * c + k[2] * s + axis[2] * d]
}

fn scaled(v: V3, s: f64) -> V3 {
    [v[0] * s, v[1] * s, v[2] * s]
}

/// Component of `v` perpendicular to unit `d`, normalised.
fn perp(v: V3, d: V3) -> V3 {
    norm(sub(v, scaled(d, dot(v, d))))
}

impl Thumb {
    /// The thumb of a placed hand: base at the CMC and rest point in world coordinates.
    fn new(d: &HandDims, pl: &Placement) -> Thumb {
        let sk = skeleton(d);
        let (base, rest) = (pl.world(sk.thumb), pl.world(sk.thumb_rest));
        let d0 = norm(sub(rest, base));
        // Axes from the hand's own directions: across the palm (+x), down (−y), the pad facing
        // mostly toward the fingers and a little down (the thumb is pronated ~80°).
        let (across, down) = (pl.dir([1.0, 0.0, 0.0]), pl.dir([0.0, -1.0, 0.0]));
        let pad0 = perp(add(scaled(across, 0.8), scaled(down, 0.6)), d0);
        let flex = norm(cross(d0, across));
        let abd = norm(cross(d0, down));
        let mut t = Thumb { base, d0, pad0, flex, abd, l: d.segments(4), neutral: [0.0, 0.0, THUMB_CURL, THUMB_CURL] };
        // Pose the thumb from its light curl so the tip lands on the rest point (the CMC aims it,
        // MCP/IP set its length); re-centring each pass removes the pull toward the last guess.
        for _ in 0..4 {
            let start = t.neutral;
            t.neutral = t.solve_with(rest, &start, [true; 4]).0;
        }
        t
    }

    /// Joint positions: CMC, MCP, IP, tip.
    pub fn joints(&self, q: &[f64; 4]) -> [V3; 4] {
        let r = |v: V3| rotate(self.abd, q[1], rotate(self.flex, q[0], v));
        let (m, pad) = (r(self.d0), r(self.pad0));
        let axis = norm(cross(m, pad));
        let p = rotate(axis, q[2], m);
        let dist = rotate(axis, q[2] + q[3], m);
        let j1 = add(self.base, scaled(m, self.l[0]));
        let j2 = add(j1, scaled(p, self.l[1]));
        [self.base, j1, j2, add(j2, scaled(dist, self.l[2]))]
    }

    fn limits(&self, j: usize) -> [f64; 2] {
        let off = if j < 2 { self.neutral[j] } else { 0.0 };
        [THUMB_LIMIT[j][0] + off, THUMB_LIMIT[j][1] + off]
    }

    /// IK toward `target`: damped least squares on the tip error (weighted so a miss dominates)
    /// plus each joint's deviation from rest, within the joint limits. Returns angles and miss.
    pub fn solve(&self, target: V3) -> ([f64; 4], f64) {
        self.solve_with(target, &self.neutral, [true; 4])
    }

    fn solve_with(&self, target: V3, start: &[f64; 4], free: [bool; 4]) -> ([f64; 4], f64) {
        let total: f64 = self.l.iter().sum();
        let w = 100.0 / total;
        let resid = |q: &[f64; 4]| -> [f64; 7] {
            let e = sub(self.joints(q)[3], target);
            let mut r = [w * e[0], w * e[1], w * e[2], 0.0, 0.0, 0.0, 0.0];
            for j in 0..4 {
                r[3 + j] = if free[j] { (q[j] - self.neutral[j]) / THUMB_SCALE[j] } else { 0.0 };
            }
            r
        };
        let energy = |r: &[f64; 7]| r.iter().map(|x| x * x).sum::<f64>();
        let mut q = *start;
        let mut r = resid(&q);
        let mut lambda = 1e-2;
        for _ in 0..60 {
            // Numeric Jacobian, 7 × 4.
            let mut jac = [[0.0; 4]; 7];
            for j in (0..4).filter(|&j| free[j]) {
                let mut qh = q;
                qh[j] += 1e-6;
                let rh = resid(&qh);
                for i in 0..7 {
                    jac[i][j] = (rh[i] - r[i]) / 1e-6;
                }
            }
            // (JᵀJ + λ·diag) δ = −Jᵀr
            let mut a = [[0.0; 4]; 4];
            let mut b = [0.0; 4];
            for j in 0..4 {
                for k in 0..4 {
                    a[j][k] = (0..7).map(|i| jac[i][j] * jac[i][k]).sum();
                }
                b[j] = -(0..7).map(|i| jac[i][j] * r[i]).sum::<f64>();
                a[j][j] += lambda * (1.0 + a[j][j]);
            }
            let delta = solve4(a, b);
            let mut next = q;
            for j in (0..4).filter(|&j| free[j]) {
                let lim = self.limits(j);
                next[j] = (q[j] + delta[j]).clamp(lim[0], lim[1]);
            }
            let rn = resid(&next);
            if energy(&rn) < energy(&r) {
                let gain = energy(&r) - energy(&rn);
                q = next;
                r = rn;
                lambda = (lambda * 0.3).max(1e-7);
                if gain < 1e-12 {
                    break;
                }
            } else {
                lambda *= 10.0;
                if lambda > 1e8 {
                    break;
                }
            }
        }
        let miss = sub(self.joints(&q)[3], target);
        (q, dot(miss, miss).sqrt())
    }

    /// The thumb's press misalignment with a key of inward normal `into` (radians): the CMC's two
    /// axes push the tip anywhere across the CMC→tip line, so only a push along that line (tip
    /// first, or pulling back) is misaligned: the angle between `into` and the plane across it.
    pub fn press_angle(&self, q: &[f64; 4], into: V3) -> f64 {
        let r = norm(sub(self.joints(q)[3], self.base));
        dot(r, into).abs().min(1.0).asin()
    }
}

/// Solve the 4 × 4 system `a x = b` (Gaussian elimination, partial pivoting); zeros if singular.
fn solve4(mut a: [[f64; 4]; 4], mut b: [f64; 4]) -> [f64; 4] {
    for c in 0..4 {
        let p = (c..4).max_by(|&i, &j| a[i][c].abs().total_cmp(&a[j][c].abs())).unwrap_or(c);
        if a[p][c].abs() < 1e-14 {
            continue;
        }
        a.swap(c, p);
        b.swap(c, p);
        for r in c + 1..4 {
            let f = a[r][c] / a[c][c];
            for k in c..4 {
                a[r][k] -= f * a[c][k];
            }
            b[r] -= f * b[c];
        }
    }
    let mut x = [0.0; 4];
    for c in (0..4).rev() {
        if a[c][c].abs() < 1e-14 {
            continue;
        }
        x[c] = (b[c] - (c + 1..4).map(|k| a[c][k] * x[k]).sum::<f64>()) / a[c][c];
    }
    x
}

// ---------- placement ----------

/// Where a hand sits (right-hand frame mm): wrist crease centre, yaw about the vertical (+ fingers
/// toward the body's midline) and roll about the forearm (+ thumb side up).
#[derive(Clone, Copy, Debug)]
struct Placement {
    wrist: V3,
    yaw: f64,
    roll: f64,
}

/// Natural inward yaw of the right hand (~10°).
const YAW0: f64 = 0.17;

impl Placement {
    /// Hand-local (right-hand frame) → right-hand world frame (mirrored world for the left hand).
    fn world(&self, p: V3) -> V3 {
        let d = self.dir(p);
        [self.wrist[0] + d[0], self.wrist[1] + d[1], self.wrist[2] + d[2]]
    }

    /// A hand-local direction in the world frame (roll, then yaw).
    fn dir(&self, p: V3) -> V3 {
        let (sr, cr) = self.roll.sin_cos();
        let (lx, ly) = (p[0] * cr + p[1] * sr, p[1] * cr - p[0] * sr);
        let (s, c) = self.yaw.sin_cos();
        [lx * c + p[2] * s, ly, -lx * s + p[2] * c]
    }
}

/// Place a right hand (targets already mirrored for the left) over its index, middle, ring,
/// pinky home targets: wrist x, z, yaw, height and roll minimise each finger's deviation from
/// its natural curl (forward and height, plus a lighter sideways term), with weak pulls toward
/// the natural yaw and no roll. Pattern search, as in the web's `placeHand`.
fn place(t: &[V3; 4], d: &HandDims) -> Placement {
    let sk = skeleton(d);
    let nat: [[f64; 2]; 4] = std::array::from_fn(|i| natural_tip(&d.segments(3 - i as u8)));
    let cost = |q: &[f64; 5]| {
        let [wx, wz, yaw, wy, roll] = *q;
        let (s, c) = yaw.sin_cos();
        let (sr, cr) = roll.sin_cos();
        let mut e = 300.0 * (yaw - YAW0).powi(2) + 300.0 * roll * roll;
        for i in 0..4 {
            let k = sk.knuckles[i];
            let (lx, ly) = (k[0] * cr + k[1] * sr, k[1] * cr - k[0] * sr);
            let vx = t[i][0] - (wx + lx * c + k[2] * s);
            let vz = t[i][2] - (wz - lx * s + k[2] * c);
            let (fwd, lat, drop) = (-vx * s - vz * c, vx * c - vz * s, wy + ly - t[i][1]);
            e += (fwd - nat[i][0]).powi(2) + 0.3 * lat * lat + (drop - nat[i][1]).powi(2);
        }
        e
    };
    let (s0, c0) = YAW0.sin_cos();
    let k1 = sk.knuckles[1];
    let (kx, kz) = (t[1][0] + s0 * nat[1][0], t[1][2] + c0 * nat[1][0]);
    let mut p = [kx - (k1[0] * c0 + k1[2] * s0), kz - (-k1[0] * s0 + k1[2] * c0), YAW0, t[1][1] + nat[1][1] - k1[1], 0.0];
    let mut best = cost(&p);
    let mut step = [8.0, 8.0, 0.06, 8.0, 0.06];
    for _ in 0..200 {
        let mut moved = false;
        for k in 0..5 {
            for sg in [1.0, -1.0] {
                let mut q = p;
                q[k] += sg * step[k];
                let c = cost(&q);
                if c < best - 1e-9 {
                    best = c;
                    p = q;
                    moved = true;
                }
            }
        }
        if !moved {
            step.iter_mut().for_each(|s| *s /= 2.0);
            if step[0] < 0.01 {
                break;
            }
        }
    }
    Placement { wrist: [p[0], p[3], p[1]], yaw: p[2], roll: p[4] }
}

// ---------- wrist posture ----------

/// Wrist and forearm angles in degrees: ulnar (+) / radial (−) deviation, extension (+) / flexion
/// (−), and forearm pronation (90 = palm flat down, 0 = thumb up).
#[derive(Clone, Copy, Debug, Default, Serialize, PartialEq)]
pub struct Posture {
    pub deviation: f32,
    pub extension: f32,
    pub pronation: f32,
}

impl Posture {
    /// G03 cost: squared deviation from neutral over a comfortable range per axis: ulnar 15°,
    /// radial 10°, extension 20°, flexion 25°, pronation ±45° about 30° (a forearm turned a
    /// third of the way from thumb-up to palm-down rests easiest).
    pub fn cost(&self) -> f32 {
        let dev = self.deviation / if self.deviation >= 0.0 { 15.0 } else { 10.0 };
        let ext = self.extension / if self.extension >= 0.0 { 20.0 } else { 25.0 };
        let pro = (self.pronation - 30.0) / 45.0;
        dev * dev + ext * ext + pro * pro
    }
}

/// The resting posture of a placed hand, from its axes and wrist in right-hand board space (mm;
/// left hands mirrored about the body's midline `mid`) and the elbow `body.elbow_width / 2`
/// outside the midline, level with the wrist, `body.forearm` from it. A split half is turned and
/// moved to suit the arm, so its forearm lines up with the hand (no resting deviation).
fn posture(fwd: V3, up: V3, wrist: V3, mid: f64, body: &Body, split: bool) -> Posture {
    let arm = if split {
        norm([fwd[0], fwd[1], 0.0])
    } else {
        let f = body.forearm as f64;
        let dx = (mid + body.elbow_width as f64 / 2.0 - wrist[0]).clamp(-0.95 * f, 0.95 * f);
        let elbow = [wrist[0] + dx, wrist[1] + (f * f - dx * dx).sqrt(), wrist[2]];
        norm(sub(wrist, elbow))
    };
    // Deviation: the forearm's direction vs the hand's within the palm's plane, + toward the pinky.
    let a = perp(arm, up);
    let dev = dot(cross(a, fwd), up).atan2(dot(a, fwd));
    let ext = dot(fwd, [0.0, 0.0, 1.0]).clamp(-1.0, 1.0).asin() - dot(arm, [0.0, 0.0, 1.0]).clamp(-1.0, 1.0).asin();
    // Pronation: the back of the hand turned from straight up about the forearm.
    let (n, z) = (perp(up, arm), perp([0.0, 0.0, 1.0], arm));
    let roll = dot(cross(z, n), arm).atan2(dot(z, n));
    Posture { deviation: dev.to_degrees() as f32, extension: ext.to_degrees() as f32, pronation: (90.0 + roll.to_degrees()) as f32 }
}

/// Whether the board's halves are separate (≥ 1 u between the hands' finger keys): a split half's
/// drawn position is illustrative, so G03 lets the user place it (`posture`).
pub fn split(g: &Geometry) -> bool {
    let (mut l, mut r) = (f32::MIN, f32::MAX);
    for k in g.key.iter().filter(|k| k.phys.is_none() && !k.finger.is_thumb()) {
        match k.hand() {
            Hand::L => l = l.max(k.pos[0] + k.size[0]),
            Hand::R => r = r.min(k.pos[0]),
        }
    }
    r - l >= 1.0
}

// ---------- per-key costs ----------

/// Comfortable MCP abduction (half of the total arc, radians) by `Finger::ord`: pinky 25°,
/// ring 21°, middle 21° (the 81° middle arc is flagged "verify", so it gets the ring's), index
/// 21°. `research/sources/hand-models.md`.
const ABDUCT: [f64; 4] = [0.436, 0.367, 0.367, 0.367];
/// Joint deviation that costs 1 (radians, ~29°).
const JOINT: f64 = 0.5;
/// Fingertip shortfall that costs 1 (mm), on top of a linear term: moving the hand.
const MISS: f64 = 10.0;
/// Shortfalls under this (mm) are IK rounding, not a miss.
const MISS_TOL: f64 = 0.5;

/// Per-key hand costs for one geometry and hand, aligned with `Geometry::key`.
#[derive(Clone, Debug, Default, Serialize)]
pub struct HandCosts {
    /// G01 reach cost per key (layer copies: their physical key).
    pub reach: Vec<f32>,
    /// G05 press misalignment per key: (angle / 45°)² between the fingertip's press direction
    /// (fingers: the distal phalanx, within the finger's plane; thumbs: palmar abduction) and the
    /// key's normal.
    pub align: Vec<f32>,
    /// G03 wrist posture cost per key: the hand's resting posture, with the extra ulnar or
    /// radial deviation of reaching sideways for a key the planted hand falls short of.
    pub wrist: Vec<f32>,
    /// Fingertip shortfall per key (mm; 0 when reachable).
    pub miss: Vec<f32>,
    /// Hands that could be placed (both home rows present).
    pub placed: [bool; 2],
    /// Resting posture per hand (L, R).
    pub posture: [Posture; 2],
}

/// A key's fingertip target (keycap top centre) and press normal in viewer axes, mm. On 3D
/// boards both are taken in the key's hand frame (`Geometry::hand_frame`), as the geometric
/// metrics are, so a rigid tent changes no finger cost; G03 turns the hand back into board space.
pub fn target(g: &Geometry, frame: Option<&Frame>, i: usize) -> (V3, V3) {
    let k = &g.key[g.phys(i)];
    let (mut c, mut n) = (pose::contact(k), pose::normal(k));
    if let Some(f) = frame {
        (c, n) = (f.point(c), f.dir(n));
    }
    ([c[0] * MM_PER_U, c[2] * MM_PER_U, c[1] * MM_PER_U], [n[0], n[2], n[1]])
}

/// The home key the hand rests on for a finger: a home key outside the stretch columns, then
/// any home key, then the finger's first key (the web's `buildRigs`). Base-layer keys only.
fn home_key(g: &Geometry, f: Finger) -> Option<usize> {
    let ks: Vec<usize> = (0..g.len()).filter(|&i| g.key[i].phys.is_none() && g.key[i].finger == f).collect();
    ks.iter().find(|&&i| g.key[i].home && !g.key[i].stretch).or_else(|| ks.iter().find(|&&i| g.key[i].home)).or(ks.first()).copied()
}

/// [`costs_with`] with the arms scaled to the hand ([`Body::for_hand`]).
pub fn costs(g: &Geometry, d: &HandDims) -> HandCosts {
    costs_with(g, d, &Body::for_hand(d))
}

/// Solve every key of the geometry for hand `d` and arms `body`. A hand without all four
/// finger keys is not placed and its keys cost 0.
pub fn costs_with(g: &Geometry, d: &HandDims, body: &Body) -> HandCosts {
    let n = g.len();
    let mut out = HandCosts {
        reach: vec![0.0; n],
        align: vec![0.0; n],
        wrist: vec![0.0; n],
        miss: vec![0.0; n],
        placed: [false; 2],
        posture: [Posture::default(); 2],
    };
    let sk = skeleton(d);
    let split = split(g);
    let base: Vec<usize> = (0..n).filter(|&i| g.key[i].phys.is_none()).collect();
    // The body's midline: the middle of the board (mm).
    let mid = base.iter().map(|&i| pose::contact(&g.key[i])[0]).sum::<f64>() / base.len().max(1) as f64 * MM_PER_U;
    for (h, hand) in [Hand::L, Hand::R].into_iter().enumerate() {
        // Mirror the left hand into the right-hand frame.
        let m = if hand == Hand::L { -1.0 } else { 1.0 };
        let local = |p: V3| [m * p[0], p[1], p[2]];
        let fingers = if hand == Hand::L {
            [Finger::LI, Finger::LM, Finger::LR, Finger::LP]
        } else {
            [Finger::RI, Finger::RM, Finger::RR, Finger::RP]
        };
        let homes: Vec<usize> = fingers.iter().filter_map(|&f| home_key(g, f)).collect();
        if homes.len() < 4 {
            continue;
        }
        out.placed[h] = true;
        let frame = g.hand_frame(hand);
        let target = |i| target(g, frame.as_ref(), i);
        let t: [V3; 4] = std::array::from_fn(|i| local(target(homes[i]).0));
        let pl = place(&t, d);
        let knuckles: [V3; 4] = std::array::from_fn(|i| pl.world(sk.knuckles[i]));
        let thumb = Thumb::new(d, &pl);

        // Back to (mirrored) board space for the wrist: viewer (x, up, toward user) in the
        // hand's frame → board (x, toward user, up) → mirror the left hand about the midline.
        let to_board_dir = |v: V3| {
            let f = [m * v[0], v[2], v[1]];
            let b = frame.as_ref().map_or(f, |fr| fr.undir(f));
            [m * b[0], b[1], b[2]]
        };
        let w = pl.wrist;
        let wf = [m * w[0] / MM_PER_U, w[2] / MM_PER_U, w[1] / MM_PER_U];
        let wb = frame.as_ref().map_or(wf, |fr| fr.unpoint(wf));
        let wrist = [mid + m * (wb[0] * MM_PER_U - mid), wb[1] * MM_PER_U, wb[2] * MM_PER_U];
        let (fwd, up) = (to_board_dir(pl.dir([0.0, 0.0, -1.0])), to_board_dir(pl.dir([0.0, 1.0, 0.0])));
        let rest = posture(fwd, up, wrist, mid, body, split);
        out.posture[h] = rest;
        // The hand's sideways axis (toward the pinky) in the hand's frame, for sideways misses.
        let side = pl.dir([1.0, 0.0, 0.0]);

        for i in base.iter().copied().filter(|&i| g.key[i].hand() == hand) {
            let f = g.key[i].finger;
            let (p, nrm) = target(i);
            let (p, nrm) = (local(p), local(nrm));
            let (reach, align, shortfall, gap) = if f.is_thumb() {
                let (q, miss) = thumb.solve(p);
                let joints = (0..4).map(|j| ((q[j] - thumb.neutral[j]) / THUMB_SCALE[j]).powi(2)).sum::<f64>() / 4.0;
                let shortfall = if miss < MISS_TOL { 0.0 } else { miss };
                let mm = shortfall / MISS;
                let angle = thumb.press_angle(&q, scaled(nrm, -1.0));
                let gap = sub(p, thumb.joints(&q)[3]);
                (joints + 2.0 * mm + mm * mm, (angle / std::f64::consts::FRAC_PI_4).powi(2), shortfall, gap)
            } else {
                let ord = f.ord() as usize;
                let l = d.segments(ord as u8);
                let fi = 3 - ord;
                let (k, home) = (knuckles[fi], t[fi]);
                let flat = |q: V3| [q[0] - k[0], q[2] - k[2]];
                let (v, v0) = (flat(p), flat(home));
                let tu = v[0].hypot(v[1]);
                let chain = solve(tu, p[1] - k[1], &l, COUPLE, -PITCH);
                let nat = [-PITCH, CURL, COUPLE * CURL];
                // Splay: the finger's plane turned away from its plane on the home key.
                let splay = if tu < 1e-6 || v0[0].hypot(v0[1]) < 1e-6 {
                    0.0
                } else {
                    (v[0] * v0[1] - v[1] * v0[0]).atan2(v[0] * v0[0] + v[1] * v0[1])
                };
                let joints: f64 = (0..3).map(|j| ((chain.angles[j] - nat[j]) / JOINT).powi(2)).sum::<f64>() / 3.0;
                let shortfall = if chain.miss < MISS_TOL { 0.0 } else { chain.miss };
                let mm = shortfall / MISS;
                // The fingertip presses along its distal phalanx; only the key's tilt within the
                // finger's plane counts (sideways tilt is the wrist's roll, not the finger's).
                let dir = if tu < 1e-6 { [0.0, 0.0] } else { [v[0] / tu, v[1] / tu] };
                let a = chain.angles.iter().sum::<f64>();
                let press = [a.cos(), -a.sin()];
                let into = [-(nrm[0] * dir[0] + nrm[2] * dir[1]), -nrm[1]];
                let len = into[0].hypot(into[1]).max(1e-9);
                let angle = ((press[0] * into[0] + press[1] * into[1]) / len).clamp(-1.0, 1.0).acos();
                // Out of reach the finger points straight at the key: the gap lies along it.
                let gap = scaled(norm(sub(p, k)), shortfall);
                (joints + (splay / ABDUCT[ord]).powi(2) + 2.0 * mm + mm * mm, (angle / std::f64::consts::FRAC_PI_4).powi(2), shortfall, gap)
            };
            out.reach[i] = reach as f32;
            out.align[i] = align as f32;
            out.miss[i] = shortfall as f32;
            // A key the planted hand falls short of sideways turns the hand about the wrist.
            let lateral = if shortfall > 0.0 { dot(gap, side) } else { 0.0 };
            let span = sub(p, pl.wrist);
            let extra = lateral.atan2(span[0].hypot(span[2]).max(1.0)).to_degrees() as f32;
            out.wrist[i] = Posture { deviation: rest.deviation + extra, ..rest }.cost();
        }
    }
    for i in 0..n {
        let p = g.phys(i);
        if p != i {
            out.reach[i] = out.reach[p];
            out.align[i] = out.align[p];
            out.wrist[i] = out.wrist[p];
            out.miss[i] = out.miss[p];
        }
    }
    out
}

/// Parity with the web's TypeScript port (`web/src/hand.ts`): this test writes the skeleton,
/// natural curl, finger IK, thumb IK and hand placement for fixed inputs to a JSON fixture that
/// `web/src/hand.parity.test.ts` checks the TypeScript against. Regenerate after an intended
/// change with `GECKO_BLESS=1 cargo test --release -p gecko-core hand_parity`.
#[cfg(test)]
mod parity {
    use super::*;
    use serde_json::{json, Value};

    const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../web/src/fixtures/hand-parity.json");

    /// Integer-valued dims so the f32 Rust fields and f64 TypeScript fields hold the same numbers.
    fn hands() -> Vec<(&'static str, HandDims)> {
        let large = HandDims {
            thumb: [50.0, 35.0, 25.0],
            index: [44.0, 25.0, 20.0],
            middle: [49.0, 29.0, 21.0],
            ring: [46.0, 28.0, 21.0],
            pinky: [36.0, 20.0, 18.0],
            palm_width: 91.0,
            palm_length: 114.0,
            span: 216.0,
            reach: 43.0,
        };
        vec![("average", AVERAGE), ("large", large)]
    }

    fn fixture() -> Value {
        let cases: Vec<Value> = hands()
            .into_iter()
            .map(|(name, d)| {
                let sk = skeleton(&d);
                // Finger names index → pinky; `segments` orders pinky 0 … index 3.
                let natural: Vec<[f64; 2]> = (0..4).map(|i| natural_tip(&d.segments(3 - i))).collect();
                let l = d.segments(2);
                let mut chains = Vec::new();
                for tu in [10.0, 35.0, 55.0, 75.0, 95.0] {
                    for tv in [-80.0, -50.0, -20.0, 5.0] {
                        let c = solve(tu, tv, &l, COUPLE, -PITCH);
                        chains.push(json!({"tu": tu, "tv": tv, "angles": c.angles, "miss": c.miss}));
                    }
                }
                let home = |dz: f64| -> [V3; 4] { std::array::from_fn(|i| [20.0 * i as f64 - 10.0, -60.0, -150.0 + dz * i as f64]) };
                let placements: Vec<Value> = [0.0, 4.0]
                    .into_iter()
                    .map(|dz| {
                        let t = home(dz);
                        let p = place(&t, &d);
                        json!({"targets": t, "wrist": p.wrist, "yaw": p.yaw, "roll": p.roll})
                    })
                    .collect();
                let thumb = Thumb::new(&d, &Placement { wrist: [0.0; 3], yaw: 0.0, roll: 0.0 });
                let reach: Vec<Value> = [[-30.0, -70.0, -110.0], [-10.0, -60.0, -140.0], [-55.0, -50.0, -90.0]]
                    .into_iter()
                    .map(|target| {
                        let (q, miss) = thumb.solve(target);
                        json!({"target": target, "q": q, "miss": miss})
                    })
                    .collect();
                json!({
                    "name": name,
                    "dims": d,
                    "knuckles": sk.knuckles, "thumb_base": sk.thumb, "thumb_rest": sk.thumb_rest,
                    "natural_tip": natural,
                    "middle_chains": chains,
                    "placements": placements,
                    "thumb": {"neutral": thumb.neutral, "neutral_joints": thumb.joints(&thumb.neutral), "reach": reach},
                })
            })
            .collect();
        json!({"note": "Generated by gecko-core hand::parity; do not edit by hand.", "cases": cases})
    }

    /// Every number in `a` and `b` agrees to `tol` (same shape otherwise).
    fn close(a: &Value, b: &Value, tol: f64, at: &str) {
        match (a, b) {
            (Value::Number(x), Value::Number(y)) => {
                let (x, y) = (x.as_f64().unwrap(), y.as_f64().unwrap());
                assert!((x - y).abs() <= tol * (1.0 + y.abs()), "{at}: {x} vs fixture {y}");
            }
            (Value::Array(x), Value::Array(y)) => {
                assert_eq!(x.len(), y.len(), "{at}: length");
                x.iter().zip(y).enumerate().for_each(|(i, (x, y))| close(x, y, tol, &format!("{at}[{i}]")));
            }
            (Value::Object(x), Value::Object(y)) => {
                assert_eq!(x.keys().collect::<Vec<_>>(), y.keys().collect::<Vec<_>>(), "{at}: keys");
                x.iter().for_each(|(k, v)| close(v, &y[k], tol, &format!("{at}.{k}")));
            }
            _ => assert_eq!(a, b, "{at}"),
        }
    }

    #[test]
    fn hand_parity_fixture_is_current() {
        let now = fixture();
        if std::env::var_os("GECKO_BLESS").is_some() {
            std::fs::create_dir_all(std::path::Path::new(FIXTURE).parent().unwrap()).unwrap();
            std::fs::write(FIXTURE, serde_json::to_string_pretty(&now).unwrap() + "\n").unwrap();
            return;
        }
        let text = std::fs::read_to_string(FIXTURE).expect("missing hand parity fixture: run with GECKO_BLESS=1");
        let saved: Value = serde_json::from_str(&text).unwrap();
        close(&now, &saved, 1e-9, "fixture");
    }
}
