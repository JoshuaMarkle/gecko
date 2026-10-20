//! The explore archive: every distinct local optimum with its raw metric vector and hit count,
//! plus LON edges. Shared by all workers behind a mutex; touched once per descent, never in the
//! swap loop, so a `HashMap` is fine here.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::atlas::{Edge, Optimum, CONSTRAINED};
use super::distance::{canonical, hamming, mirror};
use crate::hash::fnv1a;
use crate::optimize::{Kick, Observer};
use crate::report::breakdown;
use crate::scorer::{Scorer, State};

/// Raw metric values in `METRICS` order (the atlas metric list).
pub fn raw_vector(sc: &Scorer, st: &State) -> Vec<f32> {
    breakdown(sc, st).metrics.iter().map(|m| m.value as f32).collect()
}

type Exclusions = Arc<Vec<Vec<u8>>>;

struct Item {
    id: u32,
    opt: Optimum,
}

#[derive(Default)]
struct Inner {
    /// Canonical-layout hash → index into `items`.
    index: HashMap<u64, usize>,
    items: Vec<Item>,
    /// Next stable id; edges refer to ids so pruning never misdirects them.
    next_id: u32,
    edges: HashMap<(u32, u32), (u32, u8, u8)>,
    hits: u64,
    pruned: u64,
}

pub struct Archive<'a> {
    sc: &'a Scorer,
    mirror: Option<Vec<usize>>,
    t0: Instant,
    max: usize,
    exclusion_k: usize,
    exclusion_dist: usize,
    inner: Mutex<Inner>,
    /// When the exclusion list was last computed, and the list.
    excl: Mutex<(Option<Instant>, Exclusions)>,
}

/// A consistent copy of the archive, sorted by score, with edges remapped to row indices.
pub struct Snapshot {
    pub optima: Vec<Optimum>,
    pub edges: Vec<Edge>,
    pub hits: u64,
    pub pruned: u64,
}

impl<'a> Archive<'a> {
    /// `mirror`: canonicalize layouts with this slot map (only when scores are mirror-symmetric).
    pub fn new(sc: &'a Scorer, mirror: Option<Vec<usize>>, max: usize, exclusion_k: usize, exclusion_dist: usize) -> Self {
        Archive {
            sc,
            mirror,
            t0: Instant::now(),
            max: max.max(16),
            exclusion_k,
            exclusion_dist,
            inner: Mutex::new(Inner::default()),
            excl: Mutex::new((None, Arc::new(Vec::new()))),
        }
    }

    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// (optima, hits, edges)
    pub fn counts(&self) -> (usize, u64, usize) {
        let g = self.inner.lock().unwrap();
        (g.items.len(), g.hits, g.edges.len())
    }

    /// Keep the best `max` optima; drop the rest and their edges.
    fn prune(g: &mut Inner, max: usize) {
        g.items.sort_by(|a, b| a.opt.score.total_cmp(&b.opt.score));
        g.pruned += (g.items.len() - max) as u64;
        g.items.truncate(max);
        let keep: std::collections::HashSet<u32> = g.items.iter().map(|i| i.id).collect();
        g.edges.retain(|(a, b), _| keep.contains(a) && keep.contains(b));
        g.index.clear();
        for (i, it) in g.items.iter().enumerate() {
            let h = fnv1a(&it.opt.sym);
            g.index.insert(h, i);
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        let g = self.inner.lock().unwrap();
        let mut order: Vec<usize> = (0..g.items.len()).collect();
        order.sort_by(|&a, &b| g.items[a].opt.score.total_cmp(&g.items[b].opt.score).then(g.items[a].opt.sym.cmp(&g.items[b].opt.sym)));
        let mut row_of: HashMap<u32, u32> = HashMap::with_capacity(order.len());
        for (row, &i) in order.iter().enumerate() {
            row_of.insert(g.items[i].id, row as u32);
        }
        let optima = order.iter().map(|&i| g.items[i].opt.clone()).collect();
        let mut edges: Vec<Edge> = g
            .edges
            .iter()
            .filter_map(|(&(a, b), &(count, kick, strength))| {
                Some(Edge { from: *row_of.get(&a)?, to: *row_of.get(&b)?, count, kick, strength })
            })
            .collect();
        edges.sort_by_key(|e| (e.from, e.to));
        Snapshot { optima, edges, hits: g.hits, pruned: g.pruned }
    }
}

impl Observer for Archive<'_> {
    fn optimum(&self, st: &State, constrained: bool) -> u32 {
        let sym = canonical(&st.sym, self.mirror.as_deref());
        let h = fnv1a(&sym);
        {
            let mut g = self.inner.lock().unwrap();
            g.hits += 1;
            if let Some(&i) = g.index.get(&h) {
                let it = &mut g.items[i];
                it.opt.hits += 1;
                if !constrained {
                    it.opt.flags &= !CONSTRAINED;
                }
                return it.id;
            }
        }
        // New: measure outside the lock (a full breakdown costs ~0.3 ms). Re-score from scratch so
        // stored scores carry no accumulated delta drift.
        let cst = self.sc.state_from_sym(sym.clone());
        let raw = raw_vector(self.sc, &cst);
        let first_ms = self.t0.elapsed().as_millis().min(u32::MAX as u128) as u32;
        let mut g = self.inner.lock().unwrap();
        if let Some(&i) = g.index.get(&h) {
            g.items[i].opt.hits += 1;
            return g.items[i].id;
        }
        let id = g.next_id;
        g.next_id += 1;
        let opt = Optimum { sym, score: cst.score, raw, hits: 1, first_ms, flags: if constrained { CONSTRAINED } else { 0 } };
        let i = g.items.len();
        g.items.push(Item { id, opt });
        g.index.insert(h, i);
        if g.items.len() > self.max + self.max / 5 {
            Self::prune(&mut g, self.max);
        }
        id
    }

    fn edge(&self, from: u32, to: u32, kick: Kick, strength: u8) {
        let mut g = self.inner.lock().unwrap();
        let e = g.edges.entry((from, to)).or_insert((0, kick as u8, strength));
        e.0 += 1;
    }

    fn exclusions(&self) -> Arc<Vec<Vec<u8>>> {
        let mut ex = self.excl.lock().unwrap();
        if ex.0.is_some_and(|t| t.elapsed().as_millis() < 500) {
            return ex.1.clone();
        }
        // The best layouts that are pairwise far apart (funnel bottoms, roughly).
        let mut cands: Vec<(f64, Vec<u8>)> = {
            let g = self.inner.lock().unwrap();
            let mut v: Vec<(f64, usize)> =
                g.items.iter().enumerate().filter(|(_, it)| it.opt.flags & CONSTRAINED == 0).map(|(i, it)| (it.opt.score, i)).collect();
            let k = 2000.min(v.len());
            if k < v.len() {
                v.select_nth_unstable_by(k, |a, b| a.0.total_cmp(&b.0));
                v.truncate(k);
            }
            v.into_iter().map(|(s, i)| (s, g.items[i].opt.sym.clone())).collect()
        };
        cands.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut picked: Vec<Vec<u8>> = Vec::new();
        for (_, s) in cands {
            if picked.len() >= self.exclusion_k {
                break;
            }
            if picked.iter().all(|p| hamming(p, &s) >= self.exclusion_dist) {
                picked.push(s);
            }
        }
        if let Some(m) = &self.mirror {
            let mirrored: Vec<Vec<u8>> = picked.iter().map(|s| mirror(s, m)).collect();
            picked.extend(mirrored);
        }
        *ex = (Some(Instant::now()), Arc::new(picked));
        ex.1.clone()
    }
}
