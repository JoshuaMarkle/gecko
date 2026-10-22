//! Exact sub-problem solver (`gecko_core::certify`): equals exhaustive enumeration for both
//! certificate kinds, respects pins, is deterministic, and "already optimal" survives every swap.

use gecko_core::certify::{certify, default_free, Kind, Limits};
use gecko_core::rng::Rng;
use gecko_core::{Corpus, Geometry, Layout, Profile, Scorer, State};

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

fn default_profile() -> Profile {
    Profile::from_toml(&read("data/profiles/default.toml")).unwrap()
}

/// Every metric weighted, so trigram, load and placement terms all take part.
fn all_weights() -> Profile {
    let mut p = default_profile();
    p.familiar_to = Some("qwerty".into());
    for (i, d) in gecko_core::metrics::METRICS.iter().enumerate() {
        p.weights.insert(d.id.to_string(), 0.1 + i as f32 * 0.07);
    }
    p
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

/// The 6-key board of `tests/scorer.rs`.
fn tiny() -> Geometry {
    let mut s = String::from("name = \"tiny\"\n");
    let keys = [("LM", 0.0, 0.0, 1.5), ("LI", 1.0, 0.0, 1.2), ("LI", 1.0, 1.0, 1.0), ("RI", 3.0, 1.0, 1.0), ("RI", 3.0, 0.0, 1.2), ("RM", 4.0, 0.0, 1.5)];
    for (i, (f, x, y, e)) in keys.iter().enumerate() {
        s += &format!(
            "[[key]]\npos = [{x}, {y}, 0.0]\nfinger = \"{f}\"\nrow = {}\ncol = {i}\neffort = {e}\nhome = {}\n",
            *y as i8,
            *y > 0.5
        );
    }
    Geometry::from_toml(&s).unwrap()
}

/// Minimum of `kind` over every placement of `free` among their slots (others stay put).
fn brute(sc: &Scorer, st: &State, free: &[usize], kind: Kind) -> f64 {
    fn rec(sc: &Scorer, sym: &mut Vec<u8>, slots: &[usize], i: usize, kind: Kind, best: &mut f64) {
        if i == slots.len() {
            *best = best.min(kind.value(sc, &sc.state_from_sym(sym.clone())));
            return;
        }
        for j in i..slots.len() {
            sym.swap(slots[i], slots[j]);
            rec(sc, sym, slots, i + 1, kind, best);
            sym.swap(slots[i], slots[j]);
        }
    }
    let slots: Vec<usize> = free.iter().map(|&x| st.pos[x] as usize).collect();
    let mut best = f64::INFINITY;
    rec(sc, &mut st.sym.clone(), &slots, 0, kind, &mut best);
    best
}

fn check(sc: &Scorer, st: &State, free: &[usize]) {
    for kind in [Kind::Full, Kind::Pair] {
        let c = certify(sc, st, free, kind, Limits::default()).unwrap();
        let opt = brute(sc, st, free, kind);
        assert!(c.certified && c.stopped.is_none());
        assert!((c.optimum - opt).abs() <= 1e-9 * opt.abs().max(1.0), "{kind:?} {free:?}: {} vs brute force {opt}", c.optimum);
        assert_eq!(c.lower, c.optimum);
        assert!(c.root_bound <= opt + 1e-6, "root bound {} above optimum {opt}", c.root_bound);
        // Pins: every non-free symbol keeps its slot; the reported value is the layout's.
        let best = sc.state_from_sym(c.sym.clone());
        for x in (0..sc.n).filter(|x| !free.contains(x)) {
            assert_eq!(best.pos[x], st.pos[x], "pinned symbol {x} moved");
        }
        assert_eq!(kind.value(sc, &best), c.optimum);
        assert_eq!(c.improved, c.optimum < c.start - c.tol);
    }
}

#[test]
fn equals_brute_force_on_the_tiny_board() {
    let g = tiny();
    for seed in [11, 12, 13] {
        let c = synthetic("etaoin", seed);
        let start = Layout::from_str_flat("start", "tiny", "etaoin").unwrap();
        let sc = Scorer::new(&g, &c, &all_weights(), &start).unwrap();
        let st = sc.identity();
        check(&sc, &st, &[0, 1, 2, 3, 4, 5]);
        check(&sc, &st, &[0, 2, 3, 5]);
        check(&sc, &st, &[4, 1]);
    }
}

#[test]
fn equals_brute_force_on_ansi() {
    let g = ansi();
    let l = Layout::from_toml(&read("data/layouts/qwerty.toml")).unwrap();
    let c = synthetic(gecko_core::corpus::DEFAULT_CHARSET, 4);
    let mut rng = Rng::new(9);
    for (p, k) in [(all_weights(), 5), (default_profile(), 6), (default_profile(), 7)] {
        let sc = Scorer::new(&g, &c, &p, &l).unwrap();
        let mut sym: Vec<u8> = (0..30).collect();
        rng.shuffle(&mut sym);
        let st = sc.state_from_sym(sym);
        let mut free: Vec<usize> = (0..30).collect();
        rng.shuffle(&mut free);
        free.truncate(k);
        check(&sc, &st, &free);
        // A random layout is far from optimal: the search must find (and prove) a better one.
        assert!(certify(&sc, &st, &free, Kind::Full, Limits::default()).unwrap().improved);
    }
}

#[test]
fn already_optimal_means_no_improving_swap() {
    let g = ansi();
    let l = Layout::from_toml(&read("data/layouts/qwerty.toml")).unwrap();
    let c = synthetic(gecko_core::corpus::DEFAULT_CHARSET, 5);
    let sc = Scorer::new(&g, &c, &default_profile(), &l).unwrap();
    let free = default_free(&sc, &(0..30).collect::<Vec<_>>(), 6);
    for kind in [Kind::Full, Kind::Pair] {
        // Certify once, then certify the optimum again: it must report nothing to improve.
        let first = certify(&sc, &sc.identity(), &free, kind, Limits::default()).unwrap();
        let st = sc.state_from_sym(first.sym.clone());
        let c = certify(&sc, &st, &free, kind, Limits::default()).unwrap();
        assert!(c.certified && !c.improved, "{c:?}");
        let v = kind.value(&sc, &st);
        for &a in &free {
            for &b in &free {
                let mut s = st.clone();
                let d = sc.delta(&s, a, b);
                sc.apply(&mut s, a, b, d);
                let swapped = kind.value(&sc, &sc.state_from_sym(s.sym.clone()));
                assert!(swapped >= v - c.tol, "{kind:?}: swapping {a} and {b} improves {v} to {swapped}");
            }
        }
    }
}

#[test]
fn deterministic_and_limits_give_a_valid_lower_bound() {
    let g = ansi();
    let l = Layout::from_toml(&read("data/layouts/qwerty.toml")).unwrap();
    let c = synthetic(gecko_core::corpus::DEFAULT_CHARSET, 6);
    let sc = Scorer::new(&g, &c, &default_profile(), &l).unwrap();
    let st = sc.identity();
    let free = default_free(&sc, &(0..30).collect::<Vec<_>>(), 7);
    let a = certify(&sc, &st, &free, Kind::Full, Limits::default()).unwrap();
    let b = certify(&sc, &st, &free, Kind::Full, Limits::default()).unwrap();
    assert_eq!((a.sym.clone(), a.nodes, a.leaves, a.pruned), (b.sym.clone(), b.nodes, b.leaves, b.pruned));
    assert_eq!(a.free, b.free);
    // A node limit stops early: best-so-far and a bound that brackets the true optimum.
    let cut = certify(&sc, &st, &free, Kind::Full, Limits { nodes: Some(a.nodes / 3), ..Default::default() }).unwrap();
    assert!(!cut.certified && cut.stopped.as_deref() == Some("nodes"));
    assert!(cut.lower <= a.optimum + 1e-9 && cut.optimum >= a.optimum - 1e-9, "{cut:?} vs {a:?}");
}

#[test]
fn rejects_bad_free_sets() {
    let g = tiny();
    let c = synthetic("etaoin", 1);
    let start = Layout::from_str_flat("start", "tiny", "etaoin").unwrap();
    let sc = Scorer::new(&g, &c, &default_profile(), &start).unwrap();
    assert!(certify(&sc, &sc.identity(), &[1, 1], Kind::Full, Limits::default()).is_err());
    assert!(certify(&sc, &sc.identity(), &[9], Kind::Full, Limits::default()).is_err());
}

/// The real instance of `context/algorithms/bounds.md` (english, default profile, 10 free
/// symbols), against an independent threaded enumeration of all 3.6 M placements. Slow (~10 s
/// on 10 cores): `cargo test --release --test certify -- --ignored`.
#[test]
#[ignore]
fn equals_brute_force_with_ten_free_on_english() {
    let Ok(bytes) = std::fs::read(format!("{ROOT}/data/corpora/english.gkc")) else { return };
    let c = Corpus::from_bytes(&bytes).unwrap();
    // The SA 20 s seed 1 result of bounds.md.
    let l = Layout::from_str_flat("sa", "ansi-30", "xvndqjguo.rshtkpceailfbmzyw/;,").unwrap();
    let sc = Scorer::new(&ansi(), &c, &default_profile(), &l).unwrap();
    let st = sc.identity();
    let free: Vec<usize> = "nsrhtceaio".chars().map(|ch| sc.chars.iter().position(|&x| x == ch).unwrap()).collect();
    let slots: Vec<usize> = free.iter().map(|&x| st.pos[x] as usize).collect();
    // Thread t puts free[t] first and enumerates the other 9 symbols (Heap's algorithm).
    let (full, pair) = std::thread::scope(|s| {
        let hs: Vec<_> = (0..free.len())
            .map(|t| {
                let (sc, st, free, slots) = (&sc, &st, &free, &slots);
                s.spawn(move || {
                    let mut s = st.clone();
                    let mut rest: Vec<usize> = free.iter().copied().filter(|&x| x != free[t]).collect();
                    let (mut bf, mut bp) = (f64::INFINITY, f64::INFINITY);
                    let mut eval = |rest: &[usize], s: &mut State| {
                        s.pos[free[t]] = slots[0] as u8;
                        for (i, &x) in rest.iter().enumerate() {
                            s.pos[x] = slots[i + 1] as u8;
                        }
                        let (p, tri, load) = sc.parts(s);
                        bf = bf.min(p + tri + load);
                        bp = bp.min(p);
                    };
                    let k = rest.len();
                    let mut cnt = vec![0usize; k];
                    eval(&rest, &mut s);
                    let mut i = 0;
                    while i < k {
                        if cnt[i] < i {
                            if i % 2 == 0 { rest.swap(0, i) } else { rest.swap(cnt[i], i) }
                            eval(&rest, &mut s);
                            cnt[i] += 1;
                            i = 0;
                        } else {
                            cnt[i] = 0;
                            i += 1;
                        }
                    }
                    (bf, bp)
                })
            })
            .collect();
        hs.into_iter().map(|h| h.join().unwrap()).fold((f64::INFINITY, f64::INFINITY), |a, b| (a.0.min(b.0), a.1.min(b.1)))
    });
    for (kind, opt) in [(Kind::Full, full), (Kind::Pair, pair)] {
        let r = certify(&sc, &st, &free, kind, Limits::default()).unwrap();
        assert!((r.optimum - opt).abs() <= 1e-9 * opt.abs(), "{kind:?}: {} vs brute force {opt}", r.optimum);
    }
}
