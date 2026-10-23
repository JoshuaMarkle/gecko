//! Weight robustness (`gecko_core::robust`): unit costs reproduce the score, mirror-invariant key
//! distance, families, rank confidence and a small weight sweep.

use std::path::Path;

use gecko_core::rng::Rng;
use gecko_core::robust::{self, families, key_distance, mirror_slots, rank_confidence, score_of, SweepSpec, UnitCosts};
use gecko_core::store::Store;
use gecko_core::{Layout, Profile, Scorer};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn store() -> Store {
    Store::open(Path::new(ROOT)).unwrap()
}

#[test]
fn unit_costs_reproduce_the_score() {
    let st = store();
    let g = st.geometry("ansi-30").unwrap();
    let c = st.corpus("english").unwrap();
    for name in ["default", "hand", "familiar"] {
        let p = st.profile(name).unwrap();
        let mut unit = UnitCosts::new(&g, &c, &p);
        for l in ["graphite", "qwerty", "dvorak"] {
            let layout = st.layout(l, &g.name).unwrap().fit(&g).unwrap();
            let sc = Scorer::new(&g, &c, &p, &layout).unwrap();
            let want = sc.identity().score;
            let got = score_of(&unit.of(&layout).unwrap(), &p);
            assert!((got - want).abs() < 1e-4 * want.abs().max(1.0), "{name}/{l}: {got} vs {want}");
        }
    }
}

#[test]
fn mirrored_layouts_are_one_arrangement() {
    let st = store();
    let g = st.geometry("ansi-30").unwrap();
    let m = mirror_slots(&g).expect("ansi-30 is symmetric");
    let q: Vec<char> = st.layout("qwerty", &g.name).unwrap().fit(&g).unwrap().chars;
    let mirrored: Vec<char> = (0..q.len()).map(|s| q[m[s]]).collect();
    assert_eq!(key_distance(&q, &mirrored, None), 30, "every key moves without the mirror");
    assert_eq!(key_distance(&q, &mirrored, Some(&m)), 0);
    let mut swapped = q.clone();
    swapped.swap(0, 1);
    assert_eq!(key_distance(&q, &swapped, Some(&m)), 2);
    let fam = families(&[q.clone(), swapped, mirrored, "abcdefghijklmnopqrstuvwxyz,./;".chars().collect()], 4, Some(&m));
    assert_eq!(fam, [0, 0, 0, 1]);
}

#[test]
fn rank_confidence_without_noise_is_certain() {
    let st = store();
    let g = st.geometry("ansi-30").unwrap();
    let c = st.corpus("english").unwrap();
    let p = st.profile("default").unwrap();
    let mut unit = UnitCosts::new(&g, &c, &p);
    let names: Vec<String> = ["qwerty", "graphite", "colemak"].map(String::from).to_vec();
    let costs: Vec<Vec<f64>> = names.iter().map(|n| unit.of(&st.layout(n, &g.name).unwrap().fit(&g).unwrap()).unwrap()).collect();
    let rows = rank_confidence(&names, &costs, &p, 0.0, 50, 1);
    assert_eq!(rows.iter().map(|r| r.layout.as_str()).collect::<Vec<_>>(), ["graphite", "colemak", "qwerty"]);
    assert!(rows.iter().all(|r| r.p_rank == 1.0 && r.rank_lo == r.rank && r.rank_hi == r.rank));
    assert_eq!(rows[0].p_first, 1.0);
    assert_eq!(rows[2].p_beats_next, None);
    // Noise moves weights but keeps the sign; QWERTY stays last under modest noise.
    let noisy = rank_confidence(&names, &costs, &p, 0.5, 500, 7);
    assert!(noisy[2].p_rank > 0.99, "{:?}", noisy[2]);
    let q = robust::jitter(&p, 0.0, &mut Rng::new(3));
    assert_eq!(q.weights, p.weights);
}

#[test]
fn sweep_reports_every_weight() {
    let st = store();
    let g = st.geometry("ansi-30").unwrap();
    let c = st.corpus("english").unwrap();
    let p = Profile::from_toml("name = \"two\"\n[weights]\nB01 = 1.0\nM01 = 0.5\n[scale]\nB01 = 0.79\nM01 = 0.038\n").unwrap();
    let start: Layout = st.layout("qwerty", &g.name).unwrap().fit(&g).unwrap();
    let spec = SweepSpec { sigma: 0.5, draws: 2, seed: 1, evals: 40_000, seeds: 1, threads: 2, family_keys: 10 };
    let mut calls = 0;
    let r = robust::sweep(&g, &c, &p, &start, &(0..30).collect::<Vec<_>>(), &spec, |_, _| calls += 1).unwrap();
    assert_eq!(calls, 1 + 2 * 3 + 2);
    assert_eq!(r.rows.len(), calls);
    assert_eq!(r.rows[0].variant, "base");
    assert_eq!((r.rows[0].regret, r.rows[0].keys_moved, r.rows[0].base_cost), (0.0, 0, 0.0));
    assert_eq!(r.sensitivity.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), ["B01", "M01"]);
    assert_eq!(r.rows.iter().find(|x| x.variant == "M01:drop").unwrap().weights["M01"], 0.0);
    assert!(r.rows.iter().all(|x| x.judges.contains_key("E01") && x.chars.chars().count() == 30));
    assert_eq!(r.families.iter().map(|f| f.count).sum::<usize>(), r.rows.len());
}
