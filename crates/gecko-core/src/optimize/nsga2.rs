//! NSGA-II (Deb et al., 2002): non-dominated sorting, normalized crowding distance,
//! rank/crowding binary tournaments, OX + swap mutation, and elitist parent/offspring survival.
//! RWSA is a cheaper baseline: random-weight SA restarts followed by the same Pareto filter.
//! Every search swap uses the existing allocation-free delta/apply path. See nsga2.md.

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::{genetic, Algo, Control, Ctx, Outcome, Params, Progress, Scored};
use crate::metrics::{self, Dir, METRICS};
use crate::report::breakdown;
use crate::rng::Rng;
use crate::{Error, Result, Scorer, State};

const EPS: f64 = 1e-9;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct NsgaParams {
    pub objectives: Vec<String>,
    pub pop_size: usize,
    /// 0 = use the time/evaluation budget; otherwise also stop after this many generations.
    pub generations: usize,
    /// Maximum archive size; crowding preserves extremes when the archive fills.
    pub archive_size: usize,
    pub crossover_rate: f64,
    pub mutation_rate: f64,
}

impl Default for NsgaParams {
    fn default() -> Self {
        Self {
            objectives: vec!["B01".into(), "T03".into(), "M01".into()],
            pop_size: 128,
            generations: 0,
            archive_size: 512,
            crossover_rate: 0.9,
            mutation_rate: 1.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Objective {
    pub id: String,
    pub unit: String,
    /// Direction of the raw report value. Nonlinear target metrics instead use `cost` below.
    pub dir: String,
    /// Meaning of the minimized cost used for dominance (independent of profile weights).
    pub cost: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Point {
    pub sym: Vec<u8>,
    pub score: f64,
    /// Natural report values, in objective order.
    pub raw: Vec<f64>,
    /// Unit-weight metric costs, all minimized (max metrics negated; target penalties retained).
    pub costs: Vec<f64>,
}

pub fn validate(p: &NsgaParams) -> Result<()> {
    if !(2..=8).contains(&p.objectives.len()) {
        return Err(Error::invalid("objectives", "choose 2–8 distinct scored metric ids"));
    }
    for (i, id) in p.objectives.iter().enumerate() {
        let index = metrics::index_of(id).ok_or_else(|| Error::invalid("objectives", format!("unknown metric '{id}'")))?;
        if METRICS[index].dir == Dir::Info || p.objectives[..i].contains(id) {
            return Err(Error::invalid("objectives", format!("'{id}' must be a distinct scored metric")));
        }
    }
    if p.pop_size < 2
        || p.archive_size < 2
        || !p.crossover_rate.is_finite()
        || !(0.0..=1.0).contains(&p.crossover_rate)
        || !p.mutation_rate.is_finite()
        || !(0.0..=100.0).contains(&p.mutation_rate)
    {
        return Err(Error::invalid("nsga2", "population/archive >= 2, crossover in 0..1 and mutation in 0..100 are required"));
    }
    Ok(())
}

fn objectives(p: &NsgaParams) -> Vec<Objective> {
    p.objectives
        .iter()
        .map(|id| {
            let d = &METRICS[metrics::index_of(id).unwrap()];
            let cost = match id.as_str() {
                "M02" => "squared finger-load error",
                "M04" => "squared deviation of left share from 50%",
                "M09" => "squared deviation of thumb share from profile.thumb_target",
                _ if d.dir == Dir::Max => "negative raw value",
                _ => "raw value",
            };
            Objective { id: id.clone(), unit: d.unit.into(), dir: if d.dir == Dir::Max { "max" } else { "min" }.into(), cost: cost.into() }
        })
        .collect()
}

/// Strict Pareto dominance in minimized costs. The tolerance absorbs accumulated delta noise.
pub fn dominates(a: &[f64], b: &[f64]) -> bool {
    assert_eq!(a.len(), b.len());
    a.iter().zip(b).all(|(a, b)| a <= &(b + EPS)) && a.iter().zip(b).any(|(a, b)| a < &(b - EPS))
}

/// Fast non-dominated sorting and normalized crowding distance within each rank.
/// Equal/constant dimensions contribute zero crowding, so they do not favor arbitrary endpoints.
pub fn rank_and_crowding(costs: &[Vec<f64>]) -> (Vec<usize>, Vec<f64>) {
    let n = costs.len();
    let mut dominated = vec![Vec::new(); n];
    let mut counts = vec![0; n];
    for i in 0..n {
        for j in (i + 1)..n {
            if dominates(&costs[i], &costs[j]) {
                dominated[i].push(j);
                counts[j] += 1;
            } else if dominates(&costs[j], &costs[i]) {
                dominated[j].push(i);
                counts[i] += 1;
            }
        }
    }
    let mut rank = vec![0; n];
    let mut crowd = vec![0.0; n];
    let mut front: Vec<usize> = (0..n).filter(|&i| counts[i] == 0).collect();
    let mut level = 0;
    while !front.is_empty() {
        for dim in 0..costs[0].len() {
            let mut order = front.clone();
            order.sort_by(|&a, &b| costs[a][dim].total_cmp(&costs[b][dim]).then(a.cmp(&b)));
            let span = costs[*order.last().unwrap()][dim] - costs[order[0]][dim];
            if span <= EPS {
                continue;
            }
            crowd[order[0]] = f64::INFINITY;
            crowd[*order.last().unwrap()] = f64::INFINITY;
            for k in 1..order.len().saturating_sub(1) {
                crowd[order[k]] += (costs[order[k + 1]][dim] - costs[order[k - 1]][dim]) / span;
            }
        }
        let mut next = Vec::new();
        for i in front {
            rank[i] = level;
            for &j in &dominated[i] {
                counts[j] -= 1;
                if counts[j] == 0 {
                    next.push(j);
                }
            }
        }
        front = next;
        level += 1;
    }
    (rank, crowd)
}

#[derive(Clone)]
struct Individual {
    /// Objective states followed by the original profile state. Never cross scorer caches.
    states: Vec<State>,
    costs: Vec<f64>,
}

impl Individual {
    fn new(scorers: &[Scorer], sym: &[u8]) -> Self {
        let states: Vec<_> = scorers.iter().map(|sc| sc.state_from_sym(sym.to_vec())).collect();
        let costs = states[..states.len() - 1].iter().map(|st| st.score).collect();
        Self { states, costs }
    }
    fn sym(&self) -> &[u8] {
        &self.states[0].sym
    }
    fn score(&self) -> f64 {
        self.states.last().unwrap().score
    }
}

#[derive(Clone)]
struct Archived {
    sym: Vec<u8>,
    costs: Vec<f64>,
    score: f64,
}

#[derive(Default)]
struct Archive {
    members: Vec<Archived>,
}

impl Archive {
    fn offer(&mut self, point: &Individual, cap: usize) {
        if self.members.iter().any(|p| p.sym == point.sym() || dominates(&p.costs, &point.costs)) {
            return;
        }
        self.members.retain(|p| !dominates(&point.costs, &p.costs));
        self.members.push(Archived { sym: point.sym().to_vec(), costs: point.costs.clone(), score: point.score() });
        if self.members.len() > cap {
            let costs: Vec<_> = self.members.iter().map(|p| p.costs.clone()).collect();
            let (_, crowd) = rank_and_crowding(&costs);
            let remove = (0..crowd.len()).min_by(|&a, &b| crowd[a].total_cmp(&crowd[b]).then(b.cmp(&a))).unwrap();
            self.members.remove(remove);
        }
    }
}

struct Budget<'a> {
    start: Instant,
    paused_before: Duration,
    control: &'a Control,
    params: &'a Params,
    evals: u64,
}
impl Budget<'_> {
    fn elapsed(&self) -> Duration {
        self.start.elapsed().saturating_sub(self.control.paused_for().saturating_sub(self.paused_before))
    }
    fn stopped(&mut self) -> bool {
        while self.control.paused() && !self.control.cancelled() {
            std::thread::sleep(Duration::from_millis(20));
        }
        self.control.cancelled()
            || self.params.evals.is_some_and(|e| self.evals >= e)
            || (self.params.evals.is_none() && self.params.time_ms.is_some_and(|ms| self.elapsed() >= Duration::from_millis(ms)))
    }
    fn tick(&mut self) -> bool {
        if self.stopped() {
            false
        } else {
            self.evals += 1;
            true
        }
    }
}

/// Reach a crossover/shuffle permutation through delta-evaluated swaps, retaining a valid
/// partial child if the budget ends mid-child. All objective/profile states stay synchronized.
fn transform(point: &mut Individual, target: &[u8], scorers: &[Scorer], budget: &mut Budget) {
    let mut changed = false;
    for (slot, &want) in target.iter().enumerate() {
        let have = point.states[0].sym[slot] as usize;
        if have == want as usize {
            continue;
        }
        if !budget.tick() {
            return;
        }
        changed = true;
        for (i, (sc, state)) in scorers.iter().zip(&mut point.states).enumerate() {
            let delta = sc.delta(state, have, want as usize);
            sc.apply(state, have, want as usize, delta);
            if i < point.costs.len() {
                point.costs[i] = state.score;
            }
        }
    }
    if !changed {
        budget.tick();
    }
}

fn emit(archive: &Archive, ctx: &Ctx, budget: &Budget, on_progress: &mut impl FnMut(&Progress)) {
    if let Some(best) = archive.members.iter().min_by(|a, b| a.score.total_cmp(&b.score)) {
        on_progress(&Progress {
            elapsed_ms: budget.elapsed().as_millis() as u64,
            evals: budget.evals,
            best: best.score,
            best_chars: best.sym.iter().map(|&s| ctx.scorer.chars[s as usize]).collect(),
            explore: None,
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn nsga(
    ctx: &Ctx,
    scorers: &[Scorer],
    slots: &[usize],
    budget: &mut Budget,
    rng: &mut Rng,
    archive: &mut Archive,
    interval: Duration,
    on_progress: &mut impl FnMut(&Progress),
) {
    let p = &budget.params.nsga2;
    let mut pop = vec![Individual::new(scorers, &ctx.start.sym)];
    archive.offer(&pop[0], p.archive_size);
    let mut last_emit = Instant::now();
    while pop.len() < p.pop_size && !budget.stopped() && slots.len() >= 2 {
        let mut target = ctx.start.sym.clone();
        let mut symbols: Vec<_> = slots.iter().map(|&s| target[s]).collect();
        rng.shuffle(&mut symbols);
        for (&slot, symbol) in slots.iter().zip(symbols) {
            target[slot] = symbol;
        }
        let mut point = pop[0].clone();
        transform(&mut point, &target, scorers, budget);
        archive.offer(&point, p.archive_size);
        pop.push(point);
    }
    let mut generation = 0;
    while !budget.stopped() && slots.len() >= 2 && (p.generations == 0 || generation < p.generations) {
        let costs: Vec<_> = pop.iter().map(|p| p.costs.clone()).collect();
        let (ranks, crowd) = rank_and_crowding(&costs);
        let pick = |rng: &mut Rng| {
            let (a, b) = (rng.below(pop.len()), rng.below(pop.len()));
            if ranks[a] < ranks[b] || (ranks[a] == ranks[b] && crowd[a] >= crowd[b]) {
                a
            } else {
                b
            }
        };
        let mut children = Vec::new();
        while children.len() < p.pop_size && !budget.stopped() {
            let (a, b) = (pick(rng), pick(rng));
            let mut target = if rng.f64() < p.crossover_rate {
                genetic::crossover(slots, &pop[a].states[0], &pop[b].states[0], rng)
            } else {
                pop[a].sym().to_vec()
            };
            genetic::mutate(slots, &mut target, pop[a].sym(), p.mutation_rate, rng);
            let mut child = pop[a].clone();
            transform(&mut child, &target, scorers, budget);
            archive.offer(&child, p.archive_size);
            children.push(child);
        }
        pop.extend(children);
        let costs: Vec<_> = pop.iter().map(|p| p.costs.clone()).collect();
        let (ranks, crowd) = rank_and_crowding(&costs);
        let mut order: Vec<_> = (0..pop.len()).collect();
        order.sort_by(|&a, &b| ranks[a].cmp(&ranks[b]).then(crowd[b].total_cmp(&crowd[a])).then(a.cmp(&b)));
        pop = order.into_iter().take(p.pop_size).map(|i| pop[i].clone()).collect();
        generation += 1;
        if last_emit.elapsed() >= interval {
            emit(archive, ctx, budget, on_progress);
            last_emit = Instant::now();
        }
    }
}

/// Single-scorer SA inner loop; positive random weights are refolded once per restart.
#[allow(clippy::too_many_arguments)]
fn random_weight_sa(
    ctx: &Ctx,
    scorers: &[Scorer],
    budget: &mut Budget,
    rng: &mut Rng,
    archive: &mut Archive,
    interval: Duration,
    on_progress: &mut impl FnMut(&Progress),
) {
    let p = &budget.params.nsga2;
    archive.offer(&Individual::new(scorers, &ctx.start.sym), p.archive_size);
    if ctx.movable.len() < 2 {
        return;
    }
    let steps = if budget.params.sa.steps == 0 { 10_000 } else { budget.params.sa.steps };
    let mut restart = 0;
    let mut last_emit = Instant::now();
    while !budget.stopped() {
        let m = p.objectives.len();
        let mut weights = vec![0.0; m];
        if restart < m {
            weights[restart] = 1.0;
        } else {
            for w in &mut weights {
                *w = -rng.f64().max(1e-12).ln();
            }
            let total: f64 = weights.iter().sum();
            for w in &mut weights {
                *w /= total;
            }
        }
        // Reference-profile scales put heterogeneous natural-cost units on comparable scales.
        let weighted: Vec<_> = p
            .objectives
            .iter()
            .zip(weights)
            .map(|(id, w)| (id.as_str(), (w / ctx.scorer.profile.scale.get(id).copied().unwrap_or(1.0).max(1e-9) as f64) as f32))
            .collect();
        let sc = ctx.scorer.with_weights(&weighted).expect("validated objective weights");
        let sym = if restart == 0 { ctx.start.sym.clone() } else { ctx.random_state(rng).sym };
        let mut current = sc.state_from_sym(sym);
        let mut best = current.clone();
        let (mut sum, mut positive) = (0.0, 0);
        for _ in 0..64 {
            if !budget.tick() {
                break;
            }
            let (a, b) = ctx.random_pair(rng);
            let delta = sc.delta(&current, a, b);
            if delta > 0.0 {
                sum += delta;
                positive += 1;
            }
        }
        let t0 = (if positive == 0 { 1.0 } else { sum / positive as f64 }) / (1.0 / budget.params.sa.accept0.clamp(1e-6, 0.999)).ln();
        for step in 0..steps {
            if !budget.tick() {
                break;
            }
            let (a, b) = ctx.random_pair(rng);
            let delta = sc.delta(&current, a, b);
            let temp = t0 * budget.params.sa.t_end_ratio.max(1e-12).powf(step as f64 / steps as f64);
            if delta <= 0.0 || rng.f64() < (-delta / temp).exp() {
                sc.apply(&mut current, a, b, delta);
                if current.score < best.score {
                    best.clone_from(&current);
                }
            }
        }
        archive.offer(&Individual::new(scorers, &best.sym), p.archive_size);
        restart += 1;
        if last_emit.elapsed() >= interval {
            emit(archive, ctx, budget, on_progress);
            last_emit = Instant::now();
        }
    }
}

/// Ordered population runner. Threads resolve to one because rank/crowding and archive
/// insertion order are part of seeded replay. Invalid parameters must be rejected at prepare.
pub fn run(ctx: &Ctx, params: &Params, interval: Duration, control: &Control, mut on_progress: impl FnMut(&Progress)) -> Outcome {
    validate(&params.nsga2).expect("validate Pareto parameters before optimize::run");
    let mut params = params.clone();
    params.threads = 1;
    let mut scorers: Vec<_> = params.nsga2.objectives.iter().map(|id| ctx.scorer.objective(id).unwrap()).collect();
    scorers.push(ctx.scorer.clone());
    let mut slots: Vec<_> = ctx.movable.iter().map(|&s| ctx.start.pos[s] as usize).collect();
    slots.sort_unstable();
    let mut budget = Budget { start: Instant::now(), paused_before: control.paused_for(), control, params: &params, evals: 0 };
    let mut archive = Archive::default();
    let mut rng = Rng::new(params.seed);
    if params.algo == Algo::Rwsa {
        random_weight_sa(ctx, &scorers, &mut budget, &mut rng, &mut archive, interval, &mut on_progress);
    } else {
        nsga(ctx, &scorers, &slots, &mut budget, &mut rng, &mut archive, interval, &mut on_progress);
    }
    // Remove accumulated delta drift, then filter again in the exact persisted costs.
    let mut exact = Archive::default();
    for p in archive.members {
        exact.offer(&Individual::new(&scorers, &p.sym), params.nsga2.archive_size);
    }
    emit(&exact, ctx, &budget, &mut on_progress);
    let mut front: Vec<_> = exact
        .members
        .into_iter()
        .map(|p| {
            let state = ctx.scorer.state_from_sym(p.sym.clone());
            let report = breakdown(ctx.scorer, &state);
            let raw = params.nsga2.objectives.iter().map(|id| report.metrics[metrics::index_of(id).unwrap()].value).collect();
            Point { sym: p.sym, score: state.score, raw, costs: p.costs }
        })
        .collect();
    front.sort_by(|a, b| {
        a.costs
            .iter()
            .zip(&b.costs)
            .find_map(|(a, b)| {
                let cmp = a.total_cmp(b);
                (cmp != std::cmp::Ordering::Equal).then_some(cmp)
            })
            .unwrap_or_else(|| a.sym.cmp(&b.sym))
    });
    let mut top: Vec<_> = front.iter().map(|p| Scored { score: p.score, sym: p.sym.clone() }).collect();
    top.sort_by(|a, b| a.score.total_cmp(&b.score).then(a.sym.cmp(&b.sym)));
    top.truncate(params.top.max(1));
    let elapsed = budget.elapsed();
    let evals = budget.evals;
    Outcome {
        params: params.clone(),
        top,
        evals,
        elapsed_ms: elapsed.as_millis() as u64,
        evals_per_sec: evals as f64 / elapsed.as_secs_f64().max(1e-9),
        objectives: objectives(&params.nsga2),
        front,
    }
}
