//! CMA-ES (`gecko_core::cmaes`, #48): solves sphere and Rosenbrock, respects bounds, is
//! reproducible from its seed, and IPOP restarts (doubling λ) solve a multimodal function a
//! single run does not. See `context/algorithms/geometry-search.md`.

use gecko_core::cmaes::{minimize, minimize_batch, Bounds, Params, Stop};
use gecko_core::rng::Rng;

fn sphere(x: &[f64]) -> f64 {
    x.iter().map(|v| v * v).sum()
}

fn rosenbrock(x: &[f64]) -> f64 {
    x.windows(2).map(|w| 100.0 * (w[1] - w[0] * w[0]).powi(2) + (1.0 - w[0]).powi(2)).sum()
}

fn rastrigin(x: &[f64]) -> f64 {
    10.0 * x.len() as f64 + x.iter().map(|v| v * v - 10.0 * (std::f64::consts::TAU * v).cos()).sum::<f64>()
}

#[test]
fn sphere_10d() {
    let p = Params { sigma0: 1.0, restarts: 0, target: 1e-10, max_evals: 20_000, ..Params::default() };
    let r = minimize(sphere, &[3.0; 10], None, &p, &mut Rng::new(1));
    assert!(r.f <= 1e-10, "{} after {} evals", r.f, r.evals);
    assert_eq!(r.runs[0].stop, Stop::Target);
    assert!(r.evals < 8_000, "{} evals", r.evals);
}

#[test]
fn rosenbrock_10d() {
    let p = Params { sigma0: 0.5, restarts: 2, target: 1e-8, max_evals: 60_000, ..Params::default() };
    let r = minimize(rosenbrock, &[0.0; 10], None, &p, &mut Rng::new(2));
    assert!(r.f <= 1e-8, "{} after {} evals", r.f, r.evals);
    assert!(r.x.iter().all(|v| (v - 1.0).abs() < 1e-3), "{:?}", r.x);
}

#[test]
fn bounds_hold_and_find_the_edge() {
    // The unconstrained optimum (5, …) is outside: the best in-box point is the corner (1, …).
    let b = Bounds { lo: vec![-1.0; 4], hi: vec![1.0; 4] };
    let p = Params { sigma0: 0.3, restarts: 0, max_evals: 4_000, ..Params::default() };
    let mut seen_outside = false;
    let r = minimize(
        |x| {
            seen_outside |= x.iter().any(|v| !(-1.0..=1.0).contains(v));
            x.iter().map(|v| (v - 5.0).powi(2)).sum()
        },
        &[0.0; 4],
        Some(&b),
        &p,
        &mut Rng::new(3),
    );
    assert!(!seen_outside, "the objective only sees in-box points");
    assert!(r.x.iter().all(|v| (v - 1.0).abs() < 1e-6), "{:?}", r.x);
    // An interior optimum is found as without bounds.
    let r = minimize(|x| x.iter().map(|v| (v - 0.25).powi(2)).sum(), &[-0.9; 4], Some(&b), &p, &mut Rng::new(4));
    assert!(r.f < 1e-12 && r.x.iter().all(|v| (v - 0.25).abs() < 1e-6), "{:?}", r.x);
}

#[test]
fn same_seed_same_result() {
    let p = Params { sigma0: 0.5, max_evals: 3_000, ..Params::default() };
    let a = minimize(rosenbrock, &[0.0; 6], None, &p, &mut Rng::new(9));
    let b = minimize(rosenbrock, &[0.0; 6], None, &p, &mut Rng::new(9));
    assert_eq!(a.f.to_bits(), b.f.to_bits());
    assert_eq!(a.x, b.x);
    assert_eq!(a.history, b.history);
    let c = minimize(rosenbrock, &[0.0; 6], None, &p, &mut Rng::new(10));
    assert_ne!(a.x, c.x);
}

#[test]
fn ipop_restarts_solve_rastrigin() {
    let b = Bounds { lo: vec![-5.12; 5], hi: vec![5.12; 5] };
    let x0 = [3.0; 5];
    let single = Params { sigma0: 0.3, restarts: 0, max_evals: 200_000, target: 1e-8, ..Params::default() };
    let one = minimize(rastrigin, &x0, Some(&b), &single, &mut Rng::new(5));
    let ipop = Params { restarts: 9, ..single };
    let many = minimize(rastrigin, &x0, Some(&b), &ipop, &mut Rng::new(5));
    assert!(one.f > 0.5, "a default-λ run gets stuck in a local optimum ({})", one.f);
    assert!(many.f <= 1e-8, "IPOP: {} after {} evals, runs {:?}", many.f, many.evals, many.runs);
    for w in many.runs.windows(2) {
        assert_eq!(w[1].popsize, 2 * w[0].popsize);
    }
    assert!(many.evals <= 200_000);
}

#[test]
fn batch_evaluator_gets_whole_generations() {
    let p = Params { sigma0: 1.0, restarts: 0, max_evals: 600, ..Params::default() };
    let mut sizes = Vec::new();
    let r = minimize_batch(
        |xs| {
            sizes.push(xs.len());
            xs.iter().map(|x| sphere(x)).collect()
        },
        &[1.0; 3],
        None,
        &p,
        &mut Rng::new(6),
    );
    let lambda = 4 + (3.0 * 3f64.ln()).floor() as usize;
    assert!(sizes.iter().all(|&s| s == lambda));
    assert_eq!(r.evals, sizes.len() * lambda);
    assert!(r.evals <= 600 && r.runs[0].stop == Stop::Budget);
}

#[test]
fn an_empty_generation_halts_the_search() {
    let mut calls = 0;
    let out = minimize_batch(
        |xs| {
            calls += 1;
            if calls > 3 {
                return Vec::new();
            }
            xs.iter().map(|x| x.iter().map(|v| v * v).sum()).collect()
        },
        &[1.0, 1.0],
        None,
        &Params { restarts: 5, ..Default::default() },
        &mut Rng::new(1),
    );
    assert_eq!(out.runs.len(), 1);
    assert_eq!(out.runs[0].stop, Stop::Halted);
    assert_eq!(out.runs[0].generations, 3);
}

#[test]
fn restarts_can_start_from_the_best_point() {
    // Rastrigin: restarts from the best point keep the next run near the best basin.
    let f = |x: &[f64]| 10.0 * x.len() as f64 + x.iter().map(|v| v * v - 10.0 * (2.0 * std::f64::consts::PI * v).cos()).sum::<f64>();
    let bounds = Bounds { lo: vec![-5.12; 3], hi: vec![5.12; 3] };
    let p = Params { restart_from_best: true, restarts: 3, max_evals: 4000, ..Default::default() };
    let out = minimize(f, &[3.0, 3.0, 3.0], Some(&bounds), &p, &mut Rng::new(2));
    assert!(out.runs.len() > 1);
    let first = out.runs[0].best;
    assert!(out.f <= first);
}
