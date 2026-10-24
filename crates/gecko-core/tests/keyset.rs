//! Keysets, Shift pairs, Ctrl and the thumb zone: delta == full, breakdown == score, the
//! keyed corpus and the Backspace model. See `context/keysets.md`.

use gecko_core::corpus::{join_paragraphs, CODE_CHARSET};
use gecko_core::keyset::{self, Mode, PlaceOpts, Spec};
use gecko_core::layout::{BACKSPACE, CTRL, SHIFT};
use gecko_core::optimize::Ctx;
use gecko_core::report::breakdown;
use gecko_core::rng::Rng;
use gecko_core::{Corpus, Geometry, Profile, Scorer};
use proptest::prelude::*;

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn read(p: &str) -> String {
    std::fs::read_to_string(format!("{ROOT}/{p}")).unwrap()
}

fn corne() -> Geometry {
    Geometry::from_toml(&read("data/geometries/corne-36.toml")).unwrap()
}

/// Every metric weighted, a thumb zone, and Ctrl presses.
fn all_weights() -> Profile {
    let mut p = Profile::from_toml(&read("data/profiles/default.toml")).unwrap();
    p.familiar_to = Some("qwerty".into());
    p.thumb_zone = Some("␣⌫⏎".into());
    p.base_letters = true;
    p.ctrl = 0.02;
    for (i, d) in gecko_core::metrics::METRICS.iter().enumerate() {
        p.weights.insert(d.id.to_string(), 0.1 + i as f32 * 0.07);
    }
    p
}

/// Code-like text over the full ASCII charset: capitals, shifted punctuation, `::` runs.
fn code_corpus(seed: u64) -> Corpus {
    let words = ["fn", "let", "Self", "match", "x", "Vec", "the", "and", "quiz", "jump", "back"];
    let syms = ["::", "()", "{", "}", ";", ",", ".", "->", "=", "_", "\"", "'", "<", ">", "/", "?", "!", "-", "+", "[", "]", ":"];
    let mut rng = Rng::new(seed);
    let mut text = String::new();
    for i in 0..20_000 {
        text.push_str(words[rng.below(words.len())]);
        text.push_str(syms[rng.below(syms.len())]);
        if i % 9 == 0 {
            text.push('\n');
        } else {
            text.push(' ');
        }
    }
    let mut c = Corpus::new("code-synth", CODE_CHARSET);
    c.add_text(&text);
    c
}

fn keyset_scorer(mode: Mode) -> Scorer {
    let g = corne();
    let c = code_corpus(1);
    let ks = keyset::build(&g, &c, &Spec::new(mode)).unwrap();
    Scorer::new(&g, &c, &all_weights(), &ks.layout).unwrap()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]
    /// Shift pairs (keyed corpus), Ctrl (folded into the place table), the thumb zone (L04) and
    /// a symbol layer keep deltas exact.
    #[test]
    fn delta_matches_full_with_keysets(mode in 0usize..3, seed in 0u64..10_000, swaps in proptest::collection::vec((0usize..200, 0usize..200), 1..40)) {
        let sc = keyset_scorer([Mode::Alpha, Mode::Shift, Mode::Layer][mode]);
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
fn breakdown_matches_score_and_rebuild() {
    for mode in [Mode::Alpha, Mode::Shift, Mode::Layer] {
        let sc = keyset_scorer(mode);
        let ctx = Ctx::new(&sc, (0..sc.n).collect(), sc.identity());
        let st = ctx.random_state(&mut Rng::new(7));
        let b = breakdown(&sc, &st);
        assert!((b.score - st.score).abs() < 1e-6 * st.score.abs().max(1.0), "{mode:?}: {} vs {}", b.score, st.score);
        // Re-costing the same board reproduces the scorer (Ctrl and L04 included).
        let mut moved = corne();
        moved.key[3].pos[0] += 0.2;
        let mut re = sc.clone();
        re.set_geometry(&moved).unwrap();
        let fresh = Scorer::new(&moved, &code_corpus(1), &all_weights(), &sc.layout_of(&sc.identity(), "x").unwrap()).unwrap();
        let (a, f) = (re.state_from_sym(st.sym.clone()), fresh.state_from_sym(st.sym.clone()));
        assert!((a.score - f.score).abs() < 1e-6 * f.score.abs().max(1.0), "{mode:?}: rebuild {} vs fresh {}", a.score, f.score);
    }
}

#[test]
fn ctrl_costs_only_with_a_ctrl_key() {
    let sc = keyset_scorer(Mode::Alpha);
    let st = sc.identity();
    assert!(sc.chars.contains(&CTRL));
    let m01 = sc.shift_value("M01", &st);
    assert!(m01 > 0.0);
    // Without ⎈, Ctrl costs nothing.
    let g = corne();
    let c = code_corpus(1);
    let mut l = keyset::build(&g, &c, &Spec::new(Mode::Alpha)).unwrap().layout;
    for ch in l.chars.iter_mut().filter(|ch| **ch == CTRL) {
        *ch = gecko_core::layout::EMPTY;
    }
    let p = all_weights();
    let plain = Scorer::new(&g, &c, &p, &l).unwrap();
    let mut no_shift = l.clone();
    for ch in no_shift.chars.iter_mut().filter(|ch| **ch == SHIFT) {
        *ch = gecko_core::layout::EMPTY;
    }
    let bare = Scorer::new(&g, &c, &p, &no_shift).unwrap();
    assert_eq!(bare.shift_value("M01", &bare.identity()), 0.0);
    assert!(plain.shift_value("M01", &plain.identity()) > 0.0, "Shift still costs");
    // The ctrl share scales the cost.
    let mut half = p.clone();
    half.ctrl = 0.01;
    let sc2 = Scorer::new(&g, &c, &half, &sc.layout_of(&st, "x").unwrap()).unwrap();
    let shift_only = Scorer::new(&g, &c, &Profile { ctrl: 0.0, ..p.clone() }, &sc.layout_of(&st, "x").unwrap()).unwrap();
    let base = shift_only.shift_value("M01", &shift_only.identity());
    let full = sc.shift_value("M01", &st) - base;
    let halved = sc2.shift_value("M01", &sc2.identity()) - base;
    assert!((full - 2.0 * halved).abs() < 1e-6, "{full} vs 2×{halved}");
}

#[test]
fn thumb_zone_counts_misplaced_keys() {
    let sc = keyset_scorer(Mode::Alpha);
    let st = sc.identity();
    let l04 = |sc: &Scorer, st: &gecko_core::State| breakdown(sc, st).metrics.iter().find(|m| m.id == "L04").unwrap().value;
    assert_eq!(l04(&sc, &st), 0.0, "keysets start inside the zone");
    let space = sc.chars.iter().position(|&c| c == gecko_core::layout::SPACE).unwrap();
    let e = sc.chars.iter().position(|&c| c == 'e').unwrap();
    let mut moved = st.clone();
    let d = sc.delta(&moved, space, e);
    sc.apply(&mut moved, space, e, d);
    assert_eq!(l04(&sc, &moved), 2.0, "Space off the thumbs and e on one");
    // With a symbol layer, a letter on a layer key is out of its zone too.
    let sc = keyset_scorer(Mode::Layer);
    let st = sc.identity();
    assert_eq!(l04(&sc, &st), 0.0);
    let on_layer = (0..sc.n).find(|&x| sc.geometry.key[st.pos[x] as usize].layer > 0 && sc.chars[x] == '(').unwrap();
    let e = sc.chars.iter().position(|&c| c == 'e').unwrap();
    let mut moved = st.clone();
    let d = sc.delta(&moved, on_layer, e);
    sc.apply(&mut moved, on_layer, e, d);
    assert_eq!(l04(&sc, &moved), 1.0, "e on the layer");
}

#[test]
fn keyed_corpus_moves_shifted_symbols_to_their_base() {
    let mut c = Corpus::new("t", "ab;:\u{2423}");
    c.add_text("a:: b; a: b:b");
    let k = c.keyed(&[(';', ':')]).unwrap();
    assert_eq!(k.charset, vec!['a', 'b', ';', '\u{2423}']);
    assert_eq!(k.total_chars(), c.total_chars());
    let semi = k.index_of(';').unwrap();
    assert_eq!(k.mono[semi], 5);
    // Presses: `a::` one (held through `::`), `a:` one, `b:` one.
    assert_eq!(k.shift[semi], 3);
    let (a, b) = (k.index_of('a').unwrap(), k.index_of('b').unwrap());
    assert_eq!(k.shift_bi[a * 4 + semi], 2);
    assert_eq!(k.shift_bi[b * 4 + semi], 1);
    // Pairs with nothing to merge change nothing.
    assert_eq!(c.keyed(&[('x', 'y')]).unwrap().hash(), c.hash());
    // A missing base takes the shifted symbol's place.
    let mut d = Corpus::new("t", "a:");
    d.add_text("a:a");
    let kd = d.keyed(&[(';', ':')]).unwrap();
    assert_eq!(kd.charset, vec!['a', ';']);
    assert_eq!(kd.shift[1], 1);
}

#[test]
fn backspace_model_hits_its_share() {
    let c = code_corpus(2);
    let e = c.with_edits("e", 0.056, 1).unwrap();
    let bs = e.index_of(BACKSPACE).unwrap();
    let share = e.mono[bs] as f64 / e.total_chars() as f64;
    assert!((share - 0.056).abs() < 1e-4, "{share}");
    let n = e.size();
    // Runs: ⌫⌫ bigrams are about 45% of ⌫ bigrams that continue (continuation 0.45).
    let after: u64 = (0..n).map(|y| e.bi[bs * n + y]).sum();
    let cont = e.bi[bs * n + bs] as f64 / after as f64;
    assert!((cont - 0.45).abs() < 0.03, "continuation {cont}");
    assert!(c.with_edits("e", 0.5, 1).is_err());
}

#[test]
fn paragraphs_join_wrapped_lines() {
    assert_eq!(join_paragraphs("One two\nthree.\n\n\n  Four\nfive\n"), "One two three.\nFour five");
    let mut c = Corpus::new("p", CODE_CHARSET);
    c.add_prose("a b\nc\n\nd");
    let enter = c.index_of('\u{23CE}').unwrap();
    assert_eq!(c.mono[enter], 1);
}

#[test]
fn placement_never_worsens_and_keeps_fixed_keys() {
    let g = corne();
    let c = code_corpus(3);
    let mut p = Profile::from_toml(&read("data/profiles/default.toml")).unwrap();
    p.thumb_zone = Some("␣⌫⏎".into());
    p.weights.insert("L04".into(), keyset::ZONE_WEIGHT);
    let ks = keyset::build(&g, &c, &Spec::new(Mode::Layer)).unwrap();
    let opts = PlaceOpts { evals: 40_000, rounds: 1, threads: 2, ..Default::default() };
    let r = keyset::place_fixed(&g, &c, &p, &ks.layout, &opts).unwrap();
    assert!(r.score <= r.start_score);
    assert!(r.tried > 0);
    let count = |l: &gecko_core::Layout, ch: char| l.chars.iter().filter(|&&c| c == ch).count();
    for ch in [SHIFT, CTRL, gecko_core::layout::LAYER_KEYS[0]] {
        assert_eq!(count(&r.layout, ch), count(&ks.layout, ch), "{ch}");
    }
    assert!(r.layout.flat(&g).is_ok());
    let sc = Scorer::new(&g, &c, &p, &r.layout).unwrap();
    assert!((sc.identity().score - r.score).abs() < 1e-6 * r.score.abs().max(1.0));
}
