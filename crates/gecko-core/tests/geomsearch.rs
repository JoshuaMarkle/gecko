//! G6 geometry search (#54): free-parameter parsing, the approximate 3D keycap collision test,
//! and the bilevel / alternating board + layout search (determinism, collision-free results,
//! run directory). See `context/algorithms/geometry-search.md`.

use std::path::Path;

use gecko_core::geomsearch::{self, GeomParams, GeomSpec, Mode};
use gecko_core::rng::Rng;
use gecko_core::store::Store;
use gecko_core::{Finger, Geometry, Key, ParametricGeometry};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn store() -> Store {
    Store::open(Path::new(ROOT)).unwrap()
}

fn key(pos: [f32; 3], rot: [f32; 3]) -> Key {
    Key {
        pos,
        rot,
        size: [1.0, 1.0],
        finger: Finger::LI,
        row: 0,
        col: 0,
        home: false,
        stretch: false,
        hrm: None,
        effort: 1.0,
        fill: None,
        layer: 0,
        phys: None,
    }
}

fn board(keys: Vec<Key>) -> Geometry {
    Geometry {
        name: "test".into(),
        description: String::new(),
        key: keys,
        comfort: Default::default(),
        comfort_tables: Default::default(),
        layer_keys: Vec::new(),
        layer_modes: Vec::new(),
    }
}

#[test]
fn bundled_boards_are_clear_in_3d() {
    let st = store();
    let mut names: Vec<String> = std::fs::read_dir(Path::new(ROOT).join("data/geometries"))
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter_map(|n| n.strip_suffix(".toml").map(str::to_string))
        .collect();
    names.sort();
    assert!(names.iter().any(|n| n == "dactyl-manuform-5x6"));
    for name in names {
        let g = st.geometry(&name).unwrap();
        assert_eq!(g.collisions_3d(0.0), vec![], "{name}");
    }
    let well = ParametricGeometry::from_toml(
        &std::fs::read_to_string(Path::new(ROOT).join("crates/gecko-core/tests/fixtures/dactyl-well.param.toml")).unwrap(),
    )
    .unwrap()
    .generate()
    .unwrap();
    assert!(!well.is_flat());
    assert_eq!(well.collisions_3d(0.0), vec![]);
}

#[test]
fn flat_keys_collide_in_3d_as_in_2d() {
    let mut rng = Rng::new(7);
    for _ in 0..200 {
        let keys: Vec<Key> = (0..6)
            .map(|_| key([(rng.f64() * 4.0) as f32, (rng.f64() * 3.0) as f32, 0.0], [0.0, 0.0, (rng.f64() * 60.0 - 30.0) as f32]))
            .collect();
        let g = board(keys);
        let flat = g.collisions(0.0);
        let solid = g.collisions_3d(0.0);
        let pairs = |c: &[gecko_core::geometry::collision::Collision]| c.iter().map(|c| (c.a, c.b)).collect::<Vec<_>>();
        assert_eq!(pairs(&flat), pairs(&solid));
        for (a, b) in flat.iter().zip(&solid) {
            // Coplanar caps: the 2D overlap, unless the cap height (0.42 u) is the shallower way out.
            assert!((a.depth - b.depth).abs() < 1e-3 || b.depth <= a.depth, "{a:?} {b:?}");
        }
    }
}

#[test]
fn tilted_keys_collide_by_their_volumes() {
    // Two keys 1 u apart, each tilted 30° toward the other: their tops meet.
    let toward = board(vec![key([0.0, 0.0, 0.0], [0.0, 30.0, 0.0]), key([1.0, 0.0, 0.0], [0.0, -30.0, 0.0])]);
    assert_eq!(toward.collisions_3d(0.0).len(), 1);
    // A column with its keycap tops 1 u apart: a 15° concave well is clear (the tapered caps
    // lean into the gap); a 30° convex column is not (the switch bodies below converge).
    let column = |curve: f64| {
        ParametricGeometry::from_toml(&format!(
            "name = \"c\"\n[[zone]]\nname = \"f\"\nrows = [0, 1, 2]\nrow_curve = {curve}\n[[zone.column]]\nfinger = \"LI\"\n"
        ))
        .unwrap()
        .generate_unchecked()
        .unwrap()
    };
    assert_eq!(column(15.0).collisions_3d(0.0), vec![]);
    assert_eq!(column(-30.0).collisions_3d(0.0).len(), 2);
    // Stacked: one key directly above another.
    let stacked = board(vec![key([0.0, 0.0, 0.0], [0.0; 3]), key([0.2, 0.0, 0.3], [0.0; 3])]);
    assert_eq!(stacked.collisions_3d(0.0).len(), 1);
}

fn corne() -> ParametricGeometry {
    store().param_geometry("corne-36").unwrap()
}

#[test]
fn free_parameter_groups_expand() {
    let p = corne();
    let names = |spec: &str| geomsearch::parse_free(&p, spec).unwrap().into_iter().map(|v| v.name).collect::<Vec<_>>();
    let two = names("2d");
    // 5 staggers + 5 splays + 4 spreads (not the first column) + thumb anchor x/y/rotation + 3 thumb staggers.
    assert_eq!(two.len(), 20, "{two:?}");
    assert!(two.contains(&"fingers.0.stagger".to_string()) && !two.contains(&"fingers.0.spread".to_string()));
    assert!(two.contains(&"thumbs.anchor_x".to_string()) && two.contains(&"thumbs.2.stagger".to_string()));
    let three = names("3d");
    for n in ["fingers.row_curve", "fingers.col_curve", "fingers.3.lift", "split.tent", "split.tilt", "thumbs.tent", "thumbs.col_curve"] {
        assert!(three.contains(&n.to_string()), "{n} in {three:?}");
    }
    assert_eq!(three.len(), two.len() + 2 + 5 + 3 + 4);
    // Explicit names, bounds, and a zone's column field expanding to its columns.
    let v = geomsearch::parse_free(&p, "split.tent=5:25,fingers.splay").unwrap();
    assert_eq!(v.len(), 6);
    assert_eq!((v[0].lo, v[0].hi, v[0].start), (5.0, 25.0, 0.0));
    // Default bounds contain the start value, except the posture priors (H6): slope 0 to −4°,
    // opening 10–20° whatever the file says (Corne's flat halves have no opening).
    for v in geomsearch::parse_free(&p, "3d").unwrap() {
        match v.name.as_str() {
            "split.tilt" => assert_eq!((v.lo, v.hi), geomsearch::POSTURE_TILT),
            "split.angle" => assert_eq!((v.lo, v.hi, v.start), (10.0, 20.0, 0.0)),
            _ => assert!(v.lo < v.hi && v.lo <= v.start && v.start <= v.hi, "{v:?}"),
        }
    }
    for bad in ["", "wiggle", "fingers.9.stagger", "nozone.rotate", "tent=3:1", "split.nope"] {
        assert!(geomsearch::parse_free(&p, bad).is_err(), "{bad}");
    }
    // A board without [split]: the 3D preset skips split fields; asking for one fails.
    let one = ParametricGeometry::from_toml("name = \"one\"\n[[zone]]\nname = \"f\"\nrows = [0, 1, 2]\n[[zone.column]]\nfinger = \"LI\"\n")
        .unwrap();
    assert!(geomsearch::parse_free(&one, "3d").unwrap().iter().all(|v| !v.name.starts_with("split.")));
    assert!(geomsearch::parse_free(&one, "tent").is_err());
}

fn search(mode: Mode, free: &str, threads: usize) -> geomsearch::GeomResult {
    let st = store();
    let p = corne();
    let board = st.geometry("corne-36.param").unwrap();
    let corpus = st.corpus("english").unwrap();
    let profile = st.profile("hand").unwrap();
    let start = st.layout("qwerty", &board.name).unwrap().fit(&board).unwrap();
    let vars = geomsearch::parse_free(&p, free).unwrap();
    let spec = GeomSpec {
        params: &p,
        board: &board,
        corpus: &corpus,
        profile: &profile,
        start: &start,
        pins: "",
        vars: &vars,
        search: GeomParams { mode, evals: Some(60), threads, inner_evals: 3_000, polish_evals: 30_000, rounds: 2, ..Default::default() },
    };
    geomsearch::search(&spec, &Default::default(), |_| {}).unwrap()
}

#[test]
fn bilevel_search_is_reproducible_and_collision_free() {
    let a = search(Mode::Bilevel, "2d", 2);
    let b = search(Mode::Bilevel, "2d", 2);
    assert_eq!(a.best.values, b.best.values);
    assert_eq!((a.best.score, a.best.chars.clone()), (b.best.score, b.best.chars.clone()));
    // The budget stops before a generation that would pass it.
    assert!(a.candidates <= 60 && a.generations >= 4, "{} {}", a.candidates, a.generations);
    assert!(a.search_best <= a.search_start, "{} > {}", a.search_best, a.search_start);
    assert!(a.feasible && a.best.collisions == 0);
    for (v, x) in a.vars.iter().zip(&a.best.values) {
        assert!(v.lo <= *x && *x <= v.hi);
    }
}

#[test]
fn search_3d_and_alternate_mode() {
    let r = search(Mode::Bilevel, "3d", 2);
    assert!(r.feasible && r.search_best <= r.search_start);
    let a = search(Mode::Alternate, "2d", 2);
    let b = search(Mode::Alternate, "2d", 2);
    assert_eq!(a.best.values, b.best.values);
    assert!(a.feasible && a.search_best <= a.search_start);
}

#[test]
fn board_mode_keeps_the_start_layout() {
    let r = search(Mode::Board, "2d", 2);
    assert_eq!(r.best.chars, r.start.chars);
    assert_eq!(r.layout_evals, 0);
    assert!(r.feasible && r.best.score <= r.start.score + 1e-9, "{} > {}", r.best.score, r.start.score);
    assert_eq!(search(Mode::Board, "2d", 2).best.values, r.best.values);
}

#[test]
fn run_directory_holds_a_loadable_board() {
    let st = store();
    let p = corne();
    let board = st.geometry("corne-36.param").unwrap();
    let corpus = st.corpus("english").unwrap();
    let profile = st.profile("hand").unwrap();
    let start = st.layout("qwerty", &board.name).unwrap().fit(&board).unwrap();
    let vars = geomsearch::parse_free(&p, "fingers.stagger").unwrap();
    let spec = GeomSpec {
        params: &p,
        board: &board,
        corpus: &corpus,
        profile: &profile,
        start: &start,
        pins: "",
        vars: &vars,
        search: GeomParams { evals: Some(20), threads: 1, inner_evals: 1_000, polish_evals: 5_000, ..Default::default() },
    };
    let runs = std::env::temp_dir().join(format!("gecko-geom-test-{}", std::process::id()));
    let (res, dir) = geomsearch::run(&runs, &spec, "fingers.stagger", &Default::default(), |_| {}).unwrap();
    for f in ["config.json", "start.param.toml", "progress.jsonl", "best.json", "geometry.param.toml", "geometry.toml"] {
        assert!(dir.join(f).is_file(), "{f}");
    }
    let keys = Geometry::from_toml(&std::fs::read_to_string(dir.join("geometry.toml")).unwrap()).unwrap();
    let params = ParametricGeometry::from_toml(&std::fs::read_to_string(dir.join("geometry.param.toml")).unwrap()).unwrap();
    assert_eq!(keys.key.len(), board.key.len());
    let regenerated = params.generate().unwrap();
    assert_eq!(regenerated.hash(), keys.hash(), "the two saved forms describe the same board");
    // The saved board scores the best layout as the run reported.
    let layout = gecko_core::Layout::from_flat("best", &keys, res.best.chars.chars().collect()).unwrap();
    let sc = gecko_core::Scorer::new(&keys, &corpus, &profile, &layout).unwrap();
    // A fresh scorer numbers symbols in another order, so f32 sums differ in the last bits.
    let fresh = sc.identity().score;
    assert!((fresh - res.best.score).abs() < 1e-5 * res.best.score.abs().max(1.0), "{fresh} vs {}", res.best.score);
    let _ = std::fs::remove_dir_all(&runs);
}

#[test]
fn progress_carries_the_reported_board() {
    let st = store();
    let p = corne();
    let board = st.geometry("corne-36.param").unwrap();
    let corpus = st.corpus("english").unwrap();
    let profile = st.profile("hand").unwrap();
    let start = st.layout("qwerty", &board.name).unwrap().fit(&board).unwrap();
    let vars = geomsearch::parse_free(&p, "fingers.stagger").unwrap();
    let spec = GeomSpec {
        params: &p,
        board: &board,
        corpus: &corpus,
        profile: &profile,
        start: &start,
        pins: "",
        vars: &vars,
        search: GeomParams { evals: Some(20), threads: 1, inner_evals: 1_000, polish_evals: 5_000, ..Default::default() },
    };
    let mut seen = Vec::new();
    let res = geomsearch::search(&spec, &Default::default(), |pg| seen.push(pg.clone())).unwrap();
    assert!(!seen.is_empty());
    for pg in &seen {
        assert_eq!(pg.values.len(), vars.len());
        assert_eq!(pg.chars.chars().count(), board.key.len());
    }
    // The last event reports the board the search chose (polished afterwards).
    assert_eq!(seen.last().unwrap().values, res.best.values);
    // Restarts left without budget are not reported (their best would be infinite).
    assert!(res.cma_runs.iter().all(|r| r.evals > 0 && r.best.is_finite()));
}

#[test]
fn paused_time_does_not_use_the_budget() {
    let st = store();
    let p = corne();
    let board = st.geometry("corne-36.param").unwrap();
    let corpus = st.corpus("english").unwrap();
    let profile = st.profile("hand").unwrap();
    let start = st.layout("qwerty", &board.name).unwrap().fit(&board).unwrap();
    let vars = geomsearch::parse_free(&p, "fingers.stagger").unwrap();
    let spec = GeomSpec {
        params: &p,
        board: &board,
        corpus: &corpus,
        profile: &profile,
        start: &start,
        pins: "",
        vars: &vars,
        search: GeomParams { time_ms: Some(600), threads: 1, inner_evals: 500, ..Default::default() },
    };
    let control = gecko_core::optimize::Control::default();
    let t0 = std::time::Instant::now();
    let res = std::thread::scope(|s| {
        s.spawn(|| {
            std::thread::sleep(std::time::Duration::from_millis(150));
            control.set_paused(true);
            std::thread::sleep(std::time::Duration::from_millis(800));
            control.set_paused(false);
        });
        geomsearch::search(&spec, &control, |_| {}).unwrap()
    });
    let wall = t0.elapsed().as_millis() as u64;
    assert!(wall >= 900, "the pause held the search ({wall} ms)");
    assert!(res.elapsed_ms + 600 <= wall, "reported {} ms of {wall} ms wall time", res.elapsed_ms);
}

#[test]
fn run_boards_load_by_id() {
    let mut st = store();
    st.runs = std::env::temp_dir().join(format!("gecko-geom-runboard-{}", std::process::id()));
    let p = corne();
    let board = st.geometry("corne-36.param").unwrap();
    let corpus = st.corpus("english").unwrap();
    let profile = st.profile("hand").unwrap();
    let start = st.layout("qwerty", &board.name).unwrap().fit(&board).unwrap();
    let vars = geomsearch::parse_free(&p, "fingers.splay").unwrap();
    let spec = GeomSpec {
        params: &p,
        board: &board,
        corpus: &corpus,
        profile: &profile,
        start: &start,
        pins: "",
        vars: &vars,
        search: GeomParams { evals: Some(10), threads: 1, inner_evals: 500, polish_evals: 2_000, ..Default::default() },
    };
    let (res, _) = geomsearch::run(&st.runs, &spec, "fingers.splay", &Default::default(), |_| {}).unwrap();
    let id = format!("run:{}", res.id);
    let g = st.geometry(&id).unwrap();
    assert_eq!(g.name, geomsearch::best_name(&p.name, &res.id));
    let (params, keys) = st.param_board(&id).unwrap();
    assert_eq!(keys.hash(), g.hash());
    assert_eq!(params.name, g.name);
    let listed = gecko_core::run::list(&st.runs);
    assert_eq!((listed[0].kind.as_str(), listed[0].geometry.as_str()), ("geometry", g.name.as_str()));
    assert!(st.geometry("run:../x").is_err() && st.param_geometry("run:").is_err());
    let _ = std::fs::remove_dir_all(&st.runs);
}
