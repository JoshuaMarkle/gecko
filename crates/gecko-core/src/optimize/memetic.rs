//! Island memetic search: shared GA operators, bounded HC/tabu refinement, deterministic
//! ring migration. Generation barriers make evaluation-budget runs independent of thread
//! scheduling. See `context/algorithms/memetic.md`.

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::{genetic, moves, steepest_descent_capped, tabu::Tabu, Control, Ctx, Outcome, Params, Progress, Scored, TopK};
use crate::{rng::Rng, scorer::State};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LocalSearch {
    #[default]
    Hc,
    Tabu,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct MemeticParams {
    /// GA operators and population size, per island.
    pub ga: genetic::GaParams,
    pub local_search: LocalSearch,
    /// Maximum HC improving swaps or tabu iterations per child.
    pub local_steps: usize,
    /// 0 = one island per worker thread. Explicit islands may share workers.
    pub islands: usize,
    /// Generations between synchronous ring exchanges; 0 disables migration.
    pub migration_interval: u64,
    /// Distinct best individuals sent to the next island at each exchange.
    pub migrants: usize,
    /// Consecutive generations below 1/4 of the population before a diversity restart.
    /// 0 disables restarts. The best individual is always retained.
    pub collapse_generations: u64,
}

impl Default for MemeticParams {
    fn default() -> Self {
        Self {
            ga: genetic::GaParams { pop_size: 24, elitism: 2, ..Default::default() },
            local_search: LocalSearch::Hc,
            local_steps: 12,
            islands: 0,
            migration_interval: 5,
            migrants: 2,
            collapse_generations: 3,
        }
    }
}

struct Island {
    rng: Rng,
    pop: Vec<State>,
    slots: Vec<usize>,
    top: TopK,
    generation: u64,
    collapsed: u64,
}

impl Island {
    fn new(ctx: &Ctx, seed: u64, id: usize, top: usize) -> Self {
        let mut slots: Vec<_> = ctx.movable.iter().map(|&s| ctx.start.pos[s] as usize).collect();
        slots.sort_unstable();
        let mut best = TopK::new(top);
        best.offer(ctx.start.score, &ctx.start.sym);
        Self { rng: Rng::for_worker(seed, id as u64), pop: vec![ctx.start.clone()], slots, top: best, generation: 0, collapsed: 0 }
    }

    fn refine(&mut self, ctx: &Ctx, p: &MemeticParams, mut st: State) -> (State, u64) {
        match p.local_search {
            LocalSearch::Hc => {
                let mut evals = 0;
                for _ in 0..p.local_steps {
                    let before = st.score;
                    evals += steepest_descent_capped(ctx, &mut st, 1);
                    if st.score < before - 1e-9 {
                        continue;
                    }
                    let tries: [(bool, moves::MoveFn); 3] = [
                        (ctx.moves.columns, moves::column_move),
                        (ctx.moves.cycle3, moves::cycle3_move),
                        (ctx.moves.depth2, moves::depth2_move),
                    ];
                    let mut improved = false;
                    for (enabled, try_move) in tries {
                        if enabled {
                            let (e, ok) = try_move(ctx, &mut st);
                            evals += e;
                            if ok {
                                improved = true;
                                break;
                            }
                        }
                    }
                    if !improved {
                        break;
                    }
                }
                (st, evals)
            }
            LocalSearch::Tabu => Tabu::refine(ctx, st, &mut self.rng, p.local_steps),
        }
    }

    fn generation(&mut self, ctx: &Ctx, p: &MemeticParams) -> u64 {
        if self.slots.len() < 2 {
            return 1;
        }
        let size = p.ga.pop_size.max(4);
        let mut evals = 0;
        if self.generation == 0 {
            let (st, e) = self.refine(ctx, p, self.pop[0].clone());
            evals += e;
            self.top.offer(st.score, &st.sym);
            self.pop[0] = st;
        }
        self.pop.sort_by(|a, b| a.score.total_cmp(&b.score));
        let restart = p.collapse_generations > 0 && self.collapsed >= p.collapse_generations;
        let fresh = self.generation == 0 || restart;
        let elites = if fresh { 1 } else { p.ga.elitism.max(1).min(size - 1).min(self.pop.len()) };
        let mut next = self.pop[..elites].to_vec();
        // Refinement often maps many children to one optimum. Bound rejection work even
        // when there are fewer distinct optima than requested population members.
        for _ in 0..size * 3 {
            if next.len() >= size {
                break;
            }
            let child = if fresh {
                ctx.random_state(&mut self.rng)
            } else {
                let sym = genetic::offspring(&self.pop, &self.slots, &p.ga, &mut self.rng);
                ctx.scorer.state_from_sym(sym)
            };
            evals += 1;
            let (child, e) = self.refine(ctx, p, child);
            evals += e;
            self.top.offer(child.score, &child.sym);
            if !next.iter().any(|s| s.sym == child.sym) {
                next.push(child);
            }
        }
        self.pop = next;
        if restart || self.pop.len() >= (size / 4).max(2) {
            self.collapsed = 0;
        } else {
            self.collapsed += 1;
        }
        self.generation += 1;
        evals.max(1)
    }

    fn emigrants(&self, n: usize) -> Vec<Scored> {
        let mut pop: Vec<_> = self.pop.iter().collect();
        pop.sort_by(|a, b| a.score.total_cmp(&b.score));
        pop.into_iter().take(n).map(|s| Scored { score: s.score, sym: s.sym.clone() }).collect()
    }

    fn immigrate(&mut self, ctx: &Ctx, migrants: &[Scored], capacity: usize) {
        for s in migrants {
            if self.pop.iter().any(|p| p.sym == s.sym) {
                continue;
            }
            let st = ctx.scorer.state_from_sym(s.sym.clone());
            self.top.offer(st.score, &st.sym);
            if self.pop.len() < capacity.max(4) {
                self.pop.push(st);
            } else if let Some((i, worst)) = self.pop.iter().enumerate().max_by(|(_, a), (_, b)| a.score.total_cmp(&b.score)) {
                if st.score < worst.score {
                    self.pop[i] = st;
                }
            }
        }
    }
}

/// Exchange snapshots, so no island can forward an immigrant again within the same round.
fn migrate(ctx: &Ctx, islands: &mut [Island], p: &MemeticParams) {
    if islands.len() < 2 || p.migrants == 0 {
        return;
    }
    let outgoing: Vec<_> = islands.iter().map(|i| i.emigrants(p.migrants)).collect();
    let n = islands.len();
    for (id, island) in islands.iter_mut().enumerate() {
        island.immigrate(ctx, &outgoing[(id + n - 1) % n], p.ga.pop_size);
    }
}

/// Synchronous generations, evaluated in parallel. The coordinator owns cancellation and
/// pause/budget decisions, so workers never wait at a barrier abandoned by another worker.
/// Evaluation budgets stop at generation boundaries (including counted initialization and
/// refinement); the last generation may overshoot the requested budget.
pub(super) fn run(ctx: &Ctx, params: &Params, interval: Duration, control: &Control, mut on_progress: impl FnMut(&Progress)) -> Outcome {
    let mut params = params.clone();
    params.threads = params.threads.max(1);
    if params.memetic.islands == 0 {
        params.memetic.islands = params.threads;
    }
    let ctx = &ctx.clone().with_moves(params.moves.clone());
    let p = &params.memetic;
    let mut islands: Vec<_> = (0..p.islands).map(|id| Island::new(ctx, params.seed, id, params.top)).collect();
    let mut merged = TopK::new(params.top);
    let t0 = Instant::now();
    let mut last_emit = t0;
    let mut evals = 0;
    let mut generation = 0;
    loop {
        if ctx.movable.len() < 2
            || control.cancelled()
            || params.evals.is_some_and(|e| evals >= e)
            || params.time_ms.is_some_and(|ms| control.active(t0) >= Duration::from_millis(ms))
        {
            break;
        }
        if control.paused() {
            std::thread::sleep(Duration::from_millis(20));
            continue;
        }
        // Fixed island seeds and deterministic merging keep results identical even when
        // the number of worker threads changes, provided the island count is explicit.
        let chunk_size = islands.len().div_ceil(params.threads);
        let counts: Vec<_> = std::thread::scope(|scope| {
            let handles: Vec<_> = islands
                .chunks_mut(chunk_size)
                .map(|chunk| scope.spawn(move || chunk.iter_mut().map(|i| i.generation(ctx, p)).sum::<u64>()))
                .collect();
            handles.into_iter().map(|h| h.join().expect("memetic worker panicked")).collect()
        });
        evals += counts.into_iter().sum::<u64>();
        generation += 1;
        if p.migration_interval > 0 && generation % p.migration_interval == 0 {
            migrate(ctx, &mut islands, p);
        }
        for island in &islands {
            merged.merge(&island.top);
        }
        if last_emit.elapsed() >= interval {
            emit(ctx, &merged, t0, evals, control, &mut on_progress);
            last_emit = Instant::now();
        }
    }
    // Even a cancelled or zero-budget run retains its starting layout.
    for island in &islands {
        merged.merge(&island.top);
    }
    let mut top: Vec<_> = merged
        .items
        .into_iter()
        .map(|s| {
            let st = ctx.scorer.state_from_sym(s.sym);
            Scored { score: st.score, sym: st.sym }
        })
        .collect();
    top.sort_by(|a, b| a.score.total_cmp(&b.score));
    let merged = TopK { k: params.top.max(1), items: top };
    emit(ctx, &merged, t0, evals, control, &mut on_progress);
    let elapsed = control.active(t0);
    Outcome {
        params,
        top: merged.items,
        evals,
        elapsed_ms: elapsed.as_millis() as u64,
        evals_per_sec: evals as f64 / elapsed.as_secs_f64().max(1e-9),
        objectives: Vec::new(),
        front: Vec::new(),
        fidelity: None,
    }
}

fn emit(ctx: &Ctx, top: &TopK, t0: Instant, evals: u64, control: &Control, on_progress: &mut impl FnMut(&Progress)) {
    if let Some(s) = top.best() {
        on_progress(&Progress {
            elapsed_ms: control.active(t0).as_millis() as u64,
            evals,
            best: s.score,
            best_chars: s.sym.iter().map(|&x| ctx.scorer.chars[x as usize]).collect(),
            explore: None,
            workers: Vec::new(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Corpus, Geometry, Layout, Profile, Scorer};

    fn scorer() -> Scorer {
        let geometry = Geometry::from_toml(
            r#"
name = "tiny"
[[key]]
pos = [0.0, 0.0, 0.0]
finger = "LM"
row = 0
col = 0
home = true
effort = 1.5
[[key]]
pos = [1.0, 0.0, 0.0]
finger = "LI"
row = 0
col = 1
home = true
effort = 1.0
[[key]]
pos = [2.0, 0.0, 0.0]
finger = "RI"
row = 0
col = 2
home = true
effort = 1.2
"#,
        )
        .unwrap();
        let mut corpus = Corpus::new("tiny", "abc");
        corpus.add_text("aaaaaabbbacabcacbabcabc");
        let layout = Layout::from_str_flat("tiny", "tiny", "abc").unwrap();
        let profile = Profile::from_toml("name = 'tiny'\n[weights]\neffort = 1.0\nsfb = 1.0").unwrap();
        Scorer::new(&geometry, &corpus, &profile, &layout).unwrap()
    }

    #[test]
    fn migration_is_a_ring_of_snapshots_and_rejects_duplicates() {
        let sc = scorer();
        let ctx = Ctx::new(&sc, vec![0, 1, 2], sc.identity());
        let layouts = [vec![0, 1, 2], vec![1, 2, 0], vec![2, 0, 1]];
        let mut islands: Vec<_> = layouts
            .iter()
            .enumerate()
            .map(|(i, sym)| {
                let mut island = Island::new(&ctx, 1, i, 3);
                island.pop = vec![sc.state_from_sym(sym.clone())];
                island
            })
            .collect();
        let p = MemeticParams { migrants: 1, ..Default::default() };
        migrate(&ctx, &mut islands, &p);
        for (id, island) in islands.iter().enumerate() {
            assert_eq!(island.pop.len(), 2);
            assert!(island.pop.iter().any(|s| s.sym == layouts[id]));
            assert!(island.pop.iter().any(|s| s.sym == layouts[(id + 2) % 3]));
            assert!(!island.pop.iter().any(|s| s.sym == layouts[(id + 1) % 3]));
        }
        let duplicate = islands[0].emigrants(1);
        islands[0].immigrate(&ctx, &duplicate, 4);
        assert_eq!(islands[0].pop.len(), 2);
    }

    #[test]
    fn both_refiners_retain_the_best_visited_child() {
        let sc = scorer();
        let ctx = Ctx::new(&sc, vec![0, 1, 2], sc.identity());
        for local_search in [LocalSearch::Hc, LocalSearch::Tabu] {
            let p = MemeticParams { local_search, local_steps: 20, ..Default::default() };
            let mut island = Island::new(&ctx, 4, 0, 3);
            let before = sc.state_from_sym(vec![0, 2, 1]);
            let (after, evals) = island.refine(&ctx, &p, before.clone());
            assert!(after.score <= before.score + 1e-9);
            assert!((sc.full(&after) - after.score).abs() < 1e-5 * after.score.abs().max(1.0));
            assert!(evals > 0);
        }
    }

    #[test]
    fn collapsed_island_restarts_and_keeps_its_elite() {
        let sc = scorer();
        let ctx = Ctx::new(&sc, vec![0, 1, 2], sc.identity());
        let p = MemeticParams { collapse_generations: 1, ..Default::default() };
        let mut island = Island::new(&ctx, 4, 0, 3);
        island.generation(&ctx, &p);
        island.collapsed = 1;
        let before = island.top.best_score();
        island.generation(&ctx, &p);
        assert_eq!(island.collapsed, 0);
        assert!(island.top.best_score() <= before);
        assert!(!island.pop.is_empty());
    }
}
