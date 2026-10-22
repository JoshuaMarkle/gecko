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
//!
//! Files: `build` folds tables (`new`, `with_weights`, `set_geometry`), `eval` is the hot loop
//! (`full`, `delta*`, `apply`), `load` the finger-load cost functions, `qap` the bound terms.

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

mod build;
mod eval;
mod load;
mod qap;

pub use load::*;
pub(crate) use qap::QapTerms;

impl Scorer {
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
