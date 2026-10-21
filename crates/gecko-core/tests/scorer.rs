//! Core invariants: delta == full, folding == breakdown, optimizers find brute-force optima,
//! determinism, and ranking sanity on the bundled corpus. See `context/testing.md`.

use std::time::Duration;

use gecko_core::optimize::{self, Algo, Control, Ctx, Params};
use gecko_core::report::{analyze, breakdown};
use gecko_core::rng::Rng;
use gecko_core::{Corpus, Geometry, Layout, Profile, Scorer};
use proptest::prelude::*;

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn read(p: &str) -> String {
    std::fs::read_to_string(format!("{ROOT}/{p}")).unwrap()
}

/// ansi-30 with its comfort tables, as the store loads them (so E04/E05 are exercised).
fn ansi() -> Geometry {
    let mut g = Geometry::from_toml(&read("data/geometries/ansi-30.toml")).unwrap();
    for (metric, table) in g.comfort.clone() {
        g.load_comfort(&metric, &read(&format!("data/comfort/{table}.csv"))).unwrap();
    }
    g
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
        let ctx = Ctx::new(&sc, (0..30).collect(), sc.identity());
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

    /// Thumb boards: tokens (Space on a thumb carries most load there) and empty slots move
    /// like any symbol.
    #[test]
    fn delta_matches_full_with_thumbs(seed in 0u64..10_000, board in 0usize..2, swaps in proptest::collection::vec((0usize..36, 0usize..36), 1..40)) {
        let g = thumb_board(["corne-36", "sweep-34"][board]);
        let c = synthetic(gecko_core::corpus::DEFAULT_CHARSET, 3);
        let sc = Scorer::new(&g, &c, &all_weights(), &layout("qwerty").fit(&g).unwrap()).unwrap();
        let ctx = Ctx::new(&sc, (0..sc.n).collect(), sc.identity());
        let mut st = ctx.random_state(&mut Rng::new(seed));
        for (a, b) in swaps {
            let (a, b) = (a % sc.n, b % sc.n);
            let before = sc.full(&st);
            let d = sc.delta(&st, a, b);
            sc.apply(&mut st, a, b, d);
            let after = sc.full(&st);
            prop_assert!((after - before - d).abs() < 1e-5 * before.abs().max(1.0), "delta {d} vs full diff {}", after - before);
        }
    }
}

/// Layered layouts: symbols move between layers and onto empty layer slots; Y01/Y02 weighted
/// (all_weights), so the code-symbol channel is exercised.
fn layered_scorer() -> (Scorer, Layout) {
    let g = thumb_board("corne-36");
    let l = Layout::from_toml(&read("data/layouts/layered/corne-code.toml")).unwrap().fit(&g).unwrap();
    let c = synthetic(gecko_core::corpus::CODE_CHARSET, 7);
    (Scorer::new(&g, &c, &all_weights(), &l).unwrap(), l)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]
    #[test]
    fn delta_matches_full_with_layers(seed in 0u64..10_000, swaps in proptest::collection::vec((0usize..128, 0usize..128), 1..40)) {
        let (sc, _) = layered_scorer();
        let ctx = Ctx::new(&sc, (0..sc.n).collect(), sc.identity());
        let mut st = ctx.random_state(&mut Rng::new(seed));
        for (a, b) in swaps {
            let (a, b) = (ctx.movable[a % ctx.movable.len()], ctx.movable[b % ctx.movable.len()]);
            let before = sc.full(&st);
            let d = sc.delta(&st, a, b);
            sc.apply(&mut st, a, b, d);
            let after = sc.full(&st);
            prop_assert!((after - before - d).abs() < 1e-5 * before.abs().max(1.0), "delta {d} vs full diff {}", after - before);
        }
    }
}

#[test]
fn layered_layouts_round_trip_and_score() {
    let (sc, l) = layered_scorer();
    let g = thumb_board("corne-36");
    assert_eq!(sc.n, 36 + 33 + 33);
    // Layer keys never move.
    let ctx = Ctx::new(&sc, (0..sc.n).collect(), sc.identity());
    assert!(ctx.movable.iter().all(|&s| !gecko_core::layout::is_layer_key(sc.chars[s])));
    // Flat string ↔ layers, and TOML round trip.
    let st = sc.identity();
    // Flat strings carry symbols, not layer names.
    let chars = |l: &Layout| l.layers.iter().map(|ly| ly.chars.clone()).collect::<Vec<_>>();
    assert_eq!(chars(&sc.layout_of(&st, &l.name).unwrap()), chars(&l));
    let back = Layout::from_toml(&l.to_toml(&g)).unwrap();
    assert_eq!(back.layers, l.layers);
    assert!((sc.state_for(&back).unwrap().score - st.score).abs() < 1e-9);
    let b = breakdown(&sc, &st);
    assert!((b.score - st.score).abs() < 1e-6 * st.score.abs().max(1.0));
    let y01 = b.metrics.iter().find(|m| m.id == "Y01").unwrap().value;
    assert!(y01 > 0.0 && y01 < 100.0, "Y01 {y01}");
    // A symbol on the holding finger's keys is rejected.
    let mut bad = l.clone();
    let lt = g.key.iter().position(|k| k.finger == gecko_core::Finger::LT).unwrap();
    let i = bad.layers[0].chars.iter().position(|&c| c == '!').unwrap();
    bad.layers[0].chars.swap(i, lt);
    assert!(bad.flat(&g).is_err());
}

fn thumb_board(name: &str) -> Geometry {
    Geometry::from_toml(&read(&format!("data/geometries/{name}.toml"))).unwrap()
}

#[test]
fn thumb_layouts_score_and_round_trip() {
    let c = synthetic(gecko_core::corpus::DEFAULT_CHARSET, 5);
    for name in ["corne-36", "sweep-34"] {
        let g = thumb_board(name);
        let l = layout("graphite").fit(&g).unwrap();
        let sc = Scorer::new(&g, &c, &all_weights(), &l).unwrap();
        let st = sc.identity();
        let b = breakdown(&sc, &st);
        assert!((b.score - st.score).abs() < 1e-6 * st.score.abs().max(1.0), "{name}: {} vs {}", b.score, st.score);
        // Space sits on a thumb and carries real load there.
        let space = sc.chars.iter().position(|&ch| ch == gecko_core::layout::SPACE).unwrap();
        assert!(g.key[st.pos[space] as usize].finger.is_thumb());
        assert!(st.load[4] + st.load[5] > 10.0);
        // Any arrangement, including empty slots holding letters, maps back to the same state.
        let mut st2 = sc.identity();
        let empty = sc.chars.iter().position(|&ch| ch == gecko_core::layout::EMPTY);
        if let Some(e) = empty {
            let d = sc.delta(&st2, 0, e);
            sc.apply(&mut st2, 0, e, d);
        }
        let back = sc.state_for(&Layout::from_chars("x", name, "", st2.chars(&sc)).unwrap()).unwrap();
        assert_eq!(back.chars(&sc), st2.chars(&sc));
        assert!((back.score - st2.score).abs() < 1e-6 * st2.score.abs().max(1.0));
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

#[test]
fn pair_kinds_match_metrics() {
    let g = ansi();
    let c = synthetic(gecko_core::corpus::DEFAULT_CHARSET, 7);
    let sc = Scorer::new(&g, &c, &all_weights(), &layout("qwerty")).unwrap();
    let a = analyze(&sc, &sc.identity(), 10);
    let value = |id: &str| a.breakdown.metrics.iter().find(|m| m.id == id).unwrap().value;
    let sum = |v: &[gecko_core::report::Pair], k: &str| v.iter().filter(|p| p.kind == k).map(|p| p.freq).sum::<f64>();
    for (got, want) in
        [(sum(&a.bigrams, "sfb"), value("B01")), (sum(&a.bigrams, "alternate"), value("B08")), (sum(&a.skipgrams, "sfs"), value("S01"))]
    {
        assert!((got - want).abs() < 1e-3, "{got} vs {want}");
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
        let ctx = Ctx::new(&sc, (0..6).collect(), sc.identity());
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
            let ctx = Ctx::new(&sc, (0..30).collect(), sc.identity());
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
    let ctx = Ctx::new(&sc, (0..30).filter(|s| !pinned.contains(s)).collect(), sc.identity());
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
    // Colemak vs Dvorak is no longer pinned: B14 rates Colemak's D/H stretches about level with
    // Dvorak, and measured comfort (E05) favours Dvorak's alternation; see context/decisions.md.
    let (colemak, dvorak, qwerty) = (score("colemak-dh"), score("dvorak"), score("qwerty"));
    assert!(
        modern < colemak.min(dvorak) && colemak.max(dvorak) < qwerty,
        "modern {modern} colemak-dh {colemak} dvorak {dvorak} qwerty {qwerty}"
    );

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
    let ctx = Ctx::new(&sc, (0..30).collect(), sc.identity());
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

/// ansi-30 slot for (row, col).
fn at(r: usize, c: usize) -> usize {
    r * 10 + c
}

#[test]
fn stretch_on_known_pairs() {
    use gecko_core::metrics::stretch;
    let g = ansi();
    assert_eq!(stretch(&g, at(1, 1), at(1, 2)), 0.0, "home-row neighbours");
    assert_eq!(stretch(&g, at(1, 2), at(1, 3)), 0.0, "middle + index on home");
    assert_eq!(stretch(&g, at(1, 3), at(1, 6)), 0.0, "different hands");
    assert!(stretch(&g, at(1, 2), at(1, 4)) > 0.0, "LSB: middle + index center column");
    assert!(stretch(&g, at(0, 1), at(2, 0)) > 0.0, "ring top + pinky bottom scissor");
    // Finger-length rule: the shorter finger reaching up is worse.
    assert!(stretch(&g, at(0, 3), at(2, 2)) > stretch(&g, at(0, 2), at(2, 3)));
    // Row stagger shifts the bottom row right, so the left ring/pinky scissor is the worse one.
    assert!(stretch(&g, at(0, 1), at(2, 0)) > stretch(&g, at(0, 8), at(2, 9)));
}

#[test]
fn engram_comfort_table() {
    let g = ansi();
    let t = &g.comfort_tables["E04"];
    let c = |a: usize, b: usize| t[a * 30 + b];
    assert_eq!(c(at(1, 2), at(1, 3)), 1.0, "home-row inward roll");
    assert!(c(at(0, 3), at(1, 3)) < 0.5, "same-finger reach");
    assert_eq!(c(at(1, 1), at(1, 1)), 1.0, "repeats are left to B02");
}

/// Hand-computed values for the C4 metrics (B06 B07 B09 B10 S05 T09 T10, M03 M07) on ansi-30.
#[test]
fn c4_metrics_on_known_keys() {
    use gecko_core::metrics::{def, stretch, table};
    let g = ansi();
    let p = Profile::from_toml(&read("data/profiles/default.toml")).unwrap();
    let t = |id: &str| table(&g, &p, def(id).unwrap()).unwrap();
    let n = 30;
    let bi = |id: &str, a: usize, b: usize| t(id)[a * n + b];
    // B06: the shorter finger (pinky < index < ring < middle) on the upper row, one row apart.
    assert_eq!(bi("B06", at(0, 0), at(1, 1)), 100.0, "pinky top, ring home");
    assert_eq!(bi("B06", at(1, 0), at(0, 1)), 0.0, "pinky home, ring top");
    assert_eq!(bi("B06", at(0, 1), at(1, 2)), 100.0, "ring top, middle home");
    assert_eq!(bi("B06", at(1, 3), at(0, 2)), 0.0, "index home, middle top");
    assert_eq!(bi("B06", at(0, 3), at(1, 2)), 100.0, "index top, middle home");
    assert_eq!(bi("B06", at(0, 9), at(1, 8)), 100.0, "right pinky top, ring home");
    assert_eq!(bi("B06", at(0, 0), at(2, 1)), 0.0, "two rows is a full scissor (B05)");
    // B07: inward rolls only.
    assert_eq!(bi("B07", at(1, 0), at(1, 1)), 100.0);
    assert_eq!(bi("B07", at(1, 1), at(1, 0)), 0.0);
    assert_eq!(bi("B07", at(1, 9), at(0, 6)), 100.0);
    assert_eq!(bi("B07", at(1, 3), at(1, 6)), 0.0, "different hands");
    // B09: pinky–ring on one hand, either direction.
    assert_eq!(bi("B09", at(1, 1), at(1, 0)), 100.0);
    assert_eq!(bi("B09", at(1, 9), at(0, 8)), 100.0);
    assert_eq!(bi("B09", at(1, 1), at(1, 2)), 0.0);
    assert_eq!(bi("B09", at(1, 0), at(1, 9)), 0.0, "different hands");
    // B10: |Δy| × 100 on one hand.
    assert_eq!(bi("B10", at(0, 0), at(2, 1)), 200.0);
    assert_eq!(bi("B10", at(0, 3), at(1, 3)), 100.0);
    assert_eq!(bi("B10", at(0, 0), at(2, 9)), 0.0, "different hands");
    assert_eq!(bi("B10", at(1, 2), at(1, 3)), 0.0);
    // S05 is B14's stretch on skipgrams.
    assert_eq!(bi("S05", at(1, 2), at(1, 4)), 100.0 * stretch(&g, at(1, 2), at(1, 4)));
    assert!(bi("S05", at(1, 2), at(1, 4)) > 0.0);
    // Trigrams.
    let tri = |id: &str, a: usize, b: usize, c: usize| t(id)[(a * n + b) * n + c];
    let s = stretch(&g, at(1, 2), at(1, 4)) + stretch(&g, at(1, 4), at(1, 2));
    assert!((tri("T09", at(1, 2), at(1, 4), at(1, 2)) - 100.0 * s).abs() < 1e-4, "LSB there and back");
    assert_eq!(tri("T09", at(1, 2), at(1, 4), at(1, 7)), 0.0, "second step changes hands");
    assert_eq!(tri("T09", at(1, 1), at(1, 2), at(1, 3)), 0.0, "home-row roll");
    assert_eq!(tri("T10", at(0, 1), at(2, 0), at(0, 1)), 100.0, "scissor twice");
    assert_eq!(tri("T10", at(0, 2), at(2, 2), at(2, 4)), 100.0, "SFB then LSB");
    assert_eq!(tri("T10", at(0, 2), at(2, 2), at(1, 7)), 0.0, "only one bad step");
    assert_eq!(tri("T10", at(1, 0), at(1, 1), at(1, 2)), 0.0);
}

#[test]
fn c4_load_metrics() {
    use gecko_core::scorer::{balance_rss, peak_load};
    let target = [8.0, 11.0, 15.0, 16.0, 0.0, 0.0, 16.0, 15.0, 11.0, 8.0];
    let even = [12.5, 12.5, 12.5, 12.5, 0.0, 0.0, 12.5, 12.5, 12.5, 12.5];
    assert!((balance_rss(&even) - 1250f64.sqrt()).abs() < 1e-9);
    // Thumb load is ignored: shares are of non-thumb keystrokes.
    let thumbs = even.map(|l| l * 0.8);
    let thumbs = {
        let mut t = thumbs;
        t[4] = 20.0;
        t
    };
    assert!((balance_rss(&thumbs) - 1250f64.sqrt()).abs() < 1e-9);
    // Pinkies at 12.5 against a target of 8 are the peak: 156.25%.
    assert!((peak_load(&even, &target) - 156.25).abs() < 1e-9);
    let mut on_target = target;
    on_target[4] = 0.0;
    assert!((peak_load(&on_target, &target) - 100.0).abs() < 1e-9);
    let mut skewed = target;
    skewed[0] = 24.0;
    skewed[3] = 0.0;
    // Shares: LP 24 of 100 → 300% of its 8% target; RSS = √(Σ shares²).
    assert!((peak_load(&skewed, &target) - 300.0).abs() < 1e-9);
    let rss = skewed.iter().map(|s| s * s).sum::<f64>().sqrt();
    assert!((balance_rss(&skewed) - rss).abs() < 1e-9);
}
