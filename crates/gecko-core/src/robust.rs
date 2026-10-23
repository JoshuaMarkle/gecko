//! How much a profile's conclusions depend on its weights: unit metric costs (a layout's score is
//! linear in the weights), rank confidence under log-normal weight noise, and weight sweeps that
//! re-optimize under perturbed profiles. See `context/objective.md` (Robustness) and
//! `research/experiments/2026-10-22-cost-function-validity.md`.

use std::time::Duration;

use serde::Serialize;

use crate::calibrate::{costs, unit_profile};
use crate::metrics::{Dir, METRICS};
use crate::optimize::{self, Algo, Control, Params};
use crate::rng::Rng;
use crate::{Corpus, Error, Geometry, Layout, Profile, Result, Scorer};

/// Unit costs of many layouts: scorers with every metric at weight 1 and no scales, one per
/// symbol set seen (layouts with the same symbols reuse it).
pub struct UnitCosts<'a> {
    g: &'a Geometry,
    corpus: &'a Corpus,
    unit: Profile,
    scorers: Vec<Scorer>,
}

impl<'a> UnitCosts<'a> {
    /// `profile` supplies the table parameters (row offsets, hand, `sfb`, …); its weights are ignored.
    pub fn new(g: &'a Geometry, corpus: &'a Corpus, profile: &Profile) -> Self {
        UnitCosts { g, corpus, unit: unit_profile(profile), scorers: Vec::new() }
    }

    /// Each metric's signed cost at unit weight, aligned with [`METRICS`]: the score under any
    /// profile with the same tables is [`score_of`] these costs (rewards are negative; info 0).
    pub fn of(&mut self, layout: &Layout) -> Result<Vec<f64>> {
        for sc in &self.scorers {
            if let Ok(st) = sc.state_for(layout) {
                return Ok(costs(sc, &st));
            }
        }
        let sc = Scorer::new(self.g, self.corpus, &self.unit, layout)?;
        let c = costs(&sc, &sc.identity());
        self.scorers.push(sc);
        Ok(c)
    }
}

/// A layout's score under `p` from its unit costs ([`UnitCosts::of`]); equal to the scorer's
/// score when `p` has the tables the costs were measured with.
pub fn score_of(costs: &[f64], p: &Profile) -> f64 {
    METRICS.iter().zip(costs).map(|(d, c)| p.effective_weight(d.id) as f64 * c).sum()
}

/// Standard normal draw (Box–Muller).
fn normal(rng: &mut Rng) -> f64 {
    let u = 1.0 - rng.f64();
    (-2.0 * u.ln()).sqrt() * (std::f64::consts::TAU * rng.f64()).cos()
}

/// `p` with every nonzero weight multiplied by `exp(σ·z)`, z standard normal (scales kept).
pub fn jitter(p: &Profile, sigma: f64, rng: &mut Rng) -> Profile {
    let mut q = p.clone();
    for w in q.weights.values_mut() {
        if *w != 0.0 {
            *w *= (sigma * normal(rng)).exp() as f32;
        }
    }
    q
}

/// One layout's rank under weight noise.
#[derive(Clone, Debug, Serialize)]
pub struct RankStats {
    pub layout: String,
    pub score: f64,
    /// Rank under the unperturbed profile (1 = best).
    pub rank: usize,
    /// Share of draws in which it ranks first.
    pub p_first: f64,
    /// Share of draws in which it keeps its unperturbed rank.
    pub p_rank: f64,
    /// Share of draws in which it beats the next layout of the unperturbed order (none for the last).
    pub p_beats_next: Option<f64>,
    pub mean_rank: f64,
    /// 5th and 95th percentile rank over the draws.
    pub rank_lo: usize,
    pub rank_hi: usize,
}

/// Rank confidence of `names[i]` (unit costs `costs[i]`) under `draws` log-normal perturbations
/// of `p`'s weights (σ = `sigma`). Rows come back in unperturbed rank order.
pub fn rank_confidence(names: &[String], costs: &[Vec<f64>], p: &Profile, sigma: f64, draws: usize, seed: u64) -> Vec<RankStats> {
    let n = names.len();
    let base: Vec<f64> = costs.iter().map(|c| score_of(c, p)).collect();
    let order = argsort(&base);
    let mut ranks = vec![Vec::with_capacity(draws); n];
    let mut beats = vec![0usize; n];
    let mut rng = Rng::new(seed);
    for _ in 0..draws {
        let q = jitter(p, sigma, &mut rng);
        let s: Vec<f64> = costs.iter().map(|c| score_of(c, &q)).collect();
        for (r, &i) in argsort(&s).iter().enumerate() {
            ranks[i].push(r + 1);
        }
        for w in order.windows(2) {
            if s[w[0]] < s[w[1]] {
                beats[w[0]] += 1;
            }
        }
    }
    let d = draws.max(1) as f64;
    order
        .iter()
        .enumerate()
        .map(|(r, &i)| {
            let mut rk = ranks[i].clone();
            rk.sort_unstable();
            let pct = |q: f64| rk.get(((rk.len().saturating_sub(1)) as f64 * q).round() as usize).copied().unwrap_or(r + 1);
            RankStats {
                layout: names[i].clone(),
                score: base[i],
                rank: r + 1,
                p_first: rk.iter().filter(|&&x| x == 1).count() as f64 / d,
                p_rank: rk.iter().filter(|&&x| x == r + 1).count() as f64 / d,
                p_beats_next: (r + 1 < n).then(|| beats[i] as f64 / d),
                mean_rank: rk.iter().sum::<usize>() as f64 / d,
                rank_lo: pct(0.05),
                rank_hi: pct(0.95),
            }
        })
        .collect()
}

fn argsort(v: &[f64]) -> Vec<usize> {
    let mut o: Vec<usize> = (0..v.len()).collect();
    o.sort_by(|&a, &b| v[a].total_cmp(&v[b]));
    o
}

/// Each slot's left–right mirror on `g` (same layer and row, mirrored column and finger), or
/// `None` when the board is not symmetric.
pub fn mirror_slots(g: &Geometry) -> Option<Vec<usize>> {
    let k = &g.key;
    (0..k.len())
        .map(|i| {
            let row: Vec<usize> = (0..k.len()).filter(|&j| k[j].layer == k[i].layer && k[j].row == k[i].row).collect();
            let (lo, hi) = (row.iter().map(|&j| k[j].col).min()?, row.iter().map(|&j| k[j].col).max()?);
            row.into_iter().find(|&j| k[j].col == lo + hi - k[i].col && k[j].finger.index() == 9 - k[i].finger.index())
        })
        .collect()
}

/// Keys on which layouts `a` and `b` (symbols in slot order) differ, also comparing `a` with
/// `b` mirrored when `mirror` is given; the smaller count.
pub fn key_distance(a: &[char], b: &[char], mirror: Option<&[usize]>) -> usize {
    let plain = a.iter().zip(b).filter(|(x, y)| x != y).count();
    let mirrored = mirror.map_or(usize::MAX, |m| a.iter().enumerate().filter(|&(s, x)| *x != b[m[s]]).count());
    plain.min(mirrored)
}

/// Single-linkage families: layouts within `keys` keys of some member join its family.
/// Returns the family index of each layout, families numbered by first member.
pub fn families(layouts: &[Vec<char>], keys: usize, mirror: Option<&[usize]>) -> Vec<usize> {
    let n = layouts.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn root(p: &mut [usize], mut i: usize) -> usize {
        while p[i] != i {
            p[i] = p[p[i]];
            i = p[i];
        }
        i
    }
    for i in 0..n {
        for j in i + 1..n {
            if key_distance(&layouts[i], &layouts[j], mirror) <= keys {
                let (a, b) = (root(&mut parent, i), root(&mut parent, j));
                parent[a.max(b)] = a.min(b);
            }
        }
    }
    let mut id = vec![usize::MAX; n];
    let mut next = 0;
    let mut out = vec![0; n];
    for i in 0..n {
        let r = root(&mut parent, i);
        if id[r] == usize::MAX {
            id[r] = next;
            next += 1;
        }
        out[i] = id[r];
    }
    out
}

/// Settings of a weight sweep.
#[derive(Clone, Debug, Serialize)]
pub struct SweepSpec {
    /// Log-normal σ of the random draws.
    pub sigma: f64,
    /// Random weight vectors (besides drop / half / double of each weight).
    pub draws: usize,
    pub seed: u64,
    /// SA evaluations per search.
    pub evals: u64,
    /// SA seeds per weight vector; the best result is kept.
    pub seeds: u64,
    pub threads: usize,
    /// Family threshold: layouts within this many keys (mirror-invariant) share a family.
    pub family_keys: usize,
}

/// One weight vector's optimum.
#[derive(Clone, Debug, Serialize)]
pub struct SweepRow {
    /// `base`, `<id>:drop`, `<id>:half`, `<id>:double` or `draw:<k>`.
    pub variant: String,
    pub weights: std::collections::BTreeMap<String, f32>,
    pub chars: String,
    pub family: usize,
    /// Keys differing from the base optimum (mirror-invariant).
    pub keys_moved: usize,
    /// The base optimum's score minus this optimum's score, both under this weight vector: what
    /// keeping the base optimum costs if these weights were right (≥ 0 up to search noise).
    pub regret: f64,
    /// This optimum's score minus the base optimum's, both under the base profile.
    pub base_cost: f64,
    /// Score under the base profile.
    pub score: f64,
    /// Judges: E01 typing time, E05 Engram discomfort, and the raw B01 / M05 values.
    pub judges: std::collections::BTreeMap<String, f64>,
}

/// One weighted metric's effect on the optimum.
#[derive(Clone, Debug, Serialize)]
pub struct Sensitivity {
    pub id: String,
    pub weight: f32,
    /// Keys moved and regret when the weight is dropped, halved and doubled.
    pub keys_moved: [usize; 3],
    pub regret: [f64; 3],
}

#[derive(Clone, Debug, Serialize)]
pub struct FamilySummary {
    pub family: usize,
    /// Weight vectors whose optimum is in this family.
    pub count: usize,
    /// Its best member under the base profile.
    pub best: String,
    pub best_score: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct SweepReport {
    pub spec: SweepSpec,
    pub profile: String,
    pub rows: Vec<SweepRow>,
    pub families: Vec<FamilySummary>,
    pub sensitivity: Vec<Sensitivity>,
    /// Median and 90th percentile regret and keys moved over the random draws.
    pub draw_regret: [f64; 2],
    pub draw_keys_moved: [usize; 2],
}

/// The judges reported for every optimum: E01, E05, and raw B01 and M05 values.
pub const JUDGES: [&str; 4] = ["E01", "E05", "B01", "M05"];

/// Re-optimize under `p` and under perturbed weight vectors (each weight dropped, halved,
/// doubled; `spec.draws` log-normal draws), then compare the optima: families, regret, keys
/// moved, judges. `movable` slots of `start` may move. `progress(done, total)` after each search.
pub fn sweep(
    g: &Geometry,
    corpus: &Corpus,
    p: &Profile,
    start: &Layout,
    movable: &[usize],
    spec: &SweepSpec,
    mut progress: impl FnMut(usize, usize),
) -> Result<SweepReport> {
    if spec.evals == 0 || spec.seeds == 0 {
        return Err(Error::invalid("sweep", "evals and seeds must be positive"));
    }
    let weighted: Vec<(String, f32)> = p.weights.iter().filter(|(_, &w)| w != 0.0).map(|(k, &w)| (k.clone(), w)).collect();
    let mut variants: Vec<(String, Profile)> = vec![("base".into(), p.clone())];
    for (id, w) in &weighted {
        for (tag, f) in [("drop", 0.0), ("half", 0.5), ("double", 2.0)] {
            let mut q = p.clone();
            q.weights.insert(id.clone(), w * f);
            variants.push((format!("{id}:{tag}"), q));
        }
    }
    let mut rng = Rng::new(spec.seed);
    for k in 0..spec.draws {
        variants.push((format!("draw:{}", k + 1), jitter(p, spec.sigma, &mut rng)));
    }
    let total = variants.len();
    let mut unit = UnitCosts::new(g, corpus, p);
    let mut found: Vec<(Vec<char>, Vec<f64>)> = Vec::with_capacity(total);
    let mut mirror = None;
    for (k, (_, q)) in variants.iter().enumerate() {
        let sc = Scorer::new(g, corpus, q, start)?;
        let ctx = optimize::Ctx::new(&sc, movable.to_vec(), sc.identity());
        let mut best: Option<optimize::Scored> = None;
        for s in 0..spec.seeds {
            let params = Params {
                algo: Algo::Sa,
                seed: spec.seed + s,
                threads: spec.threads,
                time_ms: None,
                evals: Some(spec.evals),
                top: 1,
                ..Default::default()
            };
            let o = optimize::run(&ctx, &params, Duration::from_secs(3600), &Control::default(), |_| {});
            if let Some(t) = o.top.into_iter().next() {
                if best.as_ref().is_none_or(|b| t.score < b.score) {
                    best = Some(t);
                }
            }
        }
        let best = best.ok_or_else(|| Error::invalid("sweep", "search returned no layout"))?;
        let st = sc.state_from_sym(best.sym);
        let costs = unit.of(&sc.layout_of(&st, "sweep")?)?;
        found.push((st.chars(&sc), costs));
        if k == 0 {
            mirror = mirror_slots(&sc.geometry);
        }
        progress(k + 1, total);
    }
    let fam = families(&found.iter().map(|f| f.0.clone()).collect::<Vec<_>>(), spec.family_keys, mirror.as_deref());
    let (base_chars, base_costs) = (&found[0].0, &found[0].1);
    let base_score = score_of(base_costs, p);
    let judge_idx: Vec<(String, usize)> =
        JUDGES.iter().filter_map(|id| crate::metrics::index_of(id).map(|i| (id.to_string(), i))).collect();
    let rows: Vec<SweepRow> = variants
        .iter()
        .zip(&found)
        .zip(&fam)
        .map(|(((name, q), (chars, c)), &family)| {
            let judges = judge_idx
                .iter()
                .map(|(id, i)| {
                    let v = if METRICS[*i].dir == Dir::Max { -c[*i] } else { c[*i] };
                    (id.clone(), v)
                })
                .collect();
            SweepRow {
                variant: name.clone(),
                weights: q.weights.clone(),
                chars: chars.iter().collect(),
                family,
                keys_moved: key_distance(chars, base_chars, mirror.as_deref()),
                regret: score_of(base_costs, q) - score_of(c, q),
                base_cost: score_of(c, p) - base_score,
                score: score_of(c, p),
                judges,
            }
        })
        .collect();
    let sensitivity = weighted
        .iter()
        .map(|(id, w)| {
            let row = |tag: &str| rows.iter().find(|r| r.variant == format!("{id}:{tag}")).expect("variant row");
            let r = [row("drop"), row("half"), row("double")];
            Sensitivity { id: id.clone(), weight: *w, keys_moved: r.map(|x| x.keys_moved), regret: r.map(|x| x.regret) }
        })
        .collect();
    let nf = fam.iter().max().map_or(0, |m| m + 1);
    let mut families: Vec<FamilySummary> = (0..nf)
        .map(|f| {
            let members: Vec<&SweepRow> = rows.iter().filter(|r| r.family == f).collect();
            let b = members.iter().min_by(|a, b| a.score.total_cmp(&b.score)).expect("nonempty family");
            FamilySummary { family: f, count: members.len(), best: b.chars.clone(), best_score: b.score }
        })
        .collect();
    families.sort_by(|a, b| b.count.cmp(&a.count).then(a.best_score.total_cmp(&b.best_score)));
    let draws: Vec<&SweepRow> = rows.iter().filter(|r| r.variant.starts_with("draw:")).collect();
    let quant = |mut v: Vec<f64>| -> [f64; 2] {
        v.sort_by(f64::total_cmp);
        let at = |q: f64| v.get(((v.len().saturating_sub(1)) as f64 * q).round() as usize).copied().unwrap_or(0.0);
        [at(0.5), at(0.9)]
    };
    let draw_regret = quant(draws.iter().map(|r| r.regret).collect());
    let km = quant(draws.iter().map(|r| r.keys_moved as f64).collect());
    Ok(SweepReport {
        spec: spec.clone(),
        profile: p.name.clone(),
        rows,
        families,
        sensitivity,
        draw_regret,
        draw_keys_moved: [km[0] as usize, km[1] as usize],
    })
}
