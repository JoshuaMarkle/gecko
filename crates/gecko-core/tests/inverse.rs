//! Inverse fit (`gecko_core::inverse`): planted weights are recovered in direction, fitted weights
//! are centred, and the cross matrix is consistent with the fits.

use std::path::Path;

use gecko_core::inverse::{cross, fit_layout, InverseSpec};
use gecko_core::optimize::{self, Algo, Control, Params};
use gecko_core::store::Store;
use gecko_core::Scorer;

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn small() -> InverseSpec {
    InverseSpec { rounds: 3, evals: 300_000, seeds: 1, threads: 2, ..Default::default() }
}

#[test]
fn planted_inroll_preference_is_recovered() {
    let st = Store::open(Path::new(ROOT)).unwrap();
    let g = st.geometry("ansi-30").unwrap();
    let c = st.corpus("english").unwrap();
    let base = st.profile("default").unwrap();
    // A layout optimized under default with inward rolls weighted ×4.
    let mut planted = base.clone();
    let w = planted.weight("inroll");
    planted.weights.insert("inroll".into(), w * 4.0);
    let start = st.layout("qwerty", &g.name).unwrap().fit(&g).unwrap();
    let sc = Scorer::new(&g, &c, &planted, &start).unwrap();
    let ctx = optimize::Ctx::new(&sc, (0..sc.n).collect(), sc.identity());
    let params = Params { algo: Algo::Bls, seed: 3, threads: 2, time_ms: None, evals: Some(2_000_000), top: 1, ..Default::default() };
    let best = optimize::run(&ctx, &params, std::time::Duration::from_secs(3600), &Control::default(), |_| {}).top.remove(0);
    let layout = sc.layout_of(&sc.state_from_sym(best.sym), "planted").unwrap();
    let movable: Vec<usize> = (0..layout.chars.len()).collect();
    let f = fit_layout(&g, &c, &base, "planted", &layout, &movable, &small(), |_, _| {}).unwrap();
    let m = |id: &str| f.weights.iter().find(|w| w.id == id).unwrap().multiplier;
    assert!(m("inroll") > 1.3, "inroll ×{}", m("inroll"));
    assert!(m("inroll") > m("outroll"), "inroll ×{} outroll ×{}", m("inroll"), m("outroll"));
    let mean_log: f64 = f.weights.iter().map(|w| w.multiplier.ln()).sum::<f64>() / f.weights.len() as f64;
    assert!(mean_log.abs() < 1e-6, "centred: {mean_log}");
    assert!(f.weights.iter().all(|w| w.lo <= w.multiplier && w.multiplier <= w.hi));
}

#[test]
fn cross_matrix_matches_the_fits() {
    let st = Store::open(Path::new(ROOT)).unwrap();
    let g = st.geometry("ansi-30").unwrap();
    let c = st.corpus("english").unwrap();
    let base = st.profile("default").unwrap();
    let mut fits = Vec::new();
    for name in ["qwerty", "graphite"] {
        let l = st.layout(name, &g.name).unwrap().fit(&g).unwrap();
        let movable: Vec<usize> = (0..l.chars.len()).collect();
        fits.push(fit_layout(&g, &c, &base, name, &l, &movable, &small(), |_, _| {}).unwrap());
    }
    // QWERTY is far from optimal under the default profile.
    assert!(fits[0].base.gap_pct > 100.0, "{}", fits[0].base.gap_pct);
    let m = cross(&base, &fits);
    for (b, f) in fits.iter().enumerate() {
        // The diagonal is the fit's own gap, or more when the other layout beats it under these weights.
        assert!(m.gap_pct[b][b] >= f.fitted.gap_pct * (1.0 - 1e-6) - 1e-6, "{} vs {}", m.gap_pct[b][b], f.fitted.gap_pct);
        assert!(m.distance[b][b] == 0.0);
    }
    // Graphite beats QWERTY even under QWERTY's own fitted weights.
    assert!(m.rank[1][0] == 1 && m.rank[0][1] == 2, "{:?}", m.rank);
}
