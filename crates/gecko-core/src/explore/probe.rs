//! Landscape probe: random layouts (score + raw metrics, the "how good is good" baseline) and
//! random swap walks (ruggedness: autocorrelation and correlation length).

use super::archive::raw_vector;
use super::atlas::Sample;
use crate::optimize::{Control, Ctx};
use crate::rng::Rng;

pub struct Probe {
    pub samples: Vec<Sample>,
    pub walks: Vec<Vec<f64>>,
    pub evals: u64,
}

/// Run `samples` random layouts and `walks` random walks of `walk_len` swaps on `threads` threads.
pub fn run(ctx: &Ctx, seed: u64, threads: usize, samples: usize, walks: usize, walk_len: usize, control: &Control) -> Probe {
    let threads = threads.max(1);
    let parts: Vec<Probe> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..threads)
            .map(|w| {
                s.spawn(move || {
                    let mut rng = Rng::for_worker(seed ^ 0x9b0b_e5ed, w as u64);
                    let mut p = Probe { samples: Vec::new(), walks: Vec::new(), evals: 0 };
                    for _ in (w..samples).step_by(threads) {
                        if control.cancelled() {
                            break;
                        }
                        let st = ctx.random_state(&mut rng);
                        p.samples.push(Sample { score: st.score, raw: raw_vector(ctx.scorer, &st) });
                        p.evals += 1;
                    }
                    if ctx.movable.len() >= 2 {
                        for _ in (w..walks).step_by(threads) {
                            if control.cancelled() {
                                break;
                            }
                            let mut st = ctx.random_state(&mut rng);
                            let mut walk = Vec::with_capacity(walk_len + 1);
                            walk.push(st.score);
                            for _ in 0..walk_len {
                                let (a, b) = ctx.random_pair(&mut rng);
                                let d = ctx.scorer.delta(&st, a, b);
                                ctx.scorer.apply(&mut st, a, b, d);
                                walk.push(st.score);
                            }
                            p.evals += walk_len as u64;
                            p.walks.push(walk);
                        }
                    }
                    p
                })
            })
            .collect();
        handles.into_iter().filter_map(|h| h.join().ok()).collect()
    });
    let mut out = Probe { samples: Vec::new(), walks: Vec::new(), evals: 0 };
    for p in parts {
        out.samples.extend(p.samples);
        out.walks.extend(p.walks);
        out.evals += p.evals;
    }
    out
}
