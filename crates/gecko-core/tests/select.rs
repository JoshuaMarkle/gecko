//! B9 rank-sum selection (`select.rs`, #60): layouts from several runs are merged, re-scored
//! on one board / corpus / profile, ranked per metric and sorted by rank sum. See
//! `context/cli.md` (`gecko select`).

use std::collections::BTreeMap;

use gecko_core::optimize::{Algo, Control, Params};
use gecko_core::run::{self, RunSpec};
use gecko_core::select;
use gecko_core::store::Store;

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

#[test]
fn rank_sum_over_runs() {
    let mut s = Store::open(ROOT.as_ref()).unwrap();
    s.runs = std::env::temp_dir().join(format!("gecko-test-select-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&s.runs);
    let d = &s.defaults;
    let g = s.geometry(&d.geometry).unwrap();
    let c = s.corpus(&d.corpus).unwrap();
    let p = s.profile(&d.profile).unwrap();
    let start = s.layout(&d.reference, &g.name).unwrap().fit(&g).unwrap();
    let mut ids = Vec::new();
    for seed in [1, 2, 1] {
        let params = Params { algo: Algo::Hc, seed, threads: 1, time_ms: None, evals: Some(20_000), top: 5, ..Default::default() };
        let spec = RunSpec { geometry: &g, corpus: &c, profile: &p, start: &start, pins: "", params, holdout: None };
        let prep = run::prepare(&s.runs, &spec).unwrap();
        run::execute(&prep, &spec, &Control::default(), |_| {}).unwrap();
        ids.push(prep.config.id.clone());
    }
    let (cands, sources) = select::gather(&s.runs, &ids, 300).unwrap();
    assert_eq!(sources.len(), 3);
    assert!(sources.iter().all(|r| r.kind == "optimize" && r.layouts >= 2), "{sources:?}");
    // Seed 1 twice: its layouts are merged, each with two sources.
    let total: usize = sources.iter().map(|r| r.layouts).sum();
    assert!(cands.len() <= total - sources[0].layouts);
    assert!(cands
        .iter()
        .filter(|c| c.sources.iter().any(|s| s.starts_with(&ids[0])))
        .all(|c| c.sources.iter().any(|s| s.starts_with(&ids[2]))));

    let ids_m: Vec<String> = ["sfb", "inroll", "effort"].map(String::from).to_vec();
    let defs = select::metric_cols(&ids_m, &p).unwrap();
    let boards = BTreeMap::from([(g.name.clone(), g.clone())]);
    let (cols, ranked) = select::select(&cands, &boards, &g, &c, &p, &defs).unwrap();
    assert_eq!(cols.iter().map(|m| (m.id, m.dir)).collect::<Vec<_>>(), [("sfb", "min"), ("inroll", "max"), ("effort", "min")]);
    assert_eq!(ranked.len(), cands.len());
    let n = ranked.len() as f64;
    for (i, r) in ranked.iter().enumerate() {
        assert_eq!(r.rank, i + 1);
        assert_eq!(r.ranks.len(), 3);
        assert!((r.rank_sum - r.ranks.iter().sum::<f64>()).abs() < 1e-9);
        assert!(r.ranks.iter().all(|&x| (1.0..=n).contains(&x)));
    }
    assert!(ranked.windows(2).all(|w| w[0].rank_sum <= w[1].rank_sum));
    // Each metric's ranks are a permutation of 1..=n (sum n(n+1)/2), and higher inroll ranks better.
    for m in 0..3 {
        assert!((ranked.iter().map(|r| r.ranks[m]).sum::<f64>() - n * (n + 1.0) / 2.0).abs() < 1e-9);
    }
    let best_t03 = ranked.iter().max_by(|a, b| a.values[1].total_cmp(&b.values[1])).unwrap();
    assert_eq!(best_t03.ranks[1], ranked.iter().filter(|r| r.values[1] == best_t03.values[1]).count() as f64 / 2.0 + 0.5);
    // Re-scored values match the runs' own scores (same inputs; a fresh scorer numbers symbols
    // in another order, so f32 sums differ, and the trigram coverage cut breaks frequency ties
    // by symbol number, keeping a few different rare trigrams: ~3e-5 of the score).
    for r in &ranked {
        let src = &r.sources[0];
        let (id, rank) = src.split_once('#').unwrap();
        let res = run::load_result(&s.runs.join(id)).unwrap();
        let own = &res.best[rank.parse::<usize>().unwrap() - 1];
        assert_eq!(own.chars, r.chars);
        assert!((own.score - r.score).abs() < 1e-4 * own.score.abs().max(1.0), "{} vs {}", own.score, r.score);
    }
    assert!(select::metric_cols(&["ZZ9".to_string()], &p).is_err());
    assert!(select::gather(&s.runs, &["../x".to_string()], 10).is_err());
    let _ = std::fs::remove_dir_all(&s.runs);
}
