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

/// Like `synthetic`, with every third word capitalized and every seventh in capitals, so the
/// corpus has Shift counts.
fn synthetic_cased(charset: &str, seed: u64) -> Corpus {
    let cs: Vec<char> = charset.chars().collect();
    let mut rng = Rng::new(seed);
    let mut text = String::new();
    for w in 0..20_000 {
        let len = 1 + rng.below(7);
        for k in 0..len {
            let c = cs[(rng.below(cs.len()) * rng.below(cs.len())) / cs.len()];
            text.push(if w % 7 == 0 || (w % 3 == 0 && k == 0) { c.to_ascii_uppercase() } else { c });
        }
        text.push(' ');
    }
    let mut c = Corpus::new("synthetic-cased", charset);
    c.add_text(&text);
    assert!(c.has_shift());
    c
}

/// Shift boards: corne-36 QWERTY with `⇧` on both outer thumbs, and the layered code layout
/// with `⇧` instead of Esc. Every metric weighted, so both Shift channels are exercised.
fn shift_scorer(layered: bool) -> Scorer {
    use gecko_core::layout::{EMPTY, ESC, SHIFT};
    let g = thumb_board("corne-36");
    let (mut l, cs) = if layered {
        (Layout::from_toml(&read("data/layouts/layered/corne-code.toml")).unwrap().fit(&g).unwrap(), gecko_core::corpus::CODE_CHARSET)
    } else {
        (layout("qwerty").fit(&g).unwrap(), gecko_core::corpus::DEFAULT_CHARSET)
    };
    for c in l.chars.iter_mut().filter(|c| **c == ESC || **c == EMPTY) {
        *c = SHIFT;
    }
    Scorer::new(&g, &synthetic_cased(cs, 11), &all_weights(), &l).unwrap()
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

    /// Tabu search's incremental Δ matrix: after swapping r/s, every disjoint pair's linear
    /// delta changes by exactly `delta_shift`, and delta = linear + load.
    #[test]
    fn delta_shift_matches_recompute(seed in 0u64..10_000, moves in proptest::collection::vec((0usize..30, 0usize..30), 1..8)) {
        let g = ansi();
        let c = synthetic(gecko_core::corpus::DEFAULT_CHARSET, 3);
        let sc = Scorer::new(&g, &c, &all_weights(), &layout("qwerty")).unwrap();
        let ctx = Ctx::new(&sc, (0..30).collect(), sc.identity());
        let mut st = ctx.random_state(&mut Rng::new(seed));
        for (r, s) in moves {
            if r == s {
                continue;
            }
            let before: Vec<f64> = (0..30 * 30).map(|k| sc.delta_linear(&st, k / 30, k % 30)).collect();
            let d = sc.delta(&st, r, s);
            prop_assert!((d - sc.delta_linear(&st, r, s) - sc.delta_load(&st, r, s)).abs() < 1e-9);
            sc.apply(&mut st, r, s, d);
            for u in 0..30 {
                for v in 0..30 {
                    if u == v || [r, s].contains(&u) || [r, s].contains(&v) {
                        continue;
                    }
                    let want = sc.delta_linear(&st, u, v);
                    let got = before[u * 30 + v] + sc.delta_shift(&st, r, s, u, v);
                    prop_assert!((want - got).abs() < 1e-5 * want.abs().max(1.0), "({u},{v}) after ({r},{s}): {got} vs {want}");
                }
            }
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

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]
    #[test]
    fn delta_matches_full_with_shift(seed in 0u64..10_000, layered: bool, swaps in proptest::collection::vec((0usize..128, 0usize..128), 1..40)) {
        let sc = shift_scorer(layered);
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
fn shift_costs_effort_and_y01() {
    use gecko_core::layout::SHIFT;
    let value = |sc: &Scorer, st: &gecko_core::State, id: &str| breakdown(sc, st).metrics.iter().find(|m| m.id == id).unwrap().value;
    let sc = shift_scorer(false);
    let st = sc.identity();
    let b = breakdown(&sc, &st);
    assert!((b.score - st.score).abs() < 1e-6 * st.score.abs().max(1.0));
    // Shift is the only layer-like press on this layout: Y01 = presses per 100 keystrokes
    // (×2 when Shift's finger is involved), M01 includes the Shift key's effort.
    let y01 = value(&sc, &st, "Y01");
    assert!(y01 > 5.0 && y01 < 60.0, "Y01 {y01}");
    let m01 = sc.shift_value("M01", &st);
    assert!(m01 > 0.0 && m01 < value(&sc, &st, "M01"), "Shift effort {m01}");
    // Shift keys never move.
    let ctx = Ctx::new(&sc, (0..sc.n).collect(), sc.identity());
    assert!(ctx.movable.iter().all(|&s| sc.chars[s] != SHIFT));
    assert_eq!(sc.chars.iter().filter(|&&c| c == SHIFT).count(), 2);
    // No ⇧ on the layout: Shift costs nothing.
    let g = thumb_board("corne-36");
    let plain = Scorer::new(&g, &synthetic_cased(gecko_core::corpus::DEFAULT_CHARSET, 11), &all_weights(), &layout("qwerty").fit(&g).unwrap()).unwrap();
    let pst = plain.identity();
    assert_eq!(value(&plain, &pst, "Y01"), 0.0);
    assert_eq!(plain.shift_value("M01", &pst), 0.0);
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
    for (got, want) in [(sum(&a.bigrams, "sfb"), value("B01")), (sum(&a.bigrams, "alternate"), value("B08")), (sum(&a.skipgrams, "sfs"), value("S01"))] {
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
fn movement_on_known_pairs() {
    use gecko_core::metrics::movement;
    let g = ansi();
    assert_eq!(movement(&g, at(1, 3), at(1, 6)), 0.0, "different hands");
    assert_eq!(movement(&g, at(0, 3), at(2, 4)), 0.0, "same finger");
    assert!(movement(&g, at(1, 1), at(1, 2)) < 1.0, "same-row roll is nearly free");
    assert!(movement(&g, at(1, 2), at(1, 4)) > 4.0 * movement(&g, at(1, 2), at(1, 3)), "LSB");
    // Two-row scissor costs 4× a one-row jump, before the direction factor.
    assert!(movement(&g, at(0, 2), at(2, 1)) > 3.0 * movement(&g, at(0, 2), at(1, 1)));
    // Finger-length rule: ring (short) up with middle (long) down is the bad scissor.
    assert!(movement(&g, at(0, 1), at(2, 2)) > movement(&g, at(2, 1), at(0, 2)));
    // Ring coupling: middle→ring costs more than KLO's seed (5) on a one-row jump.
    assert!(movement(&g, at(1, 2), at(0, 1)) > 5.0 * 1.1);
    // Mirror symmetry: the table is per finger, not per hand.
    assert_eq!(movement(&g, at(0, 1), at(2, 2)), movement(&g, at(0, 8), at(2, 7)));
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

