//! Explore invariants: re-scoring raw metric vectors == full evaluation under any weights;
//! the archive finds exactly the local optima of a brute-forceable board; atlas files are
//! consistent. See `context/algorithms/explore.md`.

use std::time::Duration;

use gecko_core::explore::archive::raw_vector;
use gecko_core::explore::stats::Weights;
use gecko_core::explore::{atlas, distance, metric_ids};
use gecko_core::optimize::{Algo, Control, Ctx, Params};
use gecko_core::rng::Rng;
use gecko_core::{Corpus, Geometry, Layout, Profile, Scorer};
use proptest::prelude::*;

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn read(p: &str) -> String {
    std::fs::read_to_string(format!("{ROOT}/{p}")).unwrap()
}

fn ansi() -> Geometry {
    let mut g = Geometry::from_toml(&read("data/geometries/ansi-30.toml")).unwrap();
    for (metric, table) in g.comfort.clone() {
        g.load_comfort(&metric, &read(&format!("data/comfort/{table}.csv"))).unwrap();
    }
    g
}

fn synthetic(charset: &str, seed: u64) -> Corpus {
    let cs: Vec<char> = charset.chars().collect();
    let mut rng = Rng::new(seed);
    let mut text = String::new();
    for _ in 0..20_000 {
        for _ in 0..1 + rng.below(7) {
            text.push(cs[(rng.below(cs.len()) * rng.below(cs.len())) / cs.len()]);
        }
        text.push(' ');
    }
    let mut c = Corpus::new("synthetic", charset);
    c.add_text(&text);
    c
}

fn profile_with(seed: u64) -> Profile {
    let mut p = Profile::from_toml(&read("data/profiles/default.toml")).unwrap();
    let mut rng = Rng::new(seed);
    for d in gecko_core::metrics::METRICS {
        // Some metrics off, the rest random, including ones the base profile ignores.
        let w = if rng.below(4) == 0 { 0.0 } else { (rng.f64() * 2.0) as f32 };
        p.weights.insert(d.id.to_string(), w);
    }
    p
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]
    /// Raw vectors measured under one profile re-score exactly under any other with the same tables.
    #[test]
    fn rescore_matches_full(seed in 0u64..10_000, wseed in 0u64..10_000) {
        let g = ansi();
        let c = synthetic(gecko_core::corpus::DEFAULT_CHARSET, 3);
        let start = Layout::from_toml(&read("data/layouts/qwerty.toml")).unwrap();
        let base = Profile::from_toml(&read("data/profiles/default.toml")).unwrap();
        let other = profile_with(wseed);
        prop_assert_eq!(base.table_hash(), other.table_hash());
        let sa = Scorer::new(&g, &c, &base, &start).unwrap();
        let sb = Scorer::new(&g, &c, &other, &start).unwrap();
        let st = Ctx::new(&sa, (0..30).collect(), sa.identity()).random_state(&mut Rng::new(seed));
        let raw = raw_vector(&sa, &st);
        let want = sb.state_from_sym(st.sym.clone()).score;
        let got = Weights::new(&other, &metric_ids()).score(&raw);
        prop_assert!((got - want).abs() < 2e-5 * want.abs().max(1.0), "rescored {got} vs full {want}");
        let own = Weights::new(&base, &metric_ids()).score(&raw);
        prop_assert!((own - st.score).abs() < 2e-5 * st.score.abs().max(1.0));
    }
}

/// 6 keys, mirror-symmetric: small enough to enumerate every local optimum.
fn tiny() -> Geometry {
    let mut s = String::from("name = \"tiny\"\n");
    let keys = [
        ("LM", 0.0, 0.0, 1.5),
        ("LI", 1.0, 0.0, 1.2),
        ("LI", 1.0, 1.0, 1.0),
        ("RI", 3.0, 1.0, 1.0),
        ("RI", 3.0, 0.0, 1.2),
        ("RM", 4.0, 0.0, 1.5),
    ];
    for (i, (f, x, y, e)) in keys.iter().enumerate() {
        s += &format!("[[key]]\npos = [{x}, {y}, 0.0]\nfinger = \"{f}\"\nrow = {}\ncol = {i}\neffort = {e}\n", *y as i8);
    }
    Geometry::from_toml(&s).unwrap()
}

fn permutations(v: Vec<u8>) -> Vec<Vec<u8>> {
    if v.len() <= 1 {
        return vec![v];
    }
    let mut out = Vec::new();
    for i in 0..v.len() {
        let mut rest = v.clone();
        let x = rest.remove(i);
        for mut p in permutations(rest) {
            p.insert(0, x);
            out.push(p);
        }
    }
    out
}

fn temp_dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("gecko-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn explore_finds_every_local_optimum_of_a_tiny_board() {
    let g = tiny();
    let c = synthetic("etaoin", 11);
    let mut prof = Profile::from_toml(&read("data/profiles/default.toml")).unwrap();
    prof.weights.retain(|k, _| ["B01", "M01", "B08", "M04"].contains(&k.as_str()));
    let start = Layout::from_str_flat("start", "tiny", "etaoin").unwrap();
    let sc = Scorer::new(&g, &c, &prof, &start).unwrap();
    let mirror = distance::score_symmetry(&sc);
    assert!(mirror.is_some(), "tiny board should be mirror-symmetric");

    // Brute force: every layout with no improving swap, canonicalized.
    let ctx = Ctx::new(&sc, (0..6).collect(), sc.identity());
    let mut want: Vec<Vec<u8>> = permutations((0..6).collect())
        .into_iter()
        .filter(|p| {
            let st = sc.state_from_sym(p.clone());
            (0..6).all(|a| (a + 1..6).all(|b| sc.delta(&st, a, b) >= -1e-9))
        })
        .map(|p| distance::canonical(&p, mirror.as_deref()))
        .collect();
    want.sort();
    want.dedup();

    let dir = temp_dir("tiny");
    let mut p = Params { algo: Algo::Explore, seed: 5, threads: 2, time_ms: None, evals: Some(300_000), top: 5, ..Default::default() };
    p.explore.samples = 50;
    p.explore.walks = 4;
    p.bls.exclusion_dist = 3;
    let (out, summary) = gecko_core::explore::run(&ctx, &p, Duration::from_secs(60), &Control::default(), &dir, "tiny", |_| {}).unwrap();
    let (slots, metrics, optima) = atlas::read_optima(&std::fs::read(dir.join("optima.bin")).unwrap(), None).unwrap();
    assert_eq!((slots, metrics.len()), (6, metric_ids().len()));
    let mut got: Vec<Vec<u8>> = optima.iter().filter(|o| o.flags & atlas::CONSTRAINED == 0).map(|o| o.sym.clone()).collect();
    got.sort();
    assert_eq!(got, want, "archive should hold exactly the local optima");
    assert!(summary.symmetric && summary.done);
    assert!((optima[0].score - out.top[0].score).abs() < 1e-9, "best optimum is the run's best");
    assert!(optima.windows(2).all(|w| w[0].score <= w[1].score), "sorted by score");
    let edges = atlas::read_edges(&std::fs::read(dir.join("edges.bin")).unwrap(), optima.len()).unwrap();
    assert_eq!(edges.len(), summary.counts.edges);
    assert!(!edges.is_empty());
    let samples = atlas::read_samples(&std::fs::read(dir.join("samples.bin")).unwrap()).unwrap();
    assert_eq!(samples.len(), 50);
    let hits: u64 = optima.iter().map(|o| o.hits as u64).sum();
    assert_eq!(hits, summary.counts.hits);
    for o in &optima {
        let st = sc.state_from_sym(o.sym.clone());
        assert!((st.score - o.score).abs() < 1e-9);
        assert!((Weights::new(&prof, &metrics).score(&o.raw) - o.score).abs() < 1e-4);
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn explore_on_ansi_writes_a_consistent_atlas() {
    let g = ansi();
    let c = synthetic(gecko_core::corpus::DEFAULT_CHARSET, 2);
    let prof = Profile::from_toml(&read("data/profiles/default.toml")).unwrap();
    let start = Layout::from_toml(&read("data/layouts/qwerty.toml")).unwrap();
    let sc = Scorer::new(&g, &c, &prof, &start).unwrap();
    assert!(distance::score_symmetry(&sc).is_none(), "row stagger breaks the mirror");
    let pinned: Vec<usize> = ",./;".chars().map(|ch| sc.chars.iter().position(|&x| x == ch).unwrap()).collect();
    let ctx = Ctx::new(&sc, (0..30).filter(|s| !pinned.contains(s)).collect(), sc.identity());
    let dir = temp_dir("ansi");
    let mut p = Params { algo: Algo::Explore, seed: 1, threads: 2, time_ms: None, evals: Some(2_000_000), top: 5, ..Default::default() };
    p.explore.samples = 40;
    p.explore.walks = 4;
    p.bls.exclusion = 1.0;
    p.bls.stagnation = 10;
    let (_, summary) = gecko_core::explore::run(&ctx, &p, Duration::from_secs(60), &Control::default(), &dir, "ansi", |_| {}).unwrap();
    let (_, _, optima) = atlas::read_optima(&std::fs::read(dir.join("optima.bin")).unwrap(), None).unwrap();
    assert!(optima.len() > 5, "{} optima", optima.len());
    assert!(summary.counts.constrained > 0, "exclusion restarts ran");
    assert!(summary.probe.corr_length.is_finite() && summary.probe.corr_length > 0.0);
    assert!(summary.fdc.is_finite());
    for o in optima.iter().filter(|o| o.flags & atlas::CONSTRAINED == 0).take(20) {
        for &x in &pinned {
            assert_eq!(o.sym[ctx.start.pos[x] as usize], x as u8, "pins respected");
        }
        let st = sc.state_from_sym(o.sym.clone());
        for a in 0..ctx.movable.len() {
            for b in a + 1..ctx.movable.len() {
                assert!(sc.delta(&st, ctx.movable[a], ctx.movable[b]) >= -1e-6, "not a local optimum");
            }
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn path_relinking_reaches_the_target() {
    let g = ansi();
    let c = synthetic(gecko_core::corpus::DEFAULT_CHARSET, 6);
    let start = Layout::from_toml(&read("data/layouts/qwerty.toml")).unwrap();
    let p = Profile::from_toml(&read("data/profiles/default.toml")).unwrap();
    let sc = Scorer::new(&g, &c, &p, &start).unwrap();
    let ctx = Ctx::new(&sc, (0..30).collect(), sc.identity());
    for seed in 0..8 {
        let (a, b) = (ctx.random_state(&mut Rng::new(seed)), ctx.random_state(&mut Rng::new(seed + 100)));
        let path = gecko_core::explore::path::relink(&sc, &a.sym, &b.sym).unwrap();
        // Every step fixes at least one slot, so the path is never longer than the Hamming distance.
        assert!(path.steps.len() <= distance::hamming(&a.sym, &b.sym) && path.steps.len() >= distance::swaps(&a.sym, &b.sym));
        let mut sym = a.sym.clone();
        for s in &path.steps {
            sym.swap(s.a, s.b);
            let want = sc.state_from_sym(sym.clone()).score;
            assert!((s.score - want).abs() < 1e-6 * want.abs().max(1.0), "step score {} vs full {want}", s.score);
        }
        assert_eq!(sym, b.sym);
        assert!(path.barrier >= -1e-9);
    }
    assert!(gecko_core::explore::path::relink(&sc, &start_sym(&sc), &[0; 30]).is_err());
    assert!(gecko_core::explore::path::relink(&sc, &start_sym(&sc), &start_sym(&sc)).unwrap().steps.is_empty());
}

fn start_sym(sc: &Scorer) -> Vec<u8> {
    sc.identity().sym
}
