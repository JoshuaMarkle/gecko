//! Folded scorer: full evaluation and O(S)/O(S²) swap deltas. See `context/objective.md`.
//!
//! score = Σ f1[x]·C1[p(x)] + Σ f2[x,y]·C2[p(x),p(y)] + Σ fk[x,z]·Ck[p(x),p(z)]
//!       + Σ fkf[x,z]·Ckf[p(x),p(z)] + Σ f3[x,y,z]·C3[p(x),p(y),p(z)] + load terms (M02, M03, M04, M07, M09)
//!
//! `fk` is gap-1 skipgrams; `fkf` mixes gap-2 and gap-3 skipgrams with `sfb.skip_decay`;
//! `fy` is bigrams of two code symbols (Y02), with its own cost table `Cy`. Y03 adds
//! Σ w[o,c]·Cb[p(o),p(c)] over the few bracket pairs (o, c) on the layout (a sparse list).
//!
//! Shift (layout has `⇧` keys, corpus has case counts): `+ Σ fsh[y]·Csh[p(y)]` (presses
//! before y, per keystroke: the Shift key's effort in M01 and one press in Y01) `+ Σ fsb[x,y]·
//! Csb[p(x),p(y)]` (presses right after x: Y01's same-finger-into-Shift extra).
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

/// `Scorer::hold` entry for base-layer slots.
const NO_HOLD: u8 = u8::MAX;

#[derive(Clone)]
pub struct Scorer {
    pub n: usize,
    /// Symbol id → char. Symbol ids are the start layout's slot order.
    pub chars: Vec<char>,
    /// The geometry slots index: the board, or its virtual layered geometry.
    pub geometry: Geometry,
    pub profile: Profile,
    pub corpus_name: String,
    /// The layout's adaptive keys; the corpus was rewritten for them (`magic::rewrite`).
    pub magic: Vec<crate::magic::Magic>,
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
    /// Shift presses before symbol y / between x and y, per keystroke (zero without Shift).
    fsh: Vec<f32>,
    fsb: Vec<f32>,
    csh: Vec<f32>,
    csb: Vec<f32>,
    /// Unfolded Shift tables (reports): M01 and Y01 per press before slot j, Y01 per (i, j).
    shift_effort: Vec<f32>,
    shift_press: Vec<f32>,
    shift_pair: Vec<f32>,
    /// The Shift channels are in use: `fsh`/`csh` (mono), `fsb`/`csb` (pair).
    has_shift: bool,
    has_shift_pair: bool,
    /// Any bigram, skipgram or far-skipgram cost is non-zero.
    has_pair: bool,
    /// The far-skipgram channel is in use (skipped otherwise; it costs ~12% of delta speed).
    has_far: bool,
    /// The code-symbol bigram channel (Y02) is in use.
    has_sym: bool,
    has_tri: bool,
    /// Bracket pairs on the layout with their weights (Y03); empty when Y03 is unweighted.
    brackets: Vec<PairEntry>,
    cb: Vec<f32>,
    /// Symbol-on-slot costs (L01, L02), `cp[x * n + slot]`; `has_place` when any is non-zero.
    cp: Vec<f32>,
    has_place: bool,
    /// All bracket pairs on the layout, weighted or not (reports).
    bracket_all: Vec<PairEntry>,
    /// Non-zero trigrams grouped by symbol (CSR): every trigram containing symbol `s` is
    /// indexed in `tri_entries[tri_start[s]..tri_start[s + 1]]`. Exact, just sparse.
    tri_start: Vec<u32>,
    tri_entries: Vec<TriEntry>,
    /// All non-zero trigrams once (full evaluation).
    tri_all: Vec<TriEntry>,
    /// Non-zero trigrams grouped by unordered symbol pair (CSR, key `x * n + y` with `x < y`):
    /// every trigram containing both `x` and `y`. Used by [`Scorer::delta_shift`].
    tri_pair_start: Vec<u32>,
    tri_pair: Vec<TriEntry>,
    finger: Vec<u8>,
    /// Finger holding the layer key of each slot ([`NO_HOLD`] on the base layer): a layer
    /// keystroke loads both fingers.
    hold: Vec<u8>,
    w_load: f64,
    w_hand: f64,
    w_rss: f64,
    w_peak: f64,
    /// Any load (non-linear) metric is weighted.
    has_load: bool,
    w_thumb: f64,
    thumb_target: f64,
    target: [f64; FINGERS],
    /// Unfolded per-metric tables aligned with `METRICS` (reports, and refolding after
    /// [`Scorer::set_geometry`]).
    tables: Vec<Option<Vec<f32>>>,
    /// Signed weight each linear metric was folded with (0 = not folded), aligned with `METRICS`.
    fold_w: Vec<f32>,
    /// Weights of the Shift channel's M01 and Y01 parts (`csh`, `csb`).
    shift_w: (f32, f32),
}

#[derive(Clone, Copy, Debug)]
struct TriEntry {
    id: u32,
    x: u8,
    y: u8,
    z: u8,
    f: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct PairEntry {
    pub x: u8,
    pub y: u8,
    pub f: f32,
}

/// Mutable search state: symbol ↔ slot maps, finger loads and current trigram costs.
#[derive(Clone, Debug)]
pub struct State {
    /// `pos[symbol] = slot`
    pub pos: Vec<u8>,
    /// `sym[slot] = symbol`
    pub sym: Vec<u8>,
    /// Finger loads in % of keystrokes; a layer keystroke also loads its layer key's finger.
    pub load: [f64; FINGERS],
    pub score: f64,
    /// Current folded cost of each sparse trigram (aligned with `Scorer::tri_all`).
    /// Empty when no trigram metric is weighted. Refreshed only for touched trigrams.
    tri_cost: Vec<f32>,
}

impl State {
    pub fn chars(&self, sc: &Scorer) -> Vec<char> {
        self.sym.iter().map(|&s| sc.chars[s as usize]).collect()
    }
}

impl Scorer {
    /// Build a scorer for the symbol set of `layout` (which must fit `geometry`; layers are
    /// flattened onto the virtual geometry). With adaptive keys the corpus is first rewritten
    /// as typed with them.
    pub fn new(geometry: &Geometry, corpus: &Corpus, profile: &Profile, layout: &Layout) -> Result<Self> {
        let board = geometry;
        let rewritten;
        let corpus = if layout.magic.is_empty() {
            corpus
        } else {
            layout.check_magic()?;
            rewritten = crate::magic::rewrite(corpus, &layout.magic)?;
            &rewritten
        };
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
        let shift_keys: Vec<usize> = (0..n).filter(|&i| chars[i] == crate::layout::SHIFT).collect();
        let (mut fsh, mut fsb) = (vec![0f32; n], vec![0f32; n * n]);
        let mono_total: f64 = (0..n).filter_map(|x| cidx[x]).map(|cx| corpus.mono[cx] as f64).sum();
        if !shift_keys.is_empty() && mono_total > 0.0 {
            for x in 0..n {
                let Some(cx) = cidx[x] else { continue };
                fsh[x] = (corpus.shift[cx] as f64 / mono_total) as f32;
                for y in 0..n {
                    let Some(cy) = cidx[y] else { continue };
                    fsb[x * n + y] = (corpus.shift_bi[cx * cs + cy] as f64 / mono_total) as f32;
                }
            }
        }
        let (shift_effort, shift_press, shift_pair) = shift_tables(geometry, &shift_keys);
        truncate_coverage(&mut f3, profile.trigram_coverage as f64);
        norm(&mut f3);

        // Y03 weights: each bracket pair on the layout by the corpus frequency of its two symbols.
        let mut bracket_all = Vec::new();
        for (o, c) in metrics::BRACKET_PAIRS {
            let (Some(x), Some(y)) = (chars.iter().position(|&ch| ch == o), chars.iter().position(|&ch| ch == c)) else {
                continue;
            };
            let f = cidx[x].map_or(0, |i| corpus.mono[i]) as f64 + cidx[y].map_or(0, |i| corpus.mono[i]) as f64;
            if f > 0.0 {
                bracket_all.push((x as u8, y as u8, f));
            }
        }
        let bracket_total: f64 = bracket_all.iter().map(|e| e.2).sum();
        let bracket_all: Vec<PairEntry> =
            bracket_all.into_iter().map(|(x, y, f)| PairEntry { x, y, f: (f / bracket_total) as f32 }).collect();

        let qwerty = metrics::qwerty_slots(board, &chars);
        let prep = metrics::Prep::new(geometry);
        let tables: Vec<Option<Vec<f32>>> = METRICS
            .iter()
            .map(|d| match d.order {
                Order::Place => Some(metrics::place_table(d.id, geometry, profile, &chars, &qwerty, &f1)),
                _ => {
                    let mut t = Vec::new();
                    metrics::update_table(geometry, profile, &prep, d, &mut t, None).then_some(t)
                }
            })
            .collect();
        let mut c1 = vec![0f32; n];
        let mut c2 = vec![0f32; n * n];
        let mut ck = vec![0f32; n * n];
        let mut ckf = vec![0f32; n * n];
        let mut cy = vec![0f32; n * n];
        let mut c3 = vec![0f32; n * n * n];
        let mut cb = vec![0f32; n * n];
        let mut cp = vec![0f32; n * n];
        let (mut has_pair, mut has_far, mut has_sym, mut has_tri) = (false, false, false, false);
        let has_shift = fsh.iter().any(|&f| f > 0.0);
        let w_of = |id: &str| if has_shift { profile.effective_weight(id) } else { 0.0 };
        let (w_effort, w_y01) = (w_of("M01"), w_of("Y01"));
        let csh: Vec<f32> = shift_effort.iter().zip(&shift_press).map(|(e, p)| w_effort * e + w_y01 * p).collect();
        let csb: Vec<f32> = shift_pair.iter().map(|p| w_y01 * p).collect();
        let has_shift_pair = has_shift && csb.iter().any(|&c| c != 0.0) && fsb.iter().any(|&f| f > 0.0);
        let (mut w_load, mut w_hand, mut w_rss, mut w_peak) = (0.0, 0.0, 0.0, 0.0);
        let mut w_thumb = 0.0;
        let mut fold_w = vec![0f32; METRICS.len()];
        for (mi, (d, t)) in METRICS.iter().zip(&tables).enumerate() {
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
                    "M03" => w_rss = w as f64,
                    "M07" => w_peak = w as f64,
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
                        Order::Brackets => &mut cb,
                        Order::Place => &mut cp,
                        Order::Load => unreachable!(),
                    };
                    fold_w[mi] = signed;
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
                        tri_all.push(TriEntry { id: tri_all.len() as u32, x: x as u8, y: y as u8, z: z as u8, f });
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
        let pairs_of = |e: &TriEntry| {
            let mut v = [(e.x, e.y), (e.x, e.z), (e.y, e.z)].map(|(a, b)| (a.min(b) as usize, a.max(b) as usize));
            v.sort_unstable();
            let mut out = [usize::MAX; 3];
            for (i, &(a, b)) in v.iter().enumerate() {
                if a != b && (i == 0 || v[i - 1] != (a, b)) {
                    out[i] = a * n + b;
                }
            }
            out
        };
        let mut tri_pair_start = vec![0u32; n * n + 1];
        for e in &tri_all {
            for k in pairs_of(e).into_iter().filter(|&k| k != usize::MAX) {
                tri_pair_start[k + 1] += 1;
            }
        }
        for k in 0..n * n {
            tri_pair_start[k + 1] += tri_pair_start[k];
        }
        let mut tri_pair = vec![TriEntry { id: 0, x: 0, y: 0, z: 0, f: 0.0 }; tri_pair_start[n * n] as usize];
        let mut fill = tri_pair_start.clone();
        for e in &tri_all {
            for k in pairs_of(e).into_iter().filter(|&k| k != usize::MAX) {
                tri_pair[fill[k] as usize] = *e;
                fill[k] += 1;
            }
        }
        let has_place = cp.iter().any(|&c| c != 0.0);
        let brackets = if cb.iter().any(|&c| c != 0.0) { bracket_all.clone() } else { Vec::new() };
        if let Some(h) = &profile.hand {
            h.validate()?;
        }
        if let Some(e) = &profile.finger_effort {
            e.validate()?;
        }
        let (target, thumb_target) = finger_targets(profile, geometry);
        Ok(Scorer {
            n,
            chars,
            geometry: geometry.clone(),
            profile: profile.clone(),
            corpus_name: corpus.name.clone(),
            magic: layout.magic.clone(),
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
            fsh,
            fsb,
            csh,
            csb,
            shift_effort,
            shift_press,
            shift_pair,
            has_shift,
            has_shift_pair,
            has_pair,
            has_far,
            has_sym,
            has_tri,
            brackets,
            bracket_all,
            cb,
            cp,
            has_place,
            tri_start,
            tri_entries,
            tri_all,
            tri_pair_start,
            tri_pair,
            finger: geometry.key.iter().map(|k| k.finger.index() as u8).collect(),
            hold: (0..n).map(|i| geometry.holder(i).map_or(NO_HOLD, |h| geometry.key[h].finger.index() as u8)).collect(),
            w_load,
            w_hand,
            w_rss,
            w_peak,
            has_load: w_load != 0.0 || w_hand != 0.0 || w_rss != 0.0 || w_peak != 0.0 || w_thumb != 0.0,
            w_thumb,
            thumb_target,
            target,
            tables,
            fold_w,
            shift_w: (w_effort, w_y01),
        })
    }

    /// Re-fold existing metric tables with new nonnegative weights in natural cost units.
    /// Geometry, corpus, table parameters and symbol ids are unchanged. Construct fresh states
    /// with the returned scorer: a state's cached costs belong to its constructing scorer.
    pub fn with_weights(&self, weights: &[(&str, f32)]) -> Result<Self> {
        let mut out = self.clone();
        out.profile.weights.clear();
        out.profile.scale.clear();
        for &(id, weight) in weights {
            let mi = metrics::index_of(id).ok_or_else(|| Error::invalid("objectives", format!("unknown metric '{id}'")))?;
            if METRICS[mi].dir == Dir::Info || !weight.is_finite() || weight < 0.0 {
                return Err(Error::invalid("objectives", format!("'{id}' needs a scored metric and a finite nonnegative weight")));
            }
            out.profile.weights.insert(id.to_owned(), weight);
        }
        for table in [&mut out.c1, &mut out.c2, &mut out.ck, &mut out.ckf, &mut out.cy, &mut out.c3, &mut out.cb, &mut out.cp] {
            table.fill(0.0);
        }
        (out.w_load, out.w_hand, out.w_rss, out.w_peak, out.w_thumb) = (0.0, 0.0, 0.0, 0.0, 0.0);
        (out.has_pair, out.has_far, out.has_sym, out.has_tri) = (false, false, false, false);
        out.fold_w.fill(0.0);
        for (mi, (d, table)) in METRICS.iter().zip(&out.tables).enumerate() {
            let w = out.profile.weight(d.id);
            if w == 0.0 {
                continue;
            }
            let signed = if d.dir == Dir::Max { -w } else { w };
            match (d.order, table) {
                (Order::Load, _) => match d.id {
                    "M02" => out.w_load = w as f64,
                    "M04" => out.w_hand = w as f64,
                    "M03" => out.w_rss = w as f64,
                    "M07" => out.w_peak = w as f64,
                    "M09" => out.w_thumb = w as f64,
                    _ => {}
                },
                (order, Some(table)) => {
                    let target = match order {
                        Order::Mono => &mut out.c1,
                        Order::Bi => {
                            out.has_pair = true;
                            &mut out.c2
                        }
                        Order::Skip => {
                            out.has_pair = true;
                            &mut out.ck
                        }
                        Order::SkipFar => {
                            out.has_pair = true;
                            out.has_far = true;
                            &mut out.ckf
                        }
                        Order::SymBi => {
                            out.has_pair = true;
                            out.has_sym = true;
                            &mut out.cy
                        }
                        Order::Tri => {
                            out.has_tri = true;
                            &mut out.c3
                        }
                        Order::Brackets => &mut out.cb,
                        Order::Place => &mut out.cp,
                        Order::Load => unreachable!(),
                    };
                    out.fold_w[mi] = signed;
                    for (target, value) in target.iter_mut().zip(table) {
                        *target += signed * value;
                    }
                }
                _ => {}
            }
        }
        out.has_place = out.cp.iter().any(|&c| c != 0.0);
        out.has_load = [out.w_load, out.w_hand, out.w_rss, out.w_peak, out.w_thumb].iter().any(|&w| w != 0.0);
        out.brackets = if out.cb.iter().any(|&c| c != 0.0) { out.bracket_all.clone() } else { Vec::new() };
        let (effort, layer) = (out.profile.weight("M01"), out.profile.weight("Y01"));
        out.shift_w = (effort, layer);
        for ((cost, e), p) in out.csh.iter_mut().zip(&out.shift_effort).zip(&out.shift_press) {
            *cost = effort * e + layer * p;
        }
        for (cost, p) in out.csb.iter_mut().zip(&out.shift_pair) {
            *cost = layer * p;
        }
        out.has_shift_pair = out.has_shift && out.csb.iter().any(|&c| c != 0.0) && out.fsb.iter().any(|&f| f > 0.0);
        Ok(out)
    }

    /// One metric's actual cost at unit weight (max metrics are negated; load targets retain
    /// their nonlinear penalty). Raw report values may differ, e.g. M04 reports left share.
    pub fn objective(&self, id: &str) -> Result<Self> {
        self.with_weights(&[(id, 1.0)])
    }

    /// This scorer for a moved board: [`Scorer::set_geometry`] on a copy.
    pub fn with_geometry(&self, board: &Geometry) -> Result<Self> {
        let mut out = self.clone();
        out.set_geometry(board)?;
        Ok(out)
    }

    /// Re-cost the scorer for `board`, the physical board with keys moved (pose, size, effort,
    /// any key field) but the same keys in the same slots. Corpus tables, symbol ids and
    /// weights are kept. Only pair/triple table entries that touch a changed slot are
    /// recomputed and refolded: O(k·N) pair and O(k·N²) trigram entries for k changed slots,
    /// instead of every metric's N² and N³ table. A slot changes when its key or its
    /// [`Geometry::metric_points`] entry does (on a 3D board, moving one key re-fits its hand's
    /// plane, so every key of that hand changes); a changed layer key or comfort table changes
    /// them all. The result equals [`Scorer::new`] on `board` bit for bit. Returns the number
    /// of changed slots. Existing states hold stale costs: [`Scorer::refresh`] them.
    pub fn set_geometry(&mut self, board: &Geometry) -> Result<usize> {
        let n = self.n;
        let vg = if self.geometry.layer_keys.is_empty() {
            board.validate()?;
            board.clone()
        } else {
            let holders: Vec<_> = self.geometry.layer_keys.iter().copied().zip(self.geometry.layer_modes.iter().copied()).collect();
            board.layered(&holders)?
        };
        let same_slots = vg.len() == n && vg.key.iter().zip(&self.geometry.key).all(|(a, b)| a.layer == b.layer && a.phys == b.phys);
        if !same_slots || !vg.comfort_tables.keys().eq(self.geometry.comfort_tables.keys()) {
            return Err(Error::invalid("geometry", "keys or comfort tables differ from the scorer's board; build a new scorer"));
        }
        let prep = metrics::Prep::new(&vg);
        let (old, old_pts, new_pts) = (&self.geometry.key, self.geometry.metric_points(), prep.points());
        // Two masks: a slot's logical fields (finger, row, home, …) changed, or anything a
        // geometric table reads (its key or metric point) did. A pose-only move leaves the
        // logical tables (trigram classes, SFB, scissors, …) untouched.
        let pose_free = |k: &crate::geometry::Key, o: &crate::geometry::Key| {
            *k == crate::geometry::Key { pos: o.pos, rot: o.rot, size: o.size, ..k.clone() }
        };
        let mut logical: Vec<bool> = (0..n).map(|i| !pose_free(&vg.key[i], &old[i])).collect();
        let mut moved: Vec<bool> = (0..n).map(|i| vg.key[i] != old[i] || old_pts[i] != new_pts[i]).collect();
        if vg.layer_keys.iter().any(|&h| moved[h]) || vg.comfort_tables != self.geometry.comfort_tables {
            logical.fill(true);
            moved.fill(true);
        }
        self.geometry = vg;
        let count = moved.iter().filter(|&&c| c).count();
        if count == 0 {
            return Ok(0);
        }
        let any_logical = logical.contains(&true);
        let mask = |id: &str| if metrics::reads_points(id) { &moved } else { &logical };
        let geometry = &self.geometry;
        let qwerty = metrics::qwerty_slots(board, &self.chars);
        for (d, t) in METRICS.iter().zip(&mut self.tables) {
            match (d.order, t) {
                (Order::Load, _) | (_, None) => {}
                (Order::Place, Some(t)) => *t = metrics::place_table(d.id, geometry, &self.profile, &self.chars, &qwerty, &self.f1),
                (Order::Mono, Some(t)) => {
                    metrics::update_table(geometry, &self.profile, &prep, d, t, None);
                }
                (_, Some(t)) => {
                    if any_logical || metrics::reads_points(d.id) {
                        metrics::update_table(geometry, &self.profile, &prep, d, t, Some(mask(d.id)));
                    }
                }
            }
        }

        // Refold exactly as `new` does (0, then + w·t in `METRICS` order), so results match.
        // Per order, only entries a folded table may have changed in.
        let (tables, fold_w) = (&self.tables, &self.fold_w);
        let srcs = |order: Order| -> (Vec<(&[f32], f32)>, &Vec<bool>) {
            let used: Vec<_> =
                METRICS.iter().zip(tables).zip(fold_w).filter(|((d, t), &w)| d.order == order && w != 0.0 && t.is_some()).collect();
            let mask = if used.iter().any(|((d, _), _)| metrics::reads_points(d.id)) { &moved } else { &logical };
            (used.into_iter().map(|((_, t), &w)| (t.as_deref().unwrap(), w)).collect(), mask)
        };
        let fold = |dst: &mut [f32], srcs: &[(&[f32], f32)], e: usize| {
            let mut c = 0.0f32;
            for (t, w) in srcs {
                c += w * t[e];
            }
            dst[e] = c;
        };
        for (dst, order) in [(&mut self.c1, Order::Mono), (&mut self.cp, Order::Place)] {
            let (s, _) = srcs(order);
            (0..dst.len()).for_each(|e| fold(dst, &s, e));
        }
        for (dst, order) in [
            (&mut self.c2, Order::Bi),
            (&mut self.ck, Order::Skip),
            (&mut self.ckf, Order::SkipFar),
            (&mut self.cy, Order::SymBi),
            (&mut self.cb, Order::Brackets),
        ] {
            let (s, ch) = srcs(order);
            let idx: Vec<usize> = (0..n).filter(|&i| ch[i]).collect();
            for i in 0..n {
                if ch[i] {
                    (i * n..(i + 1) * n).for_each(|e| fold(dst, &s, e));
                } else {
                    idx.iter().for_each(|&j| fold(dst, &s, i * n + j));
                }
            }
        }
        let (s, ch) = srcs(Order::Tri);
        let idx: Vec<usize> = (0..n).filter(|&i| ch[i]).collect();
        if !idx.is_empty() {
            for ij in 0..n * n {
                let row = ij * n;
                if ch[ij / n] || ch[ij % n] {
                    (row..row + n).for_each(|e| fold(&mut self.c3, &s, e));
                } else {
                    idx.iter().for_each(|&l| fold(&mut self.c3, &s, row + l));
                }
            }
        }

        let geometry = &self.geometry;
        self.finger = geometry.key.iter().map(|k| k.finger.index() as u8).collect();
        self.hold = (0..n).map(|i| geometry.holder(i).map_or(NO_HOLD, |h| geometry.key[h].finger.index() as u8)).collect();
        let shift_keys: Vec<usize> = (0..n).filter(|&i| self.chars[i] == crate::layout::SHIFT).collect();
        (self.shift_effort, self.shift_press, self.shift_pair) = shift_tables(geometry, &shift_keys);
        let (w_effort, w_y01) = self.shift_w;
        self.csh = self.shift_effort.iter().zip(&self.shift_press).map(|(e, p)| w_effort * e + w_y01 * p).collect();
        self.csb = self.shift_pair.iter().map(|p| w_y01 * p).collect();
        self.has_shift_pair = self.has_shift && self.csb.iter().any(|&c| c != 0.0) && self.fsb.iter().any(|&f| f > 0.0);
        self.has_place = self.cp.iter().any(|&c| c != 0.0);
        self.brackets = if self.cb.iter().any(|&c| c != 0.0) { self.bracket_all.clone() } else { Vec::new() };
        (self.target, self.thumb_target) = finger_targets(&self.profile, geometry);
        Ok(count)
    }

    /// Recompute a state's cached loads, trigram costs and score (after [`Scorer::set_geometry`]).
    pub fn refresh(&self, st: &mut State) {
        *st = self.state_from_sym(std::mem::take(&mut st.sym));
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
            let s = (0..self.n)
                .find(|&i| self.chars[i] == c && used & (1 << i) == 0)
                .ok_or_else(|| Error::invalid("layout", format!("'{}' uses symbol '{c}' not in this scorer's symbol set", layout.name)))?;
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
        g.layer_modes.clear();
        let (n0, n) = (g.len(), self.n);
        for t in g.comfort_tables.values_mut() {
            *t = (0..n0 * n0).map(|ij| t[(ij / n0) * n + ij % n0]).collect();
        }
        g
    }

    /// The layout of state `st` (layers split back out, adaptive keys attached).
    pub fn layout_of(&self, st: &State, name: &str) -> Result<Layout> {
        Layout::from_flat(name, &self.base_geometry(), st.chars(self))?.with_magic(self.magic.clone())
    }

    pub fn state_from_sym(&self, sym: Vec<u8>) -> State {
        let mut pos = vec![0u8; self.n];
        for (slot, &s) in sym.iter().enumerate() {
            pos[s as usize] = slot as u8;
        }
        let load = self.loads(&pos);
        let tri_cost = if self.has_tri { self.tri_all.iter().map(|e| self.trigram_cost(&pos, e)).collect() } else { Vec::new() };
        let mut st = State { pos, sym, load, score: 0.0, tri_cost };
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
            let v = 100.0 * self.f1[x] as f64;
            l[self.finger[p as usize] as usize] += v;
            if self.hold[p as usize] != NO_HOLD {
                l[self.hold[p as usize] as usize] += v;
            }
        }
        l
    }

    /// Move `moved` % of keystrokes onto slot `pa`'s fingers and off slot `pb`'s (a swap).
    #[inline]
    fn shift_load(&self, l: &mut [f64; FINGERS], pa: usize, pb: usize, moved: f64) {
        l[self.finger[pa] as usize] += moved;
        l[self.finger[pb] as usize] -= moved;
        if self.hold[pa] != NO_HOLD {
            l[self.hold[pa] as usize] += moved;
        }
        if self.hold[pb] != NO_HOLD {
            l[self.hold[pb] as usize] -= moved;
        }
    }

    /// Slots `pa` and `pb` load the same fingers (a swap leaves loads unchanged).
    #[inline]
    fn same_load(&self, pa: usize, pb: usize) -> bool {
        self.finger[pa] == self.finger[pb] && self.hold[pa] == self.hold[pb]
    }

    fn load_cost(&self, load: &[f64; FINGERS]) -> f64 {
        let mut c = 0.0;
        if self.w_load != 0.0 {
            c += self.w_load * load_error_cost(load, &self.target);
        }
        if self.w_hand != 0.0 {
            c += self.w_hand * hand_cost(load);
        }
        if self.w_rss != 0.0 {
            c += self.w_rss * balance_rss(load);
        }
        if self.w_peak != 0.0 {
            c += self.w_peak * peak_load(load, &self.target);
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
        if self.has_shift {
            for x in 0..n {
                sum += self.fsh[x] as f64 * self.csh[pos[x] as usize] as f64;
            }
        }
        if self.has_place {
            for x in 0..n {
                sum += self.cp[x * n + pos[x] as usize] as f64;
            }
        }
        if self.has_shift_pair {
            for x in 0..n {
                for y in 0..n {
                    sum += self.fsb[x * n + y] as f64 * self.csb[pos[x] as usize * n + pos[y] as usize] as f64;
                }
            }
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
        for e in &self.brackets {
            sum += e.f as f64 * self.cb[pos[e.x as usize] as usize * n + pos[e.y as usize] as usize] as f64;
        }
        sum + self.load_cost(&self.loads(pos))
    }

    /// Score change if symbols `a` and `b` swap slots. No allocation.
    #[inline]
    pub fn delta(&self, st: &State, a: usize, b: usize) -> f64 {
        self.delta_linear(st, a, b) + self.delta_load(st, a, b)
    }

    /// The part of [`Scorer::delta`] from the n-gram terms (everything but the finger-load
    /// terms, which are not linear in the frequencies).
    #[inline]
    pub fn delta_linear(&self, st: &State, a: usize, b: usize) -> f64 {
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
        if self.has_shift {
            let (csh, fsh) = (&self.csh, &self.fsh);
            d += (fsh[a] * (csh[pb] - csh[pa]) + fsh[b] * (csh[pa] - csh[pb])) as f64;
        }
        if self.has_place {
            let cp = &self.cp;
            d += (cp[a * n + pb] - cp[a * n + pa] + cp[b * n + pa] - cp[b * n + pb]) as f64;
        }

        if self.has_pair || self.has_shift_pair {
            d += match (self.has_far, self.has_sym, self.has_shift_pair) {
                (false, false, false) => self.pair_delta::<false, false, false>(old, new, a, b),
                (true, false, false) => self.pair_delta::<true, false, false>(old, new, a, b),
                (false, true, false) => self.pair_delta::<false, true, false>(old, new, a, b),
                (true, true, false) => self.pair_delta::<true, true, false>(old, new, a, b),
                (false, false, true) => self.pair_delta::<false, false, true>(old, new, a, b),
                (true, false, true) => self.pair_delta::<true, false, true>(old, new, a, b),
                (false, true, true) => self.pair_delta::<false, true, true>(old, new, a, b),
                (true, true, true) => self.pair_delta::<true, true, true>(old, new, a, b),
            };
        }

        if self.has_tri {
            let mut acc = 0.0f64;
            let ra = self.tri_start[a] as usize..self.tri_start[a + 1] as usize;
            for e in &self.tri_entries[ra] {
                acc += (e.f * (self.trigram_cost(new, e) - st.tri_cost[e.id as usize])) as f64;
            }
            let (a8, rb) = (a as u8, self.tri_start[b] as usize..self.tri_start[b + 1] as usize);
            for e in &self.tri_entries[rb] {
                if e.x == a8 || e.y == a8 || e.z == a8 {
                    continue; // already counted with `a`
                }
                acc += (e.f * (self.trigram_cost(new, e) - st.tri_cost[e.id as usize])) as f64;
            }
            d += acc;
        }
        for e in &self.brackets {
            let (x, y) = (e.x as usize, e.y as usize);
            if x == a || x == b || y == a || y == b {
                d += (e.f * (self.cb[new[x] as usize * n + new[y] as usize] - self.cb[old[x] as usize * n + old[y] as usize])) as f64;
            }
        }
        d
    }

    /// The finger-load part of [`Scorer::delta`] (M02, M03, M04, M07).
    #[inline]
    pub fn delta_load(&self, st: &State, a: usize, b: usize) -> f64 {
        if !self.has_load {
            return 0.0;
        }
        let (pa, pb) = (st.pos[a] as usize, st.pos[b] as usize);
        if self.same_load(pa, pb) {
            return 0.0;
        }
        let mut l = st.load;
        let moved = 100.0 * (self.f1[b] as f64 - self.f1[a] as f64);
        self.shift_load(&mut l, pa, pb, moved);
        self.load_cost(&l) - self.load_cost(&st.load)
    }

    /// Change in `delta_linear(u, v)` caused by the swap of `r` and `s`, which `st` already
    /// includes. `{u, v}` and `{r, s}` must be disjoint. Only terms holding a symbol of each
    /// pair change: Taillard's O(1) QAP update for the pair channels, plus the trigrams that
    /// contain both pairs (via the pair index). Lets tabu search keep a Δ matrix up to date
    /// without recomputing every swap. No allocation.
    #[inline]
    pub fn delta_shift(&self, st: &State, r: usize, s: usize, u: usize, v: usize) -> f64 {
        let n = self.n;
        let pos = &st.pos;
        // Slots before the r/s swap.
        let (pr, ps, pu, pv) = (pos[s] as usize, pos[r] as usize, pos[u] as usize, pos[v] as usize);
        let mut d = 0.0f64;
        if self.has_pair || self.has_shift_pair {
            let chan = |f: &[f32], c: &[f32]| {
                let fwd = f[u * n + r] - f[v * n + r] - f[u * n + s] + f[v * n + s];
                let rev = f[r * n + u] - f[r * n + v] - f[s * n + u] + f[s * n + v];
                let g = c[pv * n + ps] - c[pu * n + ps] - c[pv * n + pr] + c[pu * n + pr];
                let h = c[ps * n + pv] - c[ps * n + pu] - c[pr * n + pv] + c[pr * n + pu];
                (fwd * g + rev * h) as f64
            };
            d += chan(&self.f2, &self.c2) + chan(&self.fk, &self.ck);
            if self.has_far {
                d += chan(&self.fkf, &self.ckf);
            }
            if self.has_sym {
                d += chan(&self.fy, &self.cy);
            }
            if self.has_shift_pair {
                d += chan(&self.fsb, &self.csb);
            }
        }
        if self.has_tri {
            let c3 = &self.c3;
            // Slot of symbol `z` with the r/s swap undone (`back`) and/or u/v swapped (`uv`).
            let slot = |z: usize, back: bool, uv: bool| -> usize {
                if uv && z == u {
                    pv
                } else if uv && z == v {
                    pu
                } else if back && z == r {
                    pr
                } else if back && z == s {
                    ps
                } else {
                    pos[z] as usize
                }
            };
            let cost = |e: &TriEntry, back: bool, uv: bool| {
                let (x, y, z) = (slot(e.x as usize, back, uv), slot(e.y as usize, back, uv), slot(e.z as usize, back, uv));
                c3[(x * n + y) * n + z]
            };
            let has = |e: &TriEntry, w: usize| e.x as usize == w || e.y as usize == w || e.z as usize == w;
            let list = |a: usize, b: usize| {
                let k = a.min(b) * n + a.max(b);
                &self.tri_pair[self.tri_pair_start[k] as usize..self.tri_pair_start[k + 1] as usize]
            };
            let mut acc = 0.0f64;
            // Each trigram once: skip it in a later list if an earlier one already holds it.
            for (i, (a, b)) in [(u, r), (u, s), (v, r), (v, s)].into_iter().enumerate() {
                for e in list(a, b) {
                    let seen = match i {
                        1 => has(e, r),
                        2 => has(e, u),
                        3 => has(e, u) || has(e, r),
                        _ => false,
                    };
                    if !seen {
                        acc +=
                            (e.f * ((cost(e, false, true) - cost(e, false, false)) - (cost(e, true, true) - cost(e, true, false)))) as f64;
                    }
                }
            }
            d += acc;
        }
        // Y03 is sparse, so update only bracket pairs spanning the two swaps.
        for e in &self.brackets {
            let (x, y) = (e.x as usize, e.y as usize);
            if !((x == r || x == s) && (y == u || y == v) || (y == r || y == s) && (x == u || x == v)) {
                continue;
            }
            let slot = |z: usize, back: bool, uv: bool| {
                if uv && z == u {
                    pv
                } else if uv && z == v {
                    pu
                } else if back && z == r {
                    pr
                } else if back && z == s {
                    ps
                } else {
                    pos[z] as usize
                }
            };
            let cost = |back, uv| self.cb[slot(x, back, uv) * n + slot(y, back, uv)];
            d += (e.f * ((cost(false, true) - cost(false, false)) - (cost(true, true) - cost(true, false)))) as f64;
        }
        d
    }

    /// Bigram + skipgram (+ far skipgram if `FAR`, + symbol bigram if `SYM`, + Shift pair if
    /// `SHIFT`) part of a swap delta; `old`/`new` are `pos` before/after.
    #[inline]
    fn pair_delta<const FAR: bool, const SYM: bool, const SHIFT: bool>(&self, old: &[u8], new: &[u8], a: usize, b: usize) -> f64 {
        let n = self.n;
        let (c2, ck, ckf, f2, fk, fkf) = (&self.c2, &self.ck, &self.ckf, &self.f2, &self.fk, &self.fkf);
        let (cy, fy, csb, fsb) = (&self.cy, &self.fy, &self.csb, &self.fsb);
        // Change in the pair channels for frequency index `xy` moving from slot pair `o` to `nw`.
        let term = |xy: usize, o: usize, nw: usize| {
            let mut t = f2[xy] * (c2[nw] - c2[o]) + fk[xy] * (ck[nw] - ck[o]);
            if FAR {
                t += fkf[xy] * (ckf[nw] - ckf[o]);
            }
            if SYM {
                t += fy[xy] * (cy[nw] - cy[o]);
            }
            if SHIFT {
                t += fsb[xy] * (csb[nw] - csb[o]);
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
        if !self.same_load(pa, pb) {
            self.shift_load(&mut st.load, pa, pb, 100.0 * (self.f1[b] as f64 - self.f1[a] as f64));
        }
        st.pos.swap(a, b);
        st.sym.swap(pa, pb);
        st.score += delta;
        if self.has_tri && a != b {
            let ra = self.tri_start[a] as usize..self.tri_start[a + 1] as usize;
            for e in &self.tri_entries[ra] {
                st.tri_cost[e.id as usize] = self.trigram_cost(&st.pos, e);
            }
            let (a8, rb) = (a as u8, self.tri_start[b] as usize..self.tri_start[b + 1] as usize);
            for e in &self.tri_entries[rb] {
                if e.x != a8 && e.y != a8 && e.z != a8 {
                    st.tri_cost[e.id as usize] = self.trigram_cost(&st.pos, e);
                }
            }
        }
    }

    #[inline]
    fn trigram_cost(&self, pos: &[u8], e: &TriEntry) -> f32 {
        self.c3[(pos[e.x as usize] as usize * self.n + pos[e.y as usize] as usize) * self.n + pos[e.z as usize] as usize]
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
    /// Bracket pairs on the layout and their Y03 weights (sum 1).
    pub fn bracket_pairs(&self) -> &[PairEntry] {
        &self.bracket_all
    }
    pub fn f3(&self) -> &[f32] {
        &self.f3
    }
    /// Unweighted Shift part of metric `id` (M01, Y01; 0 otherwise or without Shift).
    pub fn shift_value(&self, id: &str, st: &State) -> f64 {
        if !self.has_shift {
            return 0.0;
        }
        let (n, pos) = (self.n, &st.pos);
        let mono = |t: &[f32]| (0..n).map(|y| self.fsh[y] as f64 * t[pos[y] as usize] as f64).sum::<f64>();
        match id {
            "M01" => mono(&self.shift_effort),
            "Y01" => {
                let mut v = mono(&self.shift_press);
                for x in 0..n {
                    for y in 0..n {
                        v += self.fsb[x * n + y] as f64 * self.shift_pair[pos[x] as usize * n + pos[y] as usize] as f64;
                    }
                }
                v
            }
            _ => 0.0,
        }
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

    /// `full` split into (mono + pair terms, trigram terms, load terms).
    pub fn parts(&self, st: &State) -> (f64, f64, f64) {
        let n = self.n;
        let pos = &st.pos;
        let tri: f64 = if self.has_tri {
            self.tri_all
                .iter()
                .map(|e| {
                    e.f as f64
                        * self.c3[(pos[e.x as usize] as usize * n + pos[e.y as usize] as usize) * n + pos[e.z as usize] as usize] as f64
                })
                .sum()
        } else {
            0.0
        };
        let load = self.load_cost(&self.loads(pos));
        (self.full(st) - tri - load, tri, load)
    }

    /// The objective as quadratic-assignment terms for lower bounds (`crate::bound`).
    /// With `trigrams`, each trigram is relaxed to pair terms that never exceed its cost.
    /// `None` if a load term has a negative weight (no simple bound on it).
    pub(crate) fn qap_terms(&self, trigrams: bool) -> Option<QapTerms> {
        if [self.w_load, self.w_hand, self.w_rss, self.w_peak, self.w_thumb].iter().any(|&w| w < 0.0) {
            return None;
        }
        let n = self.n;
        let mut mono = vec![0.0f64; n * n];
        for x in 0..n {
            for i in 0..n {
                let mut v = self.f1[x] as f64 * self.c1[i] as f64;
                if self.has_shift {
                    v += self.fsh[x] as f64 * self.csh[i] as f64;
                }
                v += self.cp[x * n + i] as f64;
                mono[x * n + i] = v;
            }
        }
        let mut chans: Vec<(Vec<f32>, Vec<f32>)> = Vec::new();
        if self.has_pair {
            chans.push((self.f2.clone(), self.c2.clone()));
            chans.push((self.fk.clone(), self.ck.clone()));
            if self.has_far {
                chans.push((self.fkf.clone(), self.ckf.clone()));
            }
            if self.has_sym {
                chans.push((self.fy.clone(), self.cy.clone()));
            }
        }
        if self.has_shift_pair {
            chans.push((self.fsb.clone(), self.csb.clone()));
        }
        if !self.brackets.is_empty() {
            let mut fb = vec![0.0f32; n * n];
            for e in &self.brackets {
                fb[e.x as usize * n + e.y as usize] += e.f;
            }
            chans.push((fb, self.cb.clone()));
        }
        if trigrams && self.has_tri {
            let c3 = |i: usize, j: usize, k: usize| self.c3[(i * n + j) * n + k];
            // Distinct x y z: c3[i,j,k] ≥ ⅓(min_k c3 + min_i c3 + min_j c3), the third key on
            // any slot other than the two fixed ones. Repeated symbols: exact pair/mono terms.
            let mut tabs: [Vec<f32>; 6] = std::array::from_fn(|_| vec![0.0f32; n * n]);
            for i in 0..n {
                for j in 0..n {
                    let (mut a, mut b, mut c) = (f32::INFINITY, f32::INFINITY, f32::INFINITY);
                    for k in 0..n {
                        if k != i && k != j {
                            a = a.min(c3(i, j, k)); // (x, y) at (i, j)
                            b = b.min(c3(k, i, j)); // (y, z) at (i, j)
                            c = c.min(c3(i, k, j)); // (x, z) at (i, j)
                        }
                    }
                    let ij = i * n + j;
                    let fin = |v: f32| if v.is_finite() { v } else { 0.0 };
                    tabs[0][ij] = fin(a);
                    tabs[1][ij] = fin(b);
                    tabs[2][ij] = fin(c);
                    tabs[3][ij] = c3(i, j, i); // x = z: pair (x, y)
                    tabs[4][ij] = c3(i, i, j); // x = y: pair (x, z)
                    tabs[5][ij] = c3(i, j, j); // y = z: pair (x, y)
                }
            }
            let mut flows: [Vec<f32>; 6] = std::array::from_fn(|_| vec![0.0f32; n * n]);
            for e in &self.tri_all {
                let (x, y, z) = (e.x as usize, e.y as usize, e.z as usize);
                match (x == y, y == z, x == z) {
                    (true, true, _) => {
                        for i in 0..n {
                            mono[x * n + i] += e.f as f64 * c3(i, i, i) as f64;
                        }
                    }
                    (false, false, true) => flows[3][x * n + y] += e.f,
                    (true, false, false) => flows[4][x * n + z] += e.f,
                    (false, true, false) => flows[5][x * n + y] += e.f,
                    _ => {
                        let t = e.f / 3.0;
                        flows[0][x * n + y] += t;
                        flows[1][y * n + z] += t;
                        flows[2][x * n + z] += t;
                    }
                }
            }
            for (f, c) in flows.into_iter().zip(tabs) {
                if f.iter().any(|&v| v != 0.0) {
                    chans.push((f, c));
                }
            }
        }
        // Same-symbol pairs (x = x) sit on one slot: fold them into the mono terms.
        for (f, c) in &chans {
            for x in 0..n {
                let fx = f[x * n + x] as f64;
                if fx != 0.0 {
                    for i in 0..n {
                        mono[x * n + i] += fx * c[i * n + i] as f64;
                    }
                }
            }
        }
        Some(QapTerms { n, mono, chans })
    }
}

/// The objective as `Σ mono[x, p(x)] + Σ_{x≠y} Σ_ch F[x, y]·C[p(x), p(y)]` (see
/// [`Scorer::qap_terms`]); load terms (all weights ≥ 0) are dropped, which only lowers it.
pub(crate) struct QapTerms {
    pub n: usize,
    /// `mono[x * n + i]`: symbol x on slot i, including same-symbol pair terms.
    pub mono: Vec<f64>,
    /// (flow `F[x * n + y]`, cost `C[i * n + j]`) per pair channel.
    pub chans: Vec<(Vec<f32>, Vec<f32>)>,
}

/// Unfolded Shift cost tables for the `⇧` slots `keys` (all zero without one): the Shift key
/// that types slot j is the best of `keys` by (on j's finger, on j's hand, effort, index).
/// Per press before slot j: that key's effort (M01) and Y01 = 100, 200 when it is on j's finger
/// (held while j presses). Per press between slots i and j: Y01 + 100 when i is a different key
/// on that Shift key's finger (same finger into Shift).
fn shift_tables(g: &Geometry, keys: &[usize]) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    let n = g.len();
    let (mut effort, mut press, mut pair) = (vec![0f32; n], vec![0f32; n], vec![0f32; n * n]);
    if keys.is_empty() {
        return (effort, press, pair);
    }
    let k = &g.key;
    for j in 0..n {
        let rank = |s: usize| (k[s].finger == k[j].finger, k[s].hand() == k[j].hand());
        let s = keys.iter().copied().min_by(|&a, &b| rank(a).cmp(&rank(b)).then(k[a].effort.total_cmp(&k[b].effort))).unwrap();
        effort[j] = k[s].effort;
        press[j] = if k[s].finger == k[j].finger { 200.0 } else { 100.0 };
        for i in 0..n {
            if k[i].finger == k[s].finger && g.phys(i) != s {
                pair[i * n + j] = 100.0;
            }
        }
    }
    (effort, press, pair)
}

/// Target finger loads (%) and the thumb target: the profile's `finger_target`, or with
/// `finger_effort` each finger present on `g` by inverse typing cost, normalized to 100.
fn finger_targets(profile: &Profile, g: &Geometry) -> ([f64; FINGERS], f64) {
    let mut target = [0.0; FINGERS];
    for (t, &v) in target.iter_mut().zip(&profile.finger_target) {
        *t = v as f64;
    }
    if let Some(e) = &profile.finger_effort {
        let mut present = [false; FINGERS];
        for k in &g.key {
            present[k.finger.index()] = true;
        }
        for i in 0..FINGERS {
            target[i] = if present[i] { 1.0 / e.typing[e.index(i)] as f64 } else { 0.0 };
        }
        let total: f64 = target.iter().sum();
        if total > 0.0 {
            for t in &mut target {
                *t *= 100.0 / total;
            }
        }
    }
    let thumb_target = if profile.finger_effort.is_some() { target[4] + target[5] } else { profile.thumb_target as f64 };
    (target, thumb_target)
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

const NON_THUMB: [usize; 8] = [0, 1, 2, 3, 6, 7, 8, 9];

/// Non-thumb finger loads as % of non-thumb keystrokes (indexed like `load`; thumbs 0).
fn non_thumb_shares(load: &[f64; FINGERS]) -> [f64; FINGERS] {
    let total: f64 = NON_THUMB.iter().map(|&f| load[f]).sum();
    let k = if total > 0.0 { 100.0 / total } else { 0.0 };
    let mut s = [0.0; FINGERS];
    for f in NON_THUMB {
        s[f] = k * load[f];
    }
    s
}

/// M03 cost: root-sum-square of non-thumb finger shares (%). 35.4 when all eight are even.
pub fn balance_rss(load: &[f64; FINGERS]) -> f64 {
    non_thumb_shares(load).iter().map(|s| s * s).sum::<f64>().sqrt()
}

/// M07 cost: the highest non-thumb finger share as % of its target share (fingers with a
/// target of 0 are skipped). 100 = no finger above its target.
pub fn peak_load(load: &[f64; FINGERS], target: &[f64; FINGERS]) -> f64 {
    let s = non_thumb_shares(load);
    NON_THUMB.iter().filter(|&&f| target[f] > 0.0).map(|&f| 100.0 * s[f] / target[f]).fold(0.0, f64::max)
}

/// M02 cost: mean squared gap (%²) between finger loads and targets over non-thumb fingers.
/// Loads are taken as shares of non-thumb keystrokes, so thumb keys (Space) don't skew them.
pub fn load_error_cost(load: &[f64; FINGERS], target: &[f64; FINGERS]) -> f64 {
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
