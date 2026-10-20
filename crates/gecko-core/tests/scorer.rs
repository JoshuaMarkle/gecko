//! Core invariants: delta == full, folding == breakdown, optimizers find brute-force optima,
//! determinism, and ranking sanity on the bundled corpus. See `context/testing.md`.

use std::time::Duration;

use gecko_core::optimize::{self, Algo, Control, Ctx, Params};
use gecko_core::report::breakdown;
use gecko_core::rng::Rng;
use gecko_core::{Corpus, Geometry, Layout, Profile, Scorer};
use proptest::prelude::*;

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn read(p: &str) -> String {
    std::fs::read_to_string(format!("{ROOT}/{p}")).unwrap()
}

fn ansi() -> Geometry {
    Geometry::from_toml(&read("data/geometries/ansi-30.toml")).unwrap()
}

fn layout(name: &str) -> Layout {
    Layout::from_toml(&read(&format!("data/layouts/{name}.toml"))).unwrap()
}

/// Every metric weighted, so every code path in the scorer is exercised.
fn all_weights() -> Profile {
    let mut p = Profile::from_toml(&read("data/profiles/default.toml")).unwrap();
    for (i, d) in gecko_core::metrics::METRICS.iter().enumerate() {
        p.weights.insert(d.id.to_string(), 0.1 + i as f32 * 0.07);
    }
    p
}

/// Synthetic corpus: random words over the charset (independent of data files).
fn synthetic(charset: &str, seed: u64) -> Corpus {
    let cs: Vec<char> = charset.chars().collect();
    let mut rng = Rng::new(seed);
    let mut text = String::new();
    for _ in 0..20_000 {
        let len = 1 + rng.below(7);
        for _ in 0..len {
            // Skewed distribution so some symbols dominate.
            let i = (rng.below(cs.len()) * rng.below(cs.len())) / cs.len();
            text.push(cs[i]);
        }
        text.push(' ');
    }
    let mut c = Corpus::new("synthetic", charset);
    c.add_text(&text);
    c
}

fn real_corpus() -> Option<Corpus> {
    std::fs::read(format!("{ROOT}/data/corpora/english.gkc")).ok().map(|b| Corpus::from_bytes(&b).unwrap())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]
    #[test]
    fn delta_matches_full(seed in 0u64..10_000, swaps in proptest::collection::vec((0usize..30, 0usize..30), 1..40)) {
        let g = ansi();
        let c = synthetic(gecko_core::corpus::DEFAULT_CHARSET, 3);
        let sc = Scorer::new(&g, &c, &all_weights(), &layout("qwerty")).unwrap();
        let ctx = Ctx { scorer: &sc, movable: (0..30).collect(), start: sc.identity() };
        let mut st = ctx.random_state(&mut Rng::new(seed));
        for (a, b) in swaps {
            let before = sc.full(&st);
            let d = sc.delta(&st, a, b);
            sc.apply(&mut st, a, b, d);
            let after = sc.full(&st);
            let tol = 1e-5 * before.abs().max(1.0);
            prop_assert!((after - before - d).abs() < tol, "delta {d} vs full diff {}", after - before);
            prop_assert!((st.score - after).abs() < 1e-3 * after.abs().max(1.0));
            let loads = sc.loads(&st.pos);
            for (l, s) in loads.iter().zip(st.load.iter()) { prop_assert!((l - s).abs() < 1e-6); }
        }
    }
}

#[test]
fn folding_matches_breakdown() {
    let g = ansi();
    let c = synthetic(gecko_core::corpus::DEFAULT_CHARSET, 5);
    for name in ["qwerty", "graphite", "dvorak"] {
        let sc = Scorer::new(&g, &c, &all_weights(), &layout(name)).unwrap();
        let st = sc.identity();
        let b = breakdown(&sc, &st);
        assert!((b.score - st.score).abs() < 1e-6 * st.score.abs().max(1.0), "{name}: {} vs {}", b.score, st.score);
    }
}

/// A 6-key board: small enough to brute-force all 720 layouts.
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
        s += &format!(
            "[[key]]\npos = [{x}, {y}, 0.0]\nfinger = \"{f}\"\nrow = {}\ncol = {i}\neffort = {e}\nhome = {}\n",
            *y as i8,
            *y > 0.5
        );
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

#[test]
fn optimizers_find_brute_force_optimum() {
    let g = tiny();
    let c = synthetic("etaoin", 11);
    let start = Layout::from_str_flat("start", "tiny", "etaoin").unwrap();
    let sc = Scorer::new(&g, &c, &all_weights(), &start).unwrap();
    let best = permutations((0..6).collect()).into_iter().map(|p| sc.state_from_sym(p).score).fold(f64::INFINITY, f64::min);
    for algo in Algo::ALL {
        let mut p = Params { algo, seed: 3, threads: 2, time_ms: None, evals: Some(200_000), top: 3, ..Default::default() };
        p.sa.steps = 5_000;
        p.ga.pop_size = 16;
        let ctx = Ctx { scorer: &sc, movable: (0..6).collect(), start: sc.identity() };
        let out = optimize::run(&ctx, &p, Duration::from_secs(60), &Control::default(), |_| {});
        let got = out.top[0].score;
        assert!((got - best).abs() < 1e-6, "{}: {got} vs optimum {best}", algo.name());
    }
}

#[test]
fn deterministic_with_eval_budget() {
    let g = ansi();
    let c = synthetic(gecko_core::corpus::DEFAULT_CHARSET, 2);
    let sc = Scorer::new(&g, &c, &all_weights(), &layout("qwerty")).unwrap();
    for algo in Algo::ALL {
        let mut p = Params { algo, seed: 9, threads: 3, time_ms: None, evals: Some(60_000), top: 5, ..Default::default() };
        p.sa.steps = 10_000;
        p.ga.pop_size = 16;
        let run = || {
            let ctx = Ctx { scorer: &sc, movable: (0..30).collect(), start: sc.identity() };
            optimize::run(&ctx, &p, Duration::from_secs(60), &Control::default(), |_| {}).top
        };
        assert_eq!(run(), run(), "{} not deterministic", algo.name());
    }
}

#[test]
fn pins_are_respected() {
    let g = ansi();
    let c = synthetic(gecko_core::corpus::DEFAULT_CHARSET, 4);
    let sc = Scorer::new(&g, &c, &all_weights(), &layout("qwerty")).unwrap();
    let pinned: Vec<usize> = ",./;".chars().map(|ch| sc.chars.iter().position(|&x| x == ch).unwrap()).collect();
    let ctx = Ctx { scorer: &sc, movable: (0..30).filter(|s| !pinned.contains(s)).collect(), start: sc.identity() };
    let p = Params { algo: Algo::Sa, seed: 1, threads: 2, time_ms: None, evals: Some(100_000), top: 3, ..Default::default() };
    for s in optimize::run(&ctx, &p, Duration::from_secs(60), &Control::default(), |_| {}).top {
        for &x in &pinned {
            assert_eq!(s.sym[ctx.start.pos[x] as usize], x as u8);
        }
    }
}

#[test]
fn ranking_sanity_on_english() {
    let Some(c) = real_corpus() else { return };
    let g = ansi();
    let p = Profile::from_toml(&read("data/profiles/default.toml")).unwrap();
    let score = |n: &str| {
        let sc = Scorer::new(&g, &c, &p, &layout(n)).unwrap();
        sc.identity().score
    };
    let modern = ["graphite", "sturdy", "semimak"].map(score).into_iter().fold(f64::NEG_INFINITY, f64::max);
    let (colemak, dvorak, qwerty) = (score("colemak"), score("dvorak"), score("qwerty"));
    assert!(modern < colemak && colemak < dvorak && dvorak < qwerty, "modern {modern} colemak {colemak} dvorak {dvorak} qwerty {qwerty}");

    // Published QWERTY SFB is ≈ 6.6% on English prose.
    let sc = Scorer::new(&g, &c, &p, &layout("qwerty")).unwrap();
    let b = breakdown(&sc, &sc.identity());
    let sfb = b.metrics.iter().find(|m| m.id == "B01").unwrap().value;
    assert!((5.8..7.4).contains(&sfb), "QWERTY SFB {sfb}%");
}

#[test]
fn pause_stops_the_clock() {
    let g = ansi();
    let c = synthetic(gecko_core::corpus::DEFAULT_CHARSET, 2);
    let sc = Scorer::new(&g, &c, &all_weights(), &layout("qwerty")).unwrap();
    let ctx = Ctx { scorer: &sc, movable: (0..30).collect(), start: sc.identity() };
    let p = Params { algo: Algo::Hc, seed: 1, threads: 2, time_ms: Some(200), evals: None, top: 1, ..Default::default() };
    let control = Control::default();
    control.set_paused(true);
    let t = std::time::Instant::now();
    let out = std::thread::scope(|s| {
        s.spawn(|| {
            std::thread::sleep(Duration::from_millis(300));
            control.set_paused(false);
        });
        optimize::run(&ctx, &p, Duration::from_secs(60), &control, |_| {})
    });
    assert!(t.elapsed() >= Duration::from_millis(480), "paused time used the budget");
    assert!(out.elapsed_ms < 400, "elapsed_ms {} includes the pause", out.elapsed_ms);
}
