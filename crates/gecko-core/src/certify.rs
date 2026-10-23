//! Exact optimum of a sub-problem: all symbols but a few free ones stay on their slots, and a
//! branch and bound proves which placement of the free symbols is best (B7). Method and
//! results: `context/algorithms/bounds.md` ("Certified sub-problems").
//!
//! One free symbol is placed per level (most frequent first). Each node is bounded by
//! Gilmore–Lawler on the dual-ascent rewrite of the QAP form (`bound::ascent`): pinned and
//! placed symbols are constants, so the bound is `placed cost + GL(remaining)`. Leaves are scored
//! exactly: `Kind::Full` keeps a real [`State`] up to date with swap deltas (`Scorer::delta`),
//! `Kind::Pair` sums the QAP form, which is exact for the pair part.

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::bound::{self, Lap};
use crate::run::Input;
use crate::{hash, Error, Result, Scorer, State, SCHEMA};

/// At most this many free symbols (the dual ascent stores m⁴ pair costs; slots are a u64 mask).
pub const MAX_FREE: usize = bound::MAX_SYMBOLS;

/// Which objective a certificate is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// The real objective (`Scorer::full`): trigrams and load terms included.
    Full,
    /// Only the mono + bigram/skipgram part (`Scorer::parts().0`). Not a certificate of the score.
    Pair,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Full => "full",
            Kind::Pair => "pair",
        }
    }

    pub fn parse(s: &str) -> Result<Kind> {
        match s {
            "full" => Ok(Kind::Full),
            "pair" => Ok(Kind::Pair),
            _ => Err(Error::invalid("kind", format!("'{s}' (expected full or pair)"))),
        }
    }

    /// The layout's value of this objective.
    pub fn value(self, sc: &Scorer, st: &State) -> f64 {
        match self {
            Kind::Full => st.score,
            Kind::Pair => sc.parts(st).0,
        }
    }
}

/// Stop early; the result is then a best-so-far plus a lower bound, not a certificate.
#[derive(Clone, Copy, Debug, Default)]
pub struct Limits {
    pub time: Option<Duration>,
    pub nodes: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Certificate {
    pub kind: Kind,
    /// Free symbols in branching order (most frequent first).
    pub free: String,
    /// The start layout's value of `kind`'s objective.
    pub start: f64,
    /// Best value found over all placements of the free symbols (the optimum when `certified`).
    pub optimum: f64,
    /// No placement scores below this. Equals `optimum` when `certified`.
    pub lower: f64,
    /// The search finished: `optimum` is exact (to `tol`).
    pub certified: bool,
    /// `optimum` is below `start` by more than `tol`.
    pub improved: bool,
    /// Improvements smaller than this are treated as ties (1e-9 relative).
    pub tol: f64,
    /// The best layout, slot → symbol id.
    pub sym: Vec<u8>,
    /// Gilmore–Lawler bound at the root, after dual ascent.
    pub root_bound: f64,
    /// Placements to try: k!.
    pub space: f64,
    pub nodes: u64,
    pub leaves: u64,
    /// Subtrees cut by the bound.
    pub pruned: u64,
    /// Share of the k! placements cut by the bound (%).
    pub pruned_pct: f64,
    pub elapsed_ms: u64,
    /// `time` or `nodes` when a limit stopped the search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stopped: Option<String>,
}

/// The `k` most frequent symbols of `movable` (ties: lower symbol id). The default free set.
pub fn default_free(sc: &Scorer, movable: &[usize], k: usize) -> Vec<usize> {
    let mut v = movable.to_vec();
    v.sort_by(|&a, &b| sc.f1()[b].total_cmp(&sc.f1()[a]).then(a.cmp(&b)));
    v.truncate(k);
    v
}

/// Best placement of the `free` symbols among their slots in `st`; every other symbol stays put.
pub fn certify(sc: &Scorer, st: &State, free: &[usize], kind: Kind, limits: Limits) -> Result<Certificate> {
    let t0 = Instant::now();
    let m = free.len();
    if m > MAX_FREE {
        return Err(Error::invalid("free", format!("{m} free symbols (at most {MAX_FREE})")));
    }
    let mut seen = vec![false; sc.n];
    for &x in free {
        if x >= sc.n || std::mem::replace(&mut seen[x], true) {
            return Err(Error::invalid("free", "free symbols must be distinct symbols of the layout"));
        }
    }
    // Branch on frequent symbols first: their placement fixes the most cost.
    let mut order = free.to_vec();
    order.sort_by(|&a, &b| sc.f1()[b].total_cmp(&sc.f1()[a]).then(a.cmp(&b)));
    let terms =
        sc.qap_terms(kind == Kind::Full).ok_or_else(|| Error::invalid("certify", "a load metric has a negative weight (no bound)"))?;
    let asc = bound::ascent(&terms, &order, &st.pos, bound::ASCENT_ITERS);
    let start = kind.value(sc, st);
    let tol = 1e-9 * start.abs().max(1.0);
    let mut s = Search::new(sc, st, &order, kind, &asc, start, tol, limits, t0);
    s.dfs(0);
    // Stopped at depth D: every unexplored placement lies under a node on the current path,
    // so the smallest bound on that path (or the root bound) bounds them all.
    let lower = s.bound_at[..s.stop_depth].iter().copied().fold(s.best.min(s.root_bound), f64::min);
    let mut sym = st.sym.clone();
    for a in 0..m {
        sym[s.slots[s.best_at[a]]] = order[a] as u8;
    }
    let best = sc.state_from_sym(sym);
    let optimum = kind.value(sc, &best);
    let space: f64 = (1..=m).map(|i| i as f64).product();
    Ok(Certificate {
        kind,
        free: order.iter().map(|&x| sc.chars[x]).collect(),
        start,
        optimum,
        lower: if s.stopped.is_some() { lower } else { optimum },
        certified: s.stopped.is_none(),
        improved: optimum < start - tol,
        tol,
        sym: best.sym,
        root_bound: s.root_bound,
        space,
        nodes: s.nodes,
        leaves: s.leaves,
        pruned: s.pruned,
        pruned_pct: 100.0 * s.cut / space,
        elapsed_ms: t0.elapsed().as_millis() as u64,
        stopped: s.stopped.map(str::to_string),
    })
}

/// A certificate for a run's best layout, as written to `runs/<id>/certify-<kind>.json`.
/// Everything needed to repeat it: the run (its seed and inputs), the free set and the limits.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunCertificate {
    pub schema: u32,
    pub run: String,
    pub created: String,
    pub gecko_version: String,
    /// The run's seed (the certificate itself has no randomness).
    pub seed: u64,
    pub geometry: Input,
    pub corpus: Input,
    pub profile: String,
    pub profile_hash: String,
    /// The run's pinned symbols (never free).
    pub pins: String,
    /// The run's best layout (slot order), the start of the certificate.
    pub layout: String,
    /// What the certificate covers: the real objective, or only the pair part.
    pub covers: String,
    pub limits: LimitsJson,
    #[serde(flatten)]
    pub cert: Certificate,
    /// The best layout found (slot order) and its rows.
    pub chars: String,
    pub rows: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LimitsJson {
    pub time_ms: Option<u64>,
    pub nodes: Option<u64>,
}

/// Which symbols to free in [`certify_run`].
pub enum FreeSpec {
    /// The `k` most frequent unpinned symbols ([`default_free`]).
    Count(usize),
    Symbols(Vec<char>),
}

/// Certify the best layout of run directory `dir`. Its inputs are loaded by name from `store`
/// and must hash as recorded.
pub fn certify_run(
    store: &crate::store::Store,
    dir: &std::path::Path,
    free: &FreeSpec,
    kind: Kind,
    limits: Limits,
) -> Result<RunCertificate> {
    if crate::geomsearch::is_run(dir) {
        return Err(Error::invalid("certify", "a geometry run: certify needs a fixed board (use a layout run)"));
    }
    let config = crate::run::load_config(dir)?;
    let geometry = store.geometry(&config.geometry.name)?;
    let corpus = store.corpus(&config.corpus.name)?;
    for (what, recorded, now) in [
        ("geometry", &config.geometry.hash, hash::hex(geometry.hash())),
        ("corpus", &config.corpus.hash, hash::hex(corpus.hash())),
        ("profile", &config.profile_hash, hash::hex(config.profile.hash())),
    ] {
        if *recorded != now {
            return Err(Error::invalid("certify", format!("the run's {what} changed since it ran (hash {recorded}, now {now})")));
        }
    }
    let layout = store.layout(&format!("run:{}", config.id), &geometry.name)?.fit(&geometry)?;
    // Symbol ids follow the best layout's slots, so the identity state is that layout.
    let sc = Scorer::new(&geometry, &corpus, &config.profile, &layout)?;
    let st = sc.identity();
    let pinned = crate::layout::parse_symbols(&config.pins)?;
    let movable: Vec<usize> = (0..sc.n).filter(|&x| !pinned.contains(&sc.chars[x])).collect();
    let ids = match free {
        FreeSpec::Count(k) => default_free(&sc, &movable, *k),
        FreeSpec::Symbols(cs) => cs
            .iter()
            .map(|c| {
                let x =
                    sc.chars.iter().position(|y| y == c).ok_or_else(|| Error::invalid("symbols", format!("'{c}' is not on the layout")))?;
                if pinned.contains(c) {
                    return Err(Error::invalid("symbols", format!("'{c}' is pinned in the run")));
                }
                Ok(x)
            })
            .collect::<Result<Vec<_>>>()?,
    };
    let cert = certify(&sc, &st, &ids, kind, limits)?;
    let best = sc.layout_of(&sc.state_from_sym(cert.sym.clone()), "certified")?;
    Ok(RunCertificate {
        schema: SCHEMA,
        run: config.id.clone(),
        created: crate::run::now_stamp().1,
        gecko_version: env!("CARGO_PKG_VERSION").to_string(),
        seed: config.params.seed,
        geometry: config.geometry.clone(),
        corpus: config.corpus.clone(),
        profile: config.profile.name.clone(),
        profile_hash: config.profile_hash.clone(),
        pins: config.pins.clone(),
        layout: st.chars(&sc).into_iter().collect(),
        covers: match kind {
            Kind::Full => "the real objective (Scorer::full)".into(),
            Kind::Pair => "only the mono + bigram/skipgram part (Scorer::parts().0); not the run's score".into(),
        },
        limits: LimitsJson { time_ms: limits.time.map(|t| t.as_millis() as u64), nodes: limits.nodes },
        chars: cert.sym.iter().map(|&x| sc.chars[x as usize]).collect(),
        rows: best.all_rows(&geometry),
        cert,
    })
}

/// Depth-first search state. Every buffer is allocated once in `new`; `dfs` does not allocate.
struct Search<'a> {
    sc: &'a Scorer,
    kind: Kind,
    m: usize,
    /// Free symbol ids in branching order, and the free slots (slot index s → board slot).
    order: &'a [usize],
    slots: Vec<usize>,
    /// Exact rewrite (`bound::Ascent`): value = k + Σ lin[a, s_a] + Σ_{a<b} w[a, s_a, b, s_b],
    /// `w` symmetric; `lin` and `k` seed `cur[0]` and `g[0]`.
    w: Vec<f64>,
    /// Per depth: linear cost of each unplaced symbol on each slot given the placed ones
    /// (`cur[d·m² + a·m + s]`), cost of the placed ones (`g[d]`), and the node's bound.
    cur: Vec<f64>,
    g: Vec<f64>,
    bound_at: Vec<f64>,
    /// Candidate slots per depth, cheapest first.
    cand: Vec<usize>,
    slot_of: Vec<usize>,
    best_at: Vec<usize>,
    used: u64,
    inner: Vec<f64>,
    outer: Vec<f64>,
    rows: Vec<usize>,
    cols: Vec<usize>,
    lap: Lap,
    /// The real layout (`Kind::Full` only), kept in step with `slot_of` by swaps.
    st: State,
    best: f64,
    tol: f64,
    root_bound: f64,
    /// m! for each m (subtree sizes).
    fact: Vec<f64>,
    nodes: u64,
    leaves: u64,
    pruned: u64,
    cut: f64,
    limits: Limits,
    t0: Instant,
    stopped: Option<&'static str>,
    stop_depth: usize,
}

impl<'a> Search<'a> {
    #[allow(clippy::too_many_arguments)]
    fn new(
        sc: &'a Scorer,
        st: &State,
        order: &'a [usize],
        kind: Kind,
        asc: &bound::Ascent,
        start: f64,
        tol: f64,
        limits: Limits,
        t0: Instant,
    ) -> Self {
        let m = order.len();
        let idx = |a: usize, s: usize, b: usize, u: usize| ((a * m + s) * m + b) * m + u;
        let mut w = vec![0.0; m * m * m * m];
        for a in 0..m {
            for b in 0..m {
                for s in 0..m {
                    for u in 0..m {
                        if a != b && s != u {
                            w[idx(a, s, b, u)] = asc.r[idx(a, s, b, u)] + asc.r[idx(b, u, a, s)];
                        }
                    }
                }
            }
        }
        let mut cur = vec![0.0; (m + 1) * m * m];
        cur[..m * m].copy_from_slice(&asc.lin);
        let mut g = vec![0.0; m + 1];
        g[0] = asc.k;
        let mut fact = vec![1.0; m + 1];
        for i in 1..=m {
            fact[i] = fact[i - 1] * i as f64;
        }
        Search {
            sc,
            kind,
            m,
            order,
            slots: order.iter().map(|&x| st.pos[x] as usize).collect(),
            w,
            cur,
            g,
            bound_at: vec![f64::INFINITY; m + 1],
            cand: vec![0; m * m],
            slot_of: vec![0; m],
            // The start layout is the first incumbent: symbol a on its own slot a.
            best_at: (0..m).collect(),
            used: 0,
            inner: vec![0.0; m * m],
            outer: vec![0.0; m * m],
            rows: vec![0; m],
            cols: vec![0; m],
            lap: Lap::new(m.max(1)),
            st: st.clone(),
            best: start,
            tol,
            root_bound: asc.lower,
            fact,
            nodes: 0,
            leaves: 0,
            pruned: 0,
            cut: 0.0,
            limits,
            t0,
            stopped: None,
            stop_depth: 0,
        }
    }

    #[inline]
    fn w(&self, a: usize, s: usize, b: usize, u: usize) -> f64 {
        let m = self.m;
        self.w[((a * m + s) * m + b) * m + u]
    }

    /// Gilmore–Lawler bound on the cost still to come at depth `d` (symbols d.. unplaced).
    fn remaining_bound(&mut self, d: usize) -> f64 {
        let (m, r) = (self.m, self.m - d);
        let mut nc = 0;
        for s in 0..m {
            if self.used & (1 << s) == 0 {
                self.cols[nc] = s;
                nc += 1;
            }
        }
        for i in 0..r {
            self.rows[i] = d + i;
        }
        let base = d * m * m;
        for i in 0..r {
            let b = self.rows[i];
            for j in 0..r {
                let u = self.cols[j];
                let mut c = 0;
                for ii in (0..r).filter(|&ii| ii != i) {
                    for jj in (0..r).filter(|&jj| jj != j) {
                        self.inner[c] = 0.5 * self.w(b, u, self.rows[ii], self.cols[jj]);
                        c += 1;
                    }
                }
                let pair = if r > 1 { self.lap.solve(r - 1, &self.inner) } else { 0.0 };
                self.outer[i * r + j] = self.cur[base + b * m + u] + pair;
            }
        }
        self.lap.solve(r, &self.outer)
    }

    fn dfs(&mut self, d: usize) {
        let m = self.m;
        self.nodes += 1;
        // The clock is read every 4096 nodes, and at every node high in the tree, where one
        // bound costs O((m − d)⁵) (milliseconds at 30 free symbols).
        let clock = self.nodes.is_multiple_of(4096) || m - d >= 8;
        if clock || self.limits.nodes.is_some() {
            let over_nodes = self.limits.nodes.is_some_and(|n| self.nodes > n);
            let over_time = clock && self.limits.time.is_some_and(|t| self.t0.elapsed() >= t);
            if over_nodes || over_time {
                self.stopped = Some(if over_nodes { "nodes" } else { "time" });
                self.stop_depth = d;
                return;
            }
        }
        if d == m {
            self.leaves += 1;
            let v = match self.kind {
                Kind::Full => self.st.score,
                Kind::Pair => self.g[d],
            };
            if v < self.best - self.tol {
                self.best = v;
                self.best_at.copy_from_slice(&self.slot_of);
            }
            return;
        }
        if m - d >= 2 {
            let b = self.g[d] + self.remaining_bound(d);
            if d == 0 {
                self.root_bound = self.root_bound.max(b);
            }
            self.bound_at[d] = b;
            if b >= self.best - self.tol {
                self.pruned += 1;
                self.cut += self.fact[m - d];
                return;
            }
        } else {
            self.bound_at[d] = f64::INFINITY;
        }
        // Candidate slots for symbol d, cheapest linear cost first (insertion sort).
        let base = d * m * m;
        let mut nc = 0;
        for s in 0..m {
            if self.used & (1 << s) != 0 {
                continue;
            }
            let c = self.cur[base + d * m + s];
            let mut i = nc;
            while i > 0 && self.cur[base + d * m + self.cand[d * m + i - 1]] > c {
                self.cand[d * m + i] = self.cand[d * m + i - 1];
                i -= 1;
            }
            self.cand[d * m + i] = s;
            nc += 1;
        }
        let x = self.order[d];
        for ci in 0..nc {
            let s = self.cand[d * m + ci];
            // Child: symbol d on slot s.
            self.g[d + 1] = self.g[d] + self.cur[base + d * m + s];
            let next = (d + 1) * m * m;
            for b in d + 1..m {
                for u in 0..m {
                    if u != s && self.used & (1 << u) == 0 {
                        self.cur[next + b * m + u] = self.cur[base + b * m + u] + self.w(d, s, b, u);
                    }
                }
            }
            // Keep the real layout in step: swap x with the free symbol now on slot s.
            let y = self.st.sym[self.slots[s]] as usize;
            let saved = (self.st.score, self.st.load);
            if self.kind == Kind::Full && y != x {
                let dl = self.sc.delta(&self.st, x, y);
                self.sc.apply(&mut self.st, x, y, dl);
            }
            self.slot_of[d] = s;
            self.used |= 1 << s;
            self.dfs(d + 1);
            self.used &= !(1 << s);
            if self.kind == Kind::Full && y != x {
                self.sc.apply(&mut self.st, x, y, 0.0);
                (self.st.score, self.st.load) = saved;
            }
            if self.stopped.is_some() {
                return;
            }
        }
    }
}
