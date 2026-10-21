//! Folded scorer: full evaluation and O(S)/O(S²) swap deltas. See `context/objective.md`.
//!
//! score = Σ f1[x]·C1[p(x)] + Σ f2[x,y]·C2[p(x),p(y)] + Σ fk[x,z]·Ck[p(x),p(z)]
//!       + Σ fkf[x,z]·Ckf[p(x),p(z)] + Σ f3[x,y,z]·C3[p(x),p(y),p(z)] + load terms (M02, M04, M09)
//!
//! `fk` is gap-1 skipgrams; `fkf` mixes gap-2 and gap-3 skipgrams with `sfb.skip_decay`;
//! `fy` is bigrams of two code symbols (Y02), with its own cost table `Cy`.
//!
//! A layered layout is scored on its virtual geometry (`Geometry::layered`): one slot per
//! (layer, key), so layers cost nothing extra in the hot loop beyond the larger N.
//!
//! Frequencies are normalized over the symbols present on the layout, so values read as
//! "% of keystrokes / bigrams / skipgrams / trigrams typed on this layout".

use crate::geometry::FINGERS;
use crate::metrics::{self, Dir, Order, METRICS};
use crate::{Corpus, Error, Geometry, Layout, Profile, Result};

/// Max symbols / slots (layer slots included) supported by the stack buffers in the hot loop.
pub const MAX_KEYS: usize = 128;

pub struct Scorer {
    pub n: usize,
    /// Symbol id → char. Symbol ids are the start layout's slot order.
    pub chars: Vec<char>,
    /// The geometry slots index: the board, or its virtual layered geometry.
    pub geometry: Geometry,
    pub profile: Profile,
    pub corpus_name: String,
    f1: Vec<f32>,
    f2: Vec<f32>,
    fk: Vec<f32>,
    fkf: Vec<f32>,
    fy: Vec<f32>,
    f3: Vec<f32>,
    c1: Vec<f32>,
    c2: Vec<f32>,
    ck: Vec<f32>,
    ckf: Vec<f32>,
    cy: Vec<f32>,
    c3: Vec<f32>,
    /// Any bigram, skipgram or far-skipgram cost is non-zero.
    has_pair: bool,
    /// The far-skipgram channel is in use (skipped otherwise; it costs ~12% of delta speed).
    has_far: bool,
    /// The code-symbol bigram channel (Y02) is in use.
    has_sym: bool,
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
    w_thumb: f64,
    thumb_target: f64,
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
    /// Build a scorer for the symbol set of `layout` (which must fit `geometry`; layers are
    /// flattened onto the virtual geometry).
    pub fn new(geometry: &Geometry, corpus: &Corpus, profile: &Profile, layout: &Layout) -> Result<Self> {
        let (vg, chars) = layout.flat(geometry)?;
        let geometry = &vg;
        let n = geometry.len();
        if n > MAX_KEYS {
            return Err(Error::invalid("geometry", format!("at most {MAX_KEYS} keys")));
        }
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
        let (mut fk2, mut fk3) = (vec![0f32; n * n], vec![0f32; n * n]);
        let mut f3 = vec![0f32; n * n * n];
        for x in 0..n {
            let Some(cx) = cidx[x] else { continue };
            f1[x] = corpus.mono[cx] as f32;
            for y in 0..n {
                let Some(cy) = cidx[y] else { continue };
                f2[x * n + y] = corpus.bi[cx * cs + cy] as f32;
                fk[x * n + y] = corpus.skip[cx * cs + cy] as f32;
                fk2[x * n + y] = corpus.skip2[cx * cs + cy] as f32;
                fk3[x * n + y] = corpus.skip3[cx * cs + cy] as f32;
                for z in 0..n {
                    let Some(cz) = cidx[z] else { continue };
                    f3[(x * n + y) * n + z] = corpus.tri[(cx * cs + cy) * cs + cz] as f32;
                }
            }
        }
        let is_sym: Vec<bool> = chars.iter().map(|c| c.is_ascii_punctuation()).collect();
        let mut fy: Vec<f32> = (0..n * n).map(|xy| if is_sym[xy / n] && is_sym[xy % n] { f2[xy] } else { 0.0 }).collect();
        for v in [&mut f1, &mut f2, &mut fk, &mut fk2, &mut fk3, &mut fy, &mut f3] {
            norm(v);
        }
        // Far skipgrams: gap 2 × decay + gap 3 × decay², each normalized like gap 1, so S02 reads
        // as "% of skipgram volume, decayed" relative to S01.
        let decay = profile.sfb.skip_decay;
        let fkf: Vec<f32> = fk2.iter().zip(&fk3).map(|(a, b)| decay * a + decay * decay * b).collect();
        truncate_coverage(&mut f3, profile.trigram_coverage as f64);
        norm(&mut f3);

        let tables: Vec<Option<Vec<f32>>> = METRICS.iter().map(|d| metrics::table(geometry, profile, d)).collect();
        let mut c1 = vec![0f32; n];
        let mut c2 = vec![0f32; n * n];
        let mut ck = vec![0f32; n * n];
        let mut ckf = vec![0f32; n * n];
        let mut cy = vec![0f32; n * n];
        let mut c3 = vec![0f32; n * n * n];
        let (mut has_pair, mut has_far, mut has_sym, mut has_tri) = (false, false, false, false);
        let (mut w_load, mut w_hand, mut w_thumb) = (0.0, 0.0, 0.0);
        for (d, t) in METRICS.iter().zip(&tables) {
            let w = profile.effective_weight(d.id);
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
                    "M09" => w_thumb = w as f64,
                    _ => {}
                },
                (order, Some(t)) => {
                    let dst = match order {
                        Order::Mono => &mut c1,
                        Order::Bi => {
                            has_pair = true;
                            &mut c2
                        }
                        Order::Skip => {
                            has_pair = true;
                            &mut ck
                        }
                        Order::SkipFar => {
                            has_pair = true;
                            has_far = true;
                            &mut ckf
                        }
                        Order::SymBi => {
                            has_pair = true;
                            has_sym = true;
                            &mut cy
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
            fkf,
            fy,
            f3,
            c1,
            c2,
            ck,
            ckf,
            cy,
            c3,
            has_pair,
            has_far,
            has_sym,
            has_tri,
            tri_start,
            tri_entries,
            tri_all,
            finger: geometry.key.iter().map(|k| k.finger.index() as u8).collect(),
            w_load,
            w_hand,
            w_thumb,
            thumb_target: profile.thumb_target as f64,
            target,
            tables,
        })
    }

    /// Symbol ids for `layout` in this scorer's alphabet. Errors if the symbol sets differ.
    pub fn state_for(&self, layout: &Layout) -> Result<State> {
        let flat = if layout.layers.is_empty() { layout.chars.clone() } else { layout.flat(&self.base_geometry())?.1 };
        if flat.len() != self.n {
            return Err(Error::invalid("layout", format!("'{}' has {} slots, this scorer has {}", layout.name, flat.len(), self.n)));
        }
        let mut sym = Vec::with_capacity(self.n);
        // Repeated symbols (empty slots) take the next unused id with that char.
        let mut used = 0u128;
        for &c in &flat {
            let s = (0..self.n).find(|&i| self.chars[i] == c && used & (1 << i) == 0).ok_or_else(|| {
                Error::invalid("layout", format!("'{}' uses symbol '{c}' not in this scorer's symbol set", layout.name))
            })?;
            used |= 1 << s;
            sym.push(s as u8);
        }
        Ok(self.state_from_sym(sym))
    }

    /// The physical board (the scorer geometry without its layer keys).
    pub fn base_geometry(&self) -> Geometry {
        let mut g = self.geometry.clone();
        g.key.retain(|k| k.layer == 0);
        g.layer_keys.clear();
        let (n0, n) = (g.len(), self.n);
        for t in g.comfort_tables.values_mut() {
            *t = (0..n0 * n0).map(|ij| t[(ij / n0) * n + ij % n0]).collect();
        }
        g
    }

    /// The layout of state `st` (layers split back out).
    pub fn layout_of(&self, st: &State, name: &str) -> Result<Layout> {
        Layout::from_flat(name, &self.base_geometry(), st.chars(self))
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
        if self.w_thumb != 0.0 {
            c += self.w_thumb * thumb_cost(load, self.thumb_target);
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
        if self.has_pair {
            for x in 0..n {
                let px = pos[x] as usize * n;
                for y in 0..n {
                    let (xy, p) = (x * n + y, px + pos[y] as usize);
                    sum += self.f2[xy] as f64 * self.c2[p] as f64
                        + self.fk[xy] as f64 * self.ck[p] as f64
                        + self.fkf[xy] as f64 * self.ckf[p] as f64
                        + self.fy[xy] as f64 * self.cy[p] as f64;
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

        if self.has_pair {
            d += match (self.has_far, self.has_sym) {
                (false, false) => self.pair_delta::<false, false>(old, new, a, b),
                (true, false) => self.pair_delta::<true, false>(old, new, a, b),
                (false, true) => self.pair_delta::<false, true>(old, new, a, b),
                (true, true) => self.pair_delta::<true, true>(old, new, a, b),
            };
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

        if self.w_load != 0.0 || self.w_hand != 0.0 || self.w_thumb != 0.0 {
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

    /// Bigram + skipgram (+ far skipgram if `FAR`, + symbol bigram if `SYM`) part of a swap
    /// delta; `old`/`new` are `pos` before/after.
    #[inline]
    fn pair_delta<const FAR: bool, const SYM: bool>(&self, old: &[u8], new: &[u8], a: usize, b: usize) -> f64 {
        let n = self.n;
        let (c2, ck, ckf, f2, fk, fkf) = (&self.c2, &self.ck, &self.ckf, &self.f2, &self.fk, &self.fkf);
        let (cy, fy) = (&self.cy, &self.fy);
        // Change in the pair channels for frequency index `xy` moving from slot pair `o` to `nw`.
        let term = |xy: usize, o: usize, nw: usize| {
            let mut t = f2[xy] * (c2[nw] - c2[o]) + fk[xy] * (ck[nw] - ck[o]);
            if FAR {
                t += fkf[xy] * (ckf[nw] - ckf[o]);
            }
            if SYM {
                t += fy[xy] * (cy[nw] - cy[o]);
            }
            t
        };
        let mut acc = 0.0f64;
        for &x in &[a, b] {
            let (ox, nx) = (old[x] as usize * n, new[x] as usize * n);
            let row = x * n;
            for y in 0..n {
                acc += term(row + y, ox + old[y] as usize, nx + new[y] as usize) as f64;
            }
        }
        for x in 0..n {
            if x == a || x == b {
                continue;
            }
            let px = old[x] as usize * n;
            let row = x * n;
            for &y in &[a, b] {
                acc += term(row + y, px + old[y] as usize, px + new[y] as usize) as f64;
            }
        }
        acc
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
    pub fn fkf(&self) -> &[f32] {
        &self.fkf
    }
    pub fn fy(&self) -> &[f32] {
        &self.fy
    }
    pub fn f3(&self) -> &[f32] {
        &self.f3
    }
    pub fn target(&self) -> &[f64; FINGERS] {
        &self.target
    }
    pub fn thumb_target(&self) -> f64 {
        self.thumb_target
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
/// Loads are taken as shares of non-thumb keystrokes, so thumb keys (Space) don't skew them.
pub fn load_error_cost(load: &[f64; FINGERS], target: &[f64; FINGERS]) -> f64 {
    const NON_THUMB: [usize; 8] = [0, 1, 2, 3, 6, 7, 8, 9];
    if load[4] == 0.0 && load[5] == 0.0 {
        return NON_THUMB.iter().map(|&f| (load[f] - target[f]).powi(2)).sum::<f64>() / 8.0;
    }
    let total: f64 = NON_THUMB.iter().map(|&f| load[f]).sum();
    let k = if total > 0.0 { 100.0 / total } else { 0.0 };
    let mut s = 0.0;
    for f in NON_THUMB {
        let e = k * load[f] - target[f];
        s += e * e;
    }
    s / 8.0
}

/// M04 value: left-hand share in % of keystrokes.
pub fn left_share(load: &[f64; FINGERS]) -> f64 {
    100.0 * load[..5].iter().sum::<f64>() / load.iter().sum::<f64>().max(1e-12)
}

/// M09 value: thumb share (both thumbs) in % of keystrokes.
pub fn thumb_share(load: &[f64; FINGERS]) -> f64 {
    100.0 * (load[4] + load[5]) / load.iter().sum::<f64>().max(1e-12)
}

/// Squared distance of a share from its target, scaled by 1/10 (M04, M09).
pub fn target_cost(share: f64, target: f64) -> f64 {
    (share - target) * (share - target) / 10.0
}

/// M04 cost: squared distance of the left-hand share from 50%, scaled by 1/10.
pub fn hand_cost(load: &[f64; FINGERS]) -> f64 {
    if load.iter().sum::<f64>() <= 0.0 {
        return 0.0;
    }
    target_cost(left_share(load), 50.0)
}

/// M09 cost: squared distance of the thumb share from `target`, scaled by 1/10.
pub fn thumb_cost(load: &[f64; FINGERS], target: f64) -> f64 {
    if load.iter().sum::<f64>() <= 0.0 {
        return 0.0;
    }
    target_cost(thumb_share(load), target)
}
