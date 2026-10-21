//! Pareto direction/cost semantics, exact tiny fronts, diversity, pins and persisted replay.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Duration;

use gecko_core::metrics::{Dir, METRICS};
use gecko_core::optimize::nsga2::{dominates, rank_and_crowding, NsgaParams};
use gecko_core::optimize::{self, Algo, Control, Ctx, Params};
use gecko_core::report::breakdown;
use gecko_core::run::{self, RunSpec};
use gecko_core::store::Store;
use gecko_core::{Corpus, Finger, Geometry, Key, Layout, Profile, Scorer};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn tiny(shift: bool) -> (Geometry, Corpus, Layout, Profile) {
    let fingers = [Finger::LP, Finger::LI, Finger::RI, Finger::RP, Finger::LT, Finger::RT];
    let effort = [1.0, 4.0, 2.0, 3.0, 1.5, 1.0];
    let key = (0..if shift { 6 } else { 4 })
        .map(|i| Key {
            pos: [i as f32, 1.0, 0.0],
            rot: [0.0; 3],
            size: [1.0; 2],
            finger: fingers[i],
            row: 1,
            col: i as i8,
            home: i % 2 == 0,
            stretch: false,
            hrm: None,
            effort: effort[i],
            fill: None,
            layer: 0,
            phys: None,
        })
        .collect();
    let g = Geometry {
        name: "tiny".into(),
        description: String::new(),
        key,
        comfort: BTreeMap::new(),
        comfort_tables: BTreeMap::new(),
        layer_keys: Vec::new(),
        layer_modes: Vec::new(),
    };
    let mut c = Corpus::new("tiny", if shift { "abcd␣" } else { "abcd" });
    if shift {
        c.add_text("Abcd ABCD abCD a bcd ABCd abcd AbcD");
    } else {
        c.mono[..4].copy_from_slice(&[8, 4, 2, 1]);
        c.bi[1] = 9;
        c.bi[6] = 3;
        c.tri[6] = 10; // abc
        c.tri[27] = 3; // bcd
        c.tri[36] = 1; // cba
    }
    let l = Layout::from_flat("tiny", &g, if shift { "abcd⇧␣" } else { "abcd" }.chars().collect()).unwrap();
    let p = Profile::from_toml("name='tiny'\n[weights]\nM01=2.0\nT03=0.25\n[scale]\nM01=100.0\nT03=3.0").unwrap();
    (g, c, l, p)
}

fn params(algo: Algo) -> Params {
    Params {
        algo,
        seed: 17,
        threads: 3,
        evals: Some(20_000),
        time_ms: None,
        top: 5,
        nsga2: NsgaParams { objectives: vec!["M01".into(), "T03".into()], pop_size: 24, archive_size: 128, ..Default::default() },
        ..Default::default()
    }
}

fn permutations(xs: &mut [u8], at: usize, output: &mut Vec<Vec<u8>>) {
    if at == xs.len() {
        output.push(xs.to_vec());
        return;
    }
    for i in at..xs.len() {
        xs.swap(at, i);
        permutations(xs, at + 1, output);
        xs.swap(at, i);
    }
}
fn cost_key(cost: &[f64]) -> Vec<i64> {
    cost.iter().map(|x| (x * 1e6).round() as i64).collect()
}

#[test]
fn nondominated_ranks_crowding_and_constant_dimensions() {
    let costs = vec![vec![0.0, 3.0], vec![1.0, 2.0], vec![3.0, 0.0], vec![2.0, 3.0], vec![4.0, 4.0]];
    let (rank, crowd) = rank_and_crowding(&costs);
    assert_eq!(rank, vec![0, 0, 0, 1, 2]);
    assert!(crowd[0].is_infinite() && crowd[2].is_infinite());
    assert!((crowd[1] - 2.0).abs() < 1e-12);
    assert!(!dominates(&costs[0], &costs[1]));
    assert!(dominates(&costs[1], &costs[4]));
    let (_, crowd) = rank_and_crowding(&[vec![1.0, 2.0], vec![1.0, 2.0], vec![1.0, 2.0]]);
    assert_eq!(crowd, vec![0.0; 3]);
    assert_eq!(rank_and_crowding(&[]), (Vec::new(), Vec::new()));
}

#[test]
fn refolding_matches_fresh_scorers_including_shift_and_nonlinear_costs() {
    let (g, c, l, p) = tiny(true);
    let sc = Scorer::new(&g, &c, &p, &l).unwrap();
    for metric in METRICS.iter().filter(|m| m.dir != Dir::Info) {
        let unit = sc.objective(metric.id).unwrap();
        let mut fresh_profile = p.clone();
        fresh_profile.weights.clear();
        fresh_profile.scale.clear();
        fresh_profile.weights.insert(metric.id.into(), 1.0);
        let fresh = Scorer::new(&g, &c, &fresh_profile, &l).unwrap();
        let mut st = unit.identity();
        let mut other = fresh.identity();
        for (a, b) in [(0, 1), (1, 2), (2, 3), (0, 3)] {
            assert!((st.score - other.score).abs() < 1e-8, "{}: refold score", metric.id);
            let delta = unit.delta(&st, a, b);
            let reference = fresh.delta(&other, a, b);
            assert!((delta - reference).abs() < 1e-8, "{}: refold delta", metric.id);
            unit.apply(&mut st, a, b, delta);
            fresh.apply(&mut other, a, b, reference);
            let tol = 1e-5 * st.score.abs().max(1.0);
            assert!((unit.full(&st) - st.score).abs() < tol, "{}: delta/full diff {}", metric.id, (unit.full(&st) - st.score).abs());
        }
    }
    let m04 = sc.objective("M04").unwrap();
    let st = m04.identity();
    let raw = breakdown(&m04, &st).metrics.iter().find(|m| m.id == "M04").unwrap().value;
    assert!((st.score - gecko_core::scorer::hand_cost(&st.load)).abs() < 1e-9);
    assert!((st.score - raw).abs() > 1.0, "target cost differs from raw left-hand share");
    let rolls = sc.objective("T03").unwrap();
    let st = rolls.identity();
    let raw = breakdown(&rolls, &st).metrics.iter().find(|m| m.id == "T03").unwrap().value;
    assert!((st.score + raw).abs() < 1e-8, "max direction is negated");
}

#[test]
fn nsga_front_equals_bruteforce_on_a_tiny_min_max_tradeoff() {
    let (g, c, l, p) = tiny(false);
    let sc = Scorer::new(&g, &c, &p, &l).unwrap();
    let ctx = Ctx::new(&sc, (0..sc.n).collect(), sc.identity());
    let unit = [sc.objective("M01").unwrap(), sc.objective("T03").unwrap()];
    let mut all = Vec::new();
    permutations(&mut [0, 1, 2, 3], 0, &mut all);
    let costs: Vec<_> = all.iter().map(|sym| unit.iter().map(|sc| sc.state_from_sym(sym.clone()).score).collect::<Vec<_>>()).collect();
    let expected: BTreeSet<_> =
        costs.iter().filter(|cost| !costs.iter().any(|other| dominates(other, cost))).map(|cost| cost_key(cost)).collect();
    assert!(expected.len() >= 2, "fixture must have a real min/max tradeoff: {expected:?}");
    let out = optimize::run(&ctx, &params(Algo::Nsga2), Duration::from_secs(60), &Control::default(), |_| {});
    let actual: BTreeSet<_> = out.front.iter().map(|p| cost_key(&p.costs)).collect();
    assert_eq!(actual, expected);
    assert_eq!(out.evals, 20_000);
    assert_eq!(out.params.threads, 1);
    for point in &out.front {
        assert!((point.raw[0] - point.costs[0]).abs() < 1e-8);
        assert!((point.raw[1] + point.costs[1]).abs() < 1e-8);
    }
    let mut capped = params(Algo::Nsga2);
    capped.nsga2.archive_size = 2;
    let cap = optimize::run(&ctx, &capped, Duration::from_secs(60), &Control::default(), |_| {});
    assert_eq!(cap.front.len(), 2);
    for dim in 0..2 {
        let a = out.front.iter().map(|p| p.costs[dim]).fold(f64::INFINITY, f64::min);
        let b = cap.front.iter().map(|p| p.costs[dim]).fold(f64::INFINITY, f64::min);
        assert!((a - b).abs() < 1e-8, "crowding retains objective extremes");
    }
}

#[test]
fn both_searches_are_deterministic_pinned_and_return_nondominated_costs() {
    let (g, c, l, p) = tiny(true);
    let sc = Scorer::new(&g, &c, &p, &l).unwrap();
    let ctx = Ctx::new(&sc, vec![0, 1, 2, 3], sc.identity());
    for algo in [Algo::Nsga2, Algo::Rwsa] {
        let mut p = params(algo);
        p.sa.steps = 500;
        p.nsga2.objectives = vec!["M04".into(), "M09".into(), "T03".into()];
        let a = optimize::run(&ctx, &p, Duration::from_secs(60), &Control::default(), |_| {});
        let b = optimize::run(&ctx, &p, Duration::from_secs(60), &Control::default(), |_| {});
        assert_eq!(a.evals, p.evals.unwrap());
        assert_eq!(serde_json::to_value(&a.front).unwrap(), serde_json::to_value(&b.front).unwrap());
        assert!(!a.front.is_empty());
        for point in &a.front {
            assert_eq!(point.sym[4..], ctx.start.sym[4..]);
            assert!(!a.front.iter().any(|other| dominates(&other.costs, &point.costs)));
            for (i, id) in p.nsga2.objectives.iter().enumerate() {
                let unit = sc.objective(id).unwrap();
                assert!((unit.state_from_sym(point.sym.clone()).score - point.costs[i]).abs() < 1e-8);
            }
        }
        assert!(a.objectives[0].cost.contains("50%"));
        assert!(a.objectives[1].cost.contains("thumb_target"));
    }
    // No movable pair: return the starting point without spinning or consuming a fake budget.
    let fixed = Ctx::new(&sc, vec![0], sc.identity());
    for algo in [Algo::Nsga2, Algo::Rwsa] {
        let out = optimize::run(&fixed, &params(algo), Duration::from_secs(60), &Control::default(), |_| {});
        assert_eq!(out.evals, 0);
        assert_eq!(out.front.len(), 1);
    }
}

#[test]
fn saved_front_replays_and_tampering_is_rejected() {
    let mut store = Store::open(Path::new(ROOT)).unwrap();
    store.runs = std::env::temp_dir().join(format!("gecko-test-pareto-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&store.runs);
    let g = store.geometry("ansi-30").unwrap();
    let c = store.corpus("english").unwrap();
    let p = store.profile("default").unwrap();
    let start = store.layout("qwerty", &g.name).unwrap().fit(&g).unwrap();
    for algo in [Algo::Nsga2, Algo::Rwsa] {
        let mut params = params(algo);
        params.evals = Some(2_000);
        params.nsga2.pop_size = 12;
        params.nsga2.archive_size = 24;
        params.sa.steps = 400;
        let spec = RunSpec { geometry: &g, corpus: &c, profile: &p, start: &start, pins: ",./;", params };
        let prep = run::prepare(&store.runs, &spec).unwrap();
        let result = run::execute(&prep, &spec, &Control::default(), |_| {}).unwrap();
        assert!(!result.front.is_empty());
        for point in &result.front {
            let chars: Vec<_> = point.layout.chars.chars().collect();
            for pin in ",./;".chars() {
                let at = start.chars.iter().position(|&c| c == pin).unwrap();
                assert_eq!(chars[at], pin);
            }
        }
        let verified = run::verify(&store, &prep.dir).unwrap();
        assert!(verified.ok, "{:#?}", verified.checks);
        let mut tampered = result;
        tampered.front[0].costs[0] += 0.1;
        std::fs::write(prep.dir.join("best.json"), serde_json::to_string(&tampered).unwrap()).unwrap();
        let verified = run::verify(&store, &prep.dir).unwrap();
        assert!(!verified.ok);
        assert!(verified.checks.iter().any(|c| c.name.starts_with("front#") && !c.ok));
    }
    let mut invalid = params(Algo::Nsga2);
    invalid.nsga2.objectives = vec!["B01".into(), "B01".into()];
    assert!(gecko_core::optimize::nsga2::validate(&invalid.nsga2).is_err());
    invalid.nsga2.objectives = vec!["B01".into(), "B02".into()]; // B02 is informational.
    assert!(gecko_core::optimize::nsga2::validate(&invalid.nsga2).is_err());
    let _ = std::fs::remove_dir_all(&store.runs);
}
