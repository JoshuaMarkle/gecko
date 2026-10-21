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
    let spec = RunSpec { geometry: &g, corpus: &c, profile: &p, start: &start, pins: ",.", params };
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
