//! `run::verify`: an `--evals` run re-runs to the same best.json; a tampered result or a
//! time-budget run does not verify. See `context/cli.md` (`gecko verify`).

use gecko_core::optimize::{Algo, Control, Params};
use gecko_core::run::{self, RunSpec};
use gecko_core::store::Store;

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

/// The repo's data with a private, throwaway runs directory.
fn store(tag: &str) -> Store {
    let mut s = Store::open(ROOT.as_ref()).unwrap();
    s.runs = std::env::temp_dir().join(format!("gecko-test-verify-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&s.runs);
    s
}

fn make_run(s: &Store, params: Params) -> std::path::PathBuf {
    let d = &s.defaults;
    let g = s.geometry(&d.geometry).unwrap();
    let c = s.corpus(&d.corpus).unwrap();
    let p = s.profile(&d.profile).unwrap();
    let start = s.layout(&d.reference, &g.name).unwrap().fit(&g).unwrap();
    let spec = RunSpec { geometry: &g, corpus: &c, profile: &p, start: &start, pins: ",.", params, holdout: None };
    let prep = run::prepare(&s.runs, &spec).unwrap();
    run::execute(&prep, &spec, &Control::default(), |_| {}).unwrap();
    prep.dir
}

#[test]
fn evals_run_reproduces_and_tampering_is_caught() {
    let s = store("evals");
    let params = Params { algo: Algo::Sa, seed: 3, threads: 2, time_ms: None, evals: Some(60_000), top: 4, ..Default::default() };
    let dir = make_run(&s, params);
    let v = run::verify(&s, &dir).unwrap();
    assert!(v.ok, "{:#?}", v.checks);
    assert!(v.checks.iter().any(|c| c.name == "best#1"));

    let best = dir.join("best.json");
    let mut r = run::load_result(&dir).unwrap();
    r.best[0].score += 0.01;
    std::fs::write(&best, serde_json::to_string(&r).unwrap()).unwrap();
    let v = run::verify(&s, &dir).unwrap();
    assert!(!v.ok);
    assert!(v.checks.iter().any(|c| c.name == "best#1" && !c.ok));
    let _ = std::fs::remove_dir_all(&s.runs);
}

#[test]
fn time_run_and_changed_inputs_do_not_verify() {
    let s = store("time");
    let params = Params { algo: Algo::Hc, threads: 1, time_ms: Some(50), evals: None, top: 2, ..Default::default() };
    let dir = make_run(&s, params);
    assert!(run::verify(&s, &dir).is_err());

    let params = Params { algo: Algo::Hc, threads: 1, time_ms: None, evals: Some(5_000), top: 2, ..Default::default() };
    let dir = make_run(&s, params);
    let mut cfg = run::load_config(&dir).unwrap();
    cfg.corpus.hash = "0000000000000000".into();
    std::fs::write(dir.join("config.json"), serde_json::to_string(&cfg).unwrap()).unwrap();
    let v = run::verify(&s, &dir).unwrap();
    assert!(!v.ok && v.rerun.is_none());
    assert!(v.checks.iter().any(|c| c.name == "corpus" && !c.ok));
    let _ = std::fs::remove_dir_all(&s.runs);
}

#[test]
fn memetic_evals_run_replays_migration_and_pins() {
    let s = store("memetic");
    let mut params = Params { algo: Algo::Memetic, seed: 7, threads: 2, time_ms: None, evals: Some(100_000), top: 3, ..Default::default() };
    params.memetic.ga.pop_size = 4;
    params.memetic.local_steps = 2;
    params.memetic.migration_interval = 1;
    let dir = make_run(&s, params);
    let v = run::verify(&s, &dir).unwrap();
    assert!(v.ok, "{:#?}", v.checks);
    let _ = std::fs::remove_dir_all(&s.runs);
}

/// L06: `--holdout` scores every ranked layout on a corpus the search never saw, reports the
/// gain kept, records the corpus in config.json and verifies; the search corpus is refused.
#[test]
fn holdout_scores_results_and_verifies() {
    let s = store("holdout");
    let d = &s.defaults;
    let g = s.geometry(&d.geometry).unwrap();
    let c = s.corpus(&d.corpus).unwrap();
    let h = s.corpus("shai").unwrap();
    let p = s.profile(&d.profile).unwrap();
    let start = s.layout(&d.reference, &g.name).unwrap().fit(&g).unwrap();
    let params = Params { algo: Algo::Sa, seed: 5, threads: 2, time_ms: None, evals: Some(80_000), top: 5, ..Default::default() };
    let same = RunSpec { geometry: &g, corpus: &c, profile: &p, start: &start, pins: "", params: params.clone(), holdout: Some(&c) };
    assert!(run::prepare(&s.runs, &same).is_err(), "holding out the search corpus is refused");

    let spec = RunSpec { holdout: Some(&h), ..same };
    let prep = run::prepare(&s.runs, &spec).unwrap();
    assert_eq!(prep.config.holdout.as_ref().unwrap().name, "shai");
    let res = run::execute(&prep, &spec, &Control::default(), |_| {}).unwrap();
    let report = res.holdout.as_ref().unwrap();
    // Each holdout score is the layout scored from scratch on shai.
    let held = gecko_core::Scorer::new(&g, &h, &p, &start).unwrap();
    for r in &res.best {
        let l = gecko_core::Layout::from_flat("r", &g, r.chars.chars().collect()).unwrap();
        let want = held.state_for(&l).unwrap().score;
        assert!((r.holdout.unwrap() - want).abs() < 1e-9 * want.abs().max(1.0));
    }
    assert_eq!(report.best, res.best[0].holdout.unwrap());
    assert_eq!(report.start, held.identity().score);
    assert!(report.best < report.start && report.retained > 0.5, "{report:?}");
    let expect = (report.start - report.best) / (report.start_search - report.best_search);
    assert!((report.retained - expect).abs() < 1e-12);
    assert!(report.rank_corr.is_some_and(|r| (-1.0..=1.0).contains(&r)));

    let v = run::verify(&s, &prep.dir).unwrap();
    assert!(v.ok && v.checks.iter().any(|c| c.name == "holdout" && c.ok), "{:#?}", v.checks);
    let _ = std::fs::remove_dir_all(&s.runs);
}
