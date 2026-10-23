//! B8 adaptive fidelity: coarse scorers keep the delta == full property, the runner reports exact
//! scores, and an evaluation-budget run is reproducible. See `context/objective.md`.

use gecko_core::optimize::{self, Algo, Control, Ctx, Params};
use gecko_core::profile::Fidelity;
use gecko_core::rng::Rng;
use gecko_core::{Corpus, Geometry, Layout, Profile, Scorer};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn read(p: &str) -> String {
    std::fs::read_to_string(format!("{ROOT}/{p}")).unwrap()
}

fn scorer(fidelity: Option<Fidelity>) -> Scorer {
    let g = Geometry::from_toml(&read("data/geometries/ansi-30.toml")).unwrap();
    let c = Corpus::from_bytes(&std::fs::read(format!("{ROOT}/data/corpora/english.gkc")).unwrap()).unwrap();
    let mut p = Profile::from_toml(&read("data/profiles/default.toml")).unwrap();
    p.fidelity = fidelity;
    let l = Layout::from_toml(&read("data/layouts/qwerty.toml")).unwrap();
    Scorer::new(&g, &c, &p, &l).unwrap()
}

#[test]
fn levels_grow_to_the_full_table() {
    let f = Fidelity::new(300);
    assert_eq!(f.levels(5000), vec![300, 1200, 4800]);
    assert_eq!(f.levels(300), Vec::<usize>::new());
    assert_eq!(Fidelity { top_k: 10, grow: 10, misses: 1 }.levels(1000), vec![10, 100]);
}

#[test]
fn coarse_scorer_keeps_delta_equal_to_full() {
    let sc = scorer(None);
    let total = sc.trigram_count();
    let co = sc.coarse(200);
    assert_eq!(co.trigram_count(), 200);
    assert_eq!(sc.coarse(total).trigram_count(), total);
    let mut rng = Rng::new(7);
    let mut st = co.identity();
    for _ in 0..2000 {
        let (a, b) = (rng.below(co.n), rng.below(co.n));
        let d = co.delta(&st, a, b);
        let before = co.full(&st);
        co.apply(&mut st, a, b, d);
        let after = co.full(&st);
        assert!((after - before - d).abs() < 1e-4, "delta {d} vs full {}", after - before);
    }
    // The coarse score drops only (non-negative) trigram terms of the dropped trigrams.
    let (full, cheap) = (sc.identity(), co.identity());
    assert!((full.score - cheap.score).abs() > 0.0);
    assert!((full.score - cheap.score).abs() < 0.2 * full.score.abs().max(1.0));
}

fn search(fidelity: Option<Fidelity>, algo: Algo) -> optimize::Outcome {
    let sc = scorer(fidelity);
    let ctx = Ctx::new(&sc, (0..sc.n).collect(), sc.identity());
    let params = Params { algo, seed: 3, threads: 2, time_ms: None, evals: Some(2_000_000), top: 5, ..Default::default() };
    optimize::run(&ctx, &params, std::time::Duration::from_secs(60), &Control::default(), |_| {})
}

#[test]
fn fidelity_runs_report_exact_scores_and_reproduce() {
    for algo in [Algo::Sa, Algo::Bls] {
        let a = search(Some(Fidelity::new(300)), algo);
        let f = a.fidelity.as_ref().expect("fidelity report");
        assert_eq!(f.levels.first(), Some(&300));
        assert_eq!(f.final_level.len(), 2);
        assert!(f.confirmed > 0);
        let sc = scorer(None);
        for s in &a.top {
            assert!((sc.state_from_sym(s.sym.clone()).score - s.score).abs() < 1e-9);
        }
        let b = search(Some(Fidelity::new(300)), algo);
        assert_eq!(a.top, b.top, "{algo:?}: same seed and budget, same result");
        // Results are local optima of the exact objective.
        let st = sc.state_from_sym(a.top[0].sym.clone());
        assert!((0..sc.n).all(|x| (x + 1..sc.n).all(|y| sc.delta(&st, x, y) > -1e-9)));
    }
    assert!(search(None, Algo::Sa).fidelity.is_none());
}
