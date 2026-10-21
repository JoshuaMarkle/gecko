//! MAP-Elites invariants: every elite sits in the cell its descriptors map to, its stored score
//! and raw vector match a from-scratch evaluation, elites are local optima (full descent), pins
//! hold, and `--evals` runs on one thread are reproducible. See `context/algorithms/map-elites.md`.

use std::time::Duration;

use gecko_core::explore::stats::Weights;
use gecko_core::explore::{atlas, qd};
use gecko_core::optimize::{Algo, Control, Ctx, Params};
use gecko_core::rng::Rng;
use gecko_core::{Corpus, Geometry, Layout, Profile, Scorer};

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

fn temp_dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("gecko-test-qd-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn qd_grid_is_consistent_and_reproducible() {
    let g = ansi();
    let c = synthetic(gecko_core::corpus::DEFAULT_CHARSET, 3);
    let prof = Profile::from_toml(&read("data/profiles/default.toml")).unwrap();
    let start = Layout::from_toml(&read("data/layouts/qwerty.toml")).unwrap();
    let sc = Scorer::new(&g, &c, &prof, &start).unwrap();
    let pinned: Vec<usize> = ",./;".chars().map(|ch| sc.chars.iter().position(|&x| x == ch).unwrap()).collect();
    let ctx = Ctx::new(&sc, (0..30).filter(|s| !pinned.contains(s)).collect(), sc.identity());
    let mut p = Params { algo: Algo::Qd, seed: 4, threads: 1, time_ms: None, evals: Some(1_000_000), top: 5, ..Default::default() };
    p.qd.axes = vec!["roll_ratio".into(), "index_load".into(), "hand_balance:35:65".into()];
    p.qd.bins = 4;
    p.qd.seeds = 12;

    let dir = temp_dir("a");
    let (out, s) = qd::run(&ctx, &p, Duration::from_secs(60), &Control::default(), &dir, "a", |_| {}).unwrap();
    assert!(s.done && s.cells == 64);
    assert!(s.filled > 3, "{} cells filled", s.filled);
    assert_eq!(s.filled, s.added as usize);
    assert!(s.offspring > 12, "seeds plus mutations");
    assert!(s.axes[0].auto && !s.axes[2].auto && s.axes[2].lo == 35.0);
    assert!(
        (s.best.as_ref().unwrap().score - out.top[0].score).abs() < 1e-9,
        "run best is the best elite: {:?} vs {:?}",
        s.best,
        out.top[0]
    );
    assert!(s.elites.windows(2).all(|w| w[0].score <= w[1].score), "best first");

    let (slots, metrics, rows) = atlas::read_optima(&std::fs::read(dir.join("elites.bin")).unwrap(), None).unwrap();
    assert_eq!((slots, rows.len()), (30, s.filled));
    let w = Weights::new(&prof, &metrics);
    let mut cells: Vec<usize> = s.elites.iter().map(|e| e.cell).collect();
    cells.sort_unstable();
    cells.dedup();
    assert_eq!(cells.len(), s.filled, "one elite per cell");
    for (i, (e, o)) in s.elites.iter().zip(&rows).enumerate() {
        assert_eq!(e.row, i);
        let chars: String = o.sym.iter().map(|&x| sc.chars[x as usize]).collect();
        assert_eq!(chars, e.chars);
        let st = sc.state_from_sym(o.sym.clone());
        assert!((st.score - e.score).abs() < 1e-9 && (o.score - e.score).abs() < 1e-12);
        assert!((w.score(&o.raw) - o.score).abs() < 1e-4, "raw vector re-scores to the stored score");
        let bins: Vec<usize> = s.axes.iter().zip(&e.desc).map(|(a, &v)| a.bin(v as f64)).collect();
        assert_eq!(bins, e.bins);
        assert_eq!(e.cell, bins[0] + 4 * bins[1] + 16 * bins[2]);
        for &x in &pinned {
            assert_eq!(o.sym[ctx.start.pos[x] as usize], x as u8, "pins respected");
        }
        for a in 0..ctx.movable.len() {
            for b in a + 1..ctx.movable.len() {
                assert!(sc.delta(&st, ctx.movable[a], ctx.movable[b]) >= -1e-6, "elite is a local optimum");
            }
        }
    }

    let dir_b = temp_dir("b");
    let (_, s2) = qd::run(&ctx, &p, Duration::from_secs(60), &Control::default(), &dir_b, "a", |_| {}).unwrap();
    let key = |s: &qd::QdSummary| s.elites.iter().map(|e| (e.cell, e.chars.clone())).collect::<Vec<_>>();
    assert_eq!(key(&s), key(&s2), "evals budget on one thread is deterministic");
    assert_eq!(qd::load(&dir).unwrap().filled, s.filled);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&dir_b);
}
