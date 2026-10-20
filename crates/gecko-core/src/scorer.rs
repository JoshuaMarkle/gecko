//! Folded scorer: full evaluation and O(S)/O(S²) swap deltas. See `context/objective.md`.
//!
//! score = Σ f1[x]·C1[p(x)] + Σ f2[x,y]·C2[p(x),p(y)] + Σ fk[x,z]·Ck[p(x),p(z)]
//!       + Σ f3[x,y,z]·C3[p(x),p(y),p(z)] + load terms (M02, M04)
//!
//! Frequencies are normalized over the symbols present on the layout, so values read as
//! "% of keystrokes / bigrams / skipgrams / trigrams typed on this layout".

use crate::geometry::FINGERS;
use crate::metrics::{self, Dir, Order, METRICS};
use crate::{Corpus, Error, Geometry, Layout, Profile, Result};

/// Max symbols / slots supported by the stack buffers in the hot loop.
pub const MAX_KEYS: usize = 64;

pub struct Scorer {
    pub n: usize,
    /// Symbol id → char. Symbol ids are the start layout's slot order.
    pub chars: Vec<char>,
    pub geometry: Geometry,
    pub profile: Profile,
    pub corpus_name: String,
    f1: Vec<f32>,
    f2: Vec<f32>,
    fk: Vec<f32>,
    f3: Vec<f32>,
    c1: Vec<f32>,
    c2: Vec<f32>,
    ck: Vec<f32>,
    c3: Vec<f32>,
    has_bi: bool,
    has_skip: bool,
    has_tri: bool,
    /// Non-zero trigrams grouped by symbol (CSR): every trigram containing symbol `s` is
    /// stored in `tri_entries[tri_start[s]..tri_start[s + 1]]`. Exact, just sparse.
    tri_start: Vec<u32>,
    tri_entries: Vec<TriEntry>,
    /// All non-zero trigrams once (full evaluation).
    tri_all: Vec<TriEntry>,
    finger: Vec<u8>,
    w_load: f64,
    w_hand: f64,
    target: [f64; FINGERS],
    /// Unfolded per-metric tables aligned with `METRICS` (reports only).
    tables: Vec<Option<Vec<f32>>>,
}

#[derive(Clone, Copy, Debug)]
struct TriEntry {
    x: u8,
    y: u8,
    z: u8,
    f: f32,
}

/// Mutable search state: symbol ↔ slot maps plus finger-load accumulators.
#[derive(Clone, Debug)]
pub struct State {
    /// `pos[symbol] = slot`
    pub pos: Vec<u8>,
    /// `sym[slot] = symbol`
    pub sym: Vec<u8>,
    /// Finger loads in % of keystrokes.
    pub load: [f64; FINGERS],
    pub score: f64,
}

impl State {
    pub fn chars(&self, sc: &Scorer) -> Vec<char> {
        self.sym.iter().map(|&s| sc.chars[s as usize]).collect()
    }
}

impl Scorer {
    /// Build a scorer for the symbol set of `layout` (which must fit `geometry`).
    pub fn new(geometry: &Geometry, corpus: &Corpus, profile: &Profile, layout: &Layout) -> Result<Self> {
        layout.check_geometry(geometry)?;
        let n = geometry.len();
        if n > MAX_KEYS {
            return Err(Error::invalid("geometry", format!("at most {MAX_KEYS} keys")));
        }
        let chars = layout.chars.clone();
        let cidx: Vec<Option<usize>> = chars.iter().map(|&c| corpus.index_of(c)).collect();
        let cs = corpus.size();
        let norm = |v: &mut Vec<f32>| {
            let t: f64 = v.iter().map(|&x| x as f64).sum();
            if t > 0.0 {
                v.iter_mut().for_each(|x| *x = (*x as f64 / t) as f32);
            }
        };
        let mut f1 = vec![0f32; n];
        let mut f2 = vec![0f32; n * n];
        let mut fk = vec![0f32; n * n];
        let mut f3 = vec![0f32; n * n * n];
        for x in 0..n {
            let Some(cx) = cidx[x] else { continue };
            f1[x] = corpus.mono[cx] as f32;
            for y in 0..n {
                let Some(cy) = cidx[y] else { continue };
                f2[x * n + y] = corpus.bi[cx * cs + cy] as f32;
                fk[x * n + y] = corpus.skip[cx * cs + cy] as f32;
                for z in 0..n {
                    let Some(cz) = cidx[z] else { continue };
                    f3[(x * n + y) * n + z] = corpus.tri[(cx * cs + cy) * cs + cz] as f32;
                }
            }
        }
        for v in [&mut f1, &mut f2, &mut fk, &mut f3] {
            norm(v);
        }
        truncate_coverage(&mut f3, profile.trigram_coverage as f64);
        norm(&mut f3);

        let tables: Vec<Option<Vec<f32>>> = METRICS.iter().map(|d| metrics::table(geometry, d)).collect();
        let mut c1 = vec![0f32; n];
        let mut c2 = vec![0f32; n * n];
        let mut ck = vec![0f32; n * n];
        let mut c3 = vec![0f32; n * n * n];
        let (mut has_bi, mut has_skip, mut has_tri) = (false, false, false);
        let (mut w_load, mut w_hand) = (0.0, 0.0);
        for (d, t) in METRICS.iter().zip(&tables) {
            let w = profile.weight(d.id);
            let signed = match d.dir {
                Dir::Min => w,
                Dir::Max => -w,
                Dir::Info => 0.0,
            };
            if signed == 0.0 {
                continue;
            }
            match (d.order, t) {
                (Order::Load, _) => match d.id {
                    "M02" => w_load = w as f64,
                    "M04" => w_hand = w as f64,
                    _ => {}
                },
                (order, Some(t)) => {
                    let dst = match order {
                        Order::Mono => &mut c1,
                        Order::Bi => {
                            has_bi = true;
                            &mut c2
                        }
                        Order::Skip => {
                            has_skip = true;
                            &mut ck
                        }
                        Order::Tri => {
                            has_tri = true;
                            &mut c3
                        }
                        Order::Load => unreachable!(),
                    };
                    dst.iter_mut().zip(t).for_each(|(c, v)| *c += signed * v);
                }
                (_, None) => {}
            }
        }
        let mut tri_all = Vec::new();
        for x in 0..n {
            for y in 0..n {
                for z in 0..n {
                    let f = f3[(x * n + y) * n + z];
                    if f > 0.0 {
                        tri_all.push(TriEntry { x: x as u8, y: y as u8, z: z as u8, f });
                    }
                }
            }
        }
        let mut tri_start = vec![0u32; n + 1];
        let mut tri_entries = Vec::new();
        for sym in 0..n {
            tri_start[sym] = tri_entries.len() as u32;
            let s8 = sym as u8;
            tri_entries.extend(tri_all.iter().filter(|e| e.x == s8 || e.y == s8 || e.z == s8));
        }
        tri_start[n] = tri_entries.len() as u32;
        let mut target = [0.0; FINGERS];
        for (t, &v) in target.iter_mut().zip(&profile.finger_target) {
            *t = v as f64;
        }
        Ok(Scorer {
            n,
            chars,
            geometry: geometry.clone(),
            profile: profile.clone(),
            corpus_name: corpus.name.clone(),
            f1,
            f2,
            fk,
            f3,
            c1,
            c2,
            ck,
            c3,
            has_bi,
            has_skip,
            has_tri,
            tri_start,
            tri_entries,
            tri_all,
            finger: geometry.key.iter().map(|k| k.finger.index() as u8).collect(),
            w_load,
            w_hand,
            target,
            tables,
        })
    }

    /// Symbol ids for `layout` in this scorer's alphabet. Errors if the symbol sets differ.
    pub fn state_for(&self, layout: &Layout) -> Result<State> {
        layout.check_geometry(&self.geometry)?;
        let mut sym = Vec::with_capacity(self.n);
        for &c in &layout.chars {
            let s =
                self.chars.iter().position(|&x| x == c).ok_or_else(|| {
                    Error::invalid("layout", format!("'{}' uses symbol '{c}' not in this scorer's symbol set", layout.name))
                })?;
            sym.push(s as u8);
        }
        Ok(self.state_from_sym(sym))
    }

    pub fn state_from_sym(&self, sym: Vec<u8>) -> State {
        let mut pos = vec![0u8; self.n];
        for (slot, &s) in sym.iter().enumerate() {
            pos[s as usize] = slot as u8;
        }
        let load = self.loads(&pos);
        let mut st = State { pos, sym, load, score: 0.0 };
        st.score = self.full(&st);
        st
    }

    /// Identity state: symbol i on slot i (the start layout).
    pub fn identity(&self) -> State {
        self.state_from_sym((0..self.n as u8).collect())
    }

    pub fn loads(&self, pos: &[u8]) -> [f64; FINGERS] {
        let mut l = [0.0; FINGERS];
        for (x, &p) in pos.iter().enumerate() {
            l[self.finger[p as usize] as usize] += 100.0 * self.f1[x] as f64;
        }
        l
    }

    fn load_cost(&self, load: &[f64; FINGERS]) -> f64 {
        let mut c = 0.0;
        if self.w_load != 0.0 {
            c += self.w_load * load_error_cost(load, &self.target);
        }
        if self.w_hand != 0.0 {
            c += self.w_hand * hand_cost(load);
        }
        c
    }

    /// Full evaluation from scratch. Used for verification and reports, not in the search loop.
    pub fn full(&self, st: &State) -> f64 {
        let n = self.n;
        let pos = &st.pos;
        let mut sum = 0.0f64;
        for x in 0..n {
            sum += self.f1[x] as f64 * self.c1[pos[x] as usize] as f64;
        }
        if self.has_bi || self.has_skip {
            for x in 0..n {
                let px = pos[x] as usize * n;
                for y in 0..n {
                    let py = pos[y] as usize;
                    sum += self.f2[x * n + y] as f64 * self.c2[px + py] as f64 + self.fk[x * n + y] as f64 * self.ck[px + py] as f64;
                }
            }
        }
        if self.has_tri {
            for e in &self.tri_all {
                let i = (pos[e.x as usize] as usize * n + pos[e.y as usize] as usize) * n + pos[e.z as usize] as usize;
                sum += e.f as f64 * self.c3[i] as f64;
            }
        }
        sum + self.load_cost(&self.loads(pos))
    }

    /// Score change if symbols `a` and `b` swap slots. No allocation.
    #[inline]
    pub fn delta(&self, st: &State, a: usize, b: usize) -> f64 {
        if a == b {
            return 0.0;
        }
        let n = self.n;
        let pos = &st.pos;
        let (pa, pb) = (pos[a] as usize, pos[b] as usize);
        let mut old = [0u8; MAX_KEYS];
        let mut new = [0u8; MAX_KEYS];
        old[..n].copy_from_slice(pos);
        new[..n].copy_from_slice(pos);
        new[a] = pb as u8;
        new[b] = pa as u8;
        let (old, new) = (&old[..n], &new[..n]);

        let (c1, f1) = (&self.c1, &self.f1);
        let mut d = (f1[a] * (c1[pb] - c1[pa]) + f1[b] * (c1[pa] - c1[pb])) as f64;

        if self.has_bi || self.has_skip {
            let (c2, ck, f2, fk) = (&self.c2, &self.ck, &self.f2, &self.fk);
            let mut acc = 0.0f64;
            for &x in &[a, b] {
                let (ox, nx) = (old[x] as usize * n, new[x] as usize * n);
                let row = x * n;
                for y in 0..n {
                    let (oy, ny) = (old[y] as usize, new[y] as usize);
                    acc += (f2[row + y] * (c2[nx + ny] - c2[ox + oy]) + fk[row + y] * (ck[nx + ny] - ck[ox + oy])) as f64;
                }
            }
            for x in 0..n {
                if x == a || x == b {
                    continue;
                }
                let px = old[x] as usize * n;
                let row = x * n;
                for &y in &[a, b] {
                    let (oy, ny) = (old[y] as usize, new[y] as usize);
                    acc += (f2[row + y] * (c2[px + ny] - c2[px + oy]) + fk[row + y] * (ck[px + ny] - ck[px + oy])) as f64;
                }
            }
            d += acc;
        }

        if self.has_tri {
            let c3 = &self.c3;
            let cost =
                |m: &[u8], e: &TriEntry| c3[(m[e.x as usize] as usize * n + m[e.y as usize] as usize) * n + m[e.z as usize] as usize];
            let mut acc = 0.0f64;
            let ra = self.tri_start[a] as usize..self.tri_start[a + 1] as usize;
            for e in &self.tri_entries[ra] {
                acc += (e.f * (cost(new, e) - cost(old, e))) as f64;
            }
            let (a8, rb) = (a as u8, self.tri_start[b] as usize..self.tri_start[b + 1] as usize);
            for e in &self.tri_entries[rb] {
                if e.x == a8 || e.y == a8 || e.z == a8 {
                    continue; // already counted with `a`
                }
                acc += (e.f * (cost(new, e) - cost(old, e))) as f64;
            }
            d += acc;
        }

        if self.w_load != 0.0 || self.w_hand != 0.0 {
            let (fa, fb) = (self.finger[pa] as usize, self.finger[pb] as usize);
            if fa != fb {
                let mut l = st.load;
                let moved = 100.0 * (self.f1[b] as f64 - self.f1[a] as f64);
                l[fa] += moved;
                l[fb] -= moved;
                d += self.load_cost(&l) - self.load_cost(&st.load);
            }
        }
        d
    }

    /// Apply the swap of symbols `a` and `b`, given its precomputed delta.
    #[inline]
    pub fn apply(&self, st: &mut State, a: usize, b: usize, delta: f64) {
        let (pa, pb) = (st.pos[a] as usize, st.pos[b] as usize);
        let (fa, fb) = (self.finger[pa] as usize, self.finger[pb] as usize);
        if fa != fb {
            let moved = 100.0 * (self.f1[b] as f64 - self.f1[a] as f64);
            st.load[fa] += moved;
            st.load[fb] -= moved;
        }
        st.pos.swap(a, b);
        st.sym.swap(pa, pb);
        st.score += delta;
    }

    // ---- accessors for reports ----

    pub fn table(&self, metric: usize) -> Option<&[f32]> {
        self.tables[metric].as_deref()
    }
    pub fn f1(&self) -> &[f32] {
        &self.f1
    }
    pub fn f2(&self) -> &[f32] {
        &self.f2
    }
    pub fn fk(&self) -> &[f32] {
        &self.fk
    }
    pub fn f3(&self) -> &[f32] {
        &self.f3
    }
    pub fn target(&self) -> &[f64; FINGERS] {
        &self.target
    }
    pub fn finger_of_slot(&self, slot: usize) -> usize {
        self.finger[slot] as usize
    }
}

/// Zero the least frequent entries so the rest cover `coverage` of the total.
fn truncate_coverage(f: &mut [f32], coverage: f64) {
    if coverage >= 1.0 {
        return;
    }
    let total: f64 = f.iter().map(|&x| x as f64).sum();
    let mut idx: Vec<usize> = (0..f.len()).filter(|&i| f[i] > 0.0).collect();
    idx.sort_by(|&a, &b| f[b].total_cmp(&f[a]).then(a.cmp(&b)));
    let mut acc = 0.0;
    let mut keep = idx.len();
    for (k, &i) in idx.iter().enumerate() {
        acc += f[i] as f64;
        if acc >= coverage * total {
            keep = k + 1;
            break;
        }
    }
    for &i in &idx[keep..] {
        f[i] = 0.0;
    }
}

/// M02 cost: mean squared gap (%²) between finger loads and targets over non-thumb fingers.
pub fn load_error_cost(load: &[f64; FINGERS], target: &[f64; FINGERS]) -> f64 {
    let mut s = 0.0;
    for f in [0, 1, 2, 3, 6, 7, 8, 9] {
        let e = load[f] - target[f];
        s += e * e;
    }
    s / 8.0
}

/// M04 cost: squared distance of the left-hand share from 50%, scaled by 1/10.
pub fn hand_cost(load: &[f64; FINGERS]) -> f64 {
    let left: f64 = load[..5].iter().sum();
    let total: f64 = load.iter().sum();
    if total <= 0.0 {
        return 0.0;
    }
    let l = 100.0 * left / total;
    (l - 50.0) * (l - 50.0) / 10.0
}
