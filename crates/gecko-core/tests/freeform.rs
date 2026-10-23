//! I1 free-form search (#70): key-pose variables, mirroring on a symmetric split, and a short
//! search over them. See `context/algorithms/geometry-search.md#free-form-keys-i1`.

use std::path::Path;

use gecko_core::geomsearch::{self, GeomParams, GeomSpec};
use gecko_core::store::Store;
use gecko_core::{Geometry, Hand, ParametricGeometry};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn store() -> Store {
    Store::open(Path::new(ROOT)).unwrap()
}

fn corne() -> ParametricGeometry {
    store().param_geometry("corne-36").unwrap()
}

#[test]
fn key_terms_expand_to_left_half_poses() {
    let p = corne();
    let g = p.generate_unchecked().unwrap();
    let left = g.key.iter().filter(|k| k.phys.is_none() && k.hand() == Hand::L).count();
    let keys = geomsearch::parse_free(&p, "keys").unwrap();
    assert_eq!(keys.len(), 6 * left, "mirrored split: only the left half is free");
    assert!(geomsearch::is_freeform(&keys));
    assert_eq!(geomsearch::parse_free(&p, "keys2d").unwrap().len(), 3 * left);
    // Bounds are relative to each key's start value; a later range overrides the default.
    let v = geomsearch::parse_free(&p, "keys2d,keys.x=-0.2:0.1").unwrap();
    for v in v.iter().filter(|v| v.name.ends_with(".x")) {
        assert!((v.lo - (v.start - 0.2)).abs() < 1e-9 && (v.hi - (v.start + 0.1)).abs() < 1e-9, "{v:?}");
    }
    for v in v.iter().filter(|v| v.name.ends_with(".rz")) {
        assert!((v.hi - v.lo - 30.0).abs() < 1e-9, "default ±15°: {v:?}");
    }
    for bad in ["keys,stagger", "keys.w", "keys=1:0"] {
        assert!(geomsearch::parse_free(&p, bad).is_err(), "{bad}");
    }
    assert!(!geomsearch::is_freeform(&geomsearch::parse_free(&p, "2d").unwrap()));
}

#[test]
fn key_values_rebuild_the_board_and_mirror_it() {
    let p = corne();
    let start = p.generate_unchecked().unwrap();
    let vars = geomsearch::parse_free(&p, "keys").unwrap();
    let x: Vec<f64> = vars.iter().map(|v| v.start).collect();
    let g = geomsearch::board_at(&p, &vars, &x, true).unwrap();
    for (a, b) in g.key.iter().zip(&start.key) {
        assert_eq!((a.pos, a.rot), (b.pos, b.rot), "start values give the start board");
    }
    // Move one left key: its right partner mirrors x and the y/z rotations, copies the rest.
    let i = vars.iter().position(|v| v.name.ends_with(".x")).unwrap();
    let k: usize = vars[i].name.split('.').nth(1).unwrap().parse().unwrap();
    let mut x2 = x.clone();
    x2[i] += 0.2; // x
    x2[i + 1] += 0.1; // y
    x2[i + 5] += 5.0; // rz
    let g2 = geomsearch::board_at(&p, &vars, &x2, true).unwrap();
    let moved: Vec<usize> = (0..g2.len()).filter(|&j| g2.key[j].pos != start.key[j].pos).collect();
    assert_eq!(moved.len(), 2, "{moved:?}");
    let r = moved.into_iter().find(|&j| j != k).unwrap();
    let (a, b, a0, b0) = (&g2.key[k], &g2.key[r], &start.key[k], &start.key[r]);
    assert!(((a.pos[0] - a0.pos[0]) + (b.pos[0] - b0.pos[0])).abs() < 1e-5, "x mirrors");
    assert!(((a.pos[1] - a0.pos[1]) - (b.pos[1] - b0.pos[1])).abs() < 1e-5, "y copies");
    assert!(((a.rot[2] - a0.rot[2]) + (b.rot[2] - b0.rot[2])).abs() < 1e-4, "rz mirrors");
}

#[test]
fn freeform_search_is_reproducible_and_writes_keys_only() {
    let st = store();
    let p = corne();
    let board = st.geometry("corne-36.param").unwrap();
    let corpus = st.corpus("english").unwrap();
    let profile = st.profile("hand").unwrap();
    let start = st.layout("qwerty", &board.name).unwrap().fit(&board).unwrap();
    let vars = geomsearch::parse_free(&p, "keys2d").unwrap();
    let spec = GeomSpec {
        params: &p,
        board: &board,
        corpus: &corpus,
        profile: &profile,
        start: &start,
        pins: "",
        vars: &vars,
        search: GeomParams { evals: Some(40), threads: 2, inner_evals: 2_000, polish_evals: 20_000, ..Default::default() },
    };
    let a = geomsearch::search(&spec, &Default::default(), |_| {}).unwrap();
    let b = geomsearch::search(&spec, &Default::default(), |_| {}).unwrap();
    assert_eq!(a.best.values, b.best.values);
    assert_eq!(a.best.chars, b.best.chars);
    assert!(a.feasible && a.best.collisions == 0);
    assert!(a.search_best <= a.search_start, "{} > {}", a.search_best, a.search_start);
    assert!(a.best.rough >= 0.0 && a.start.rough == 0.0);
    for (v, x) in a.vars.iter().zip(&a.best.values) {
        assert!(v.lo - 1e-6 <= *x && *x <= v.hi + 1e-6, "{v:?} {x}");
    }

    let runs = std::env::temp_dir().join(format!("gecko-freeform-test-{}", std::process::id()));
    let (res, dir) = geomsearch::run(&runs, &spec, "keys2d", &Default::default(), |_| {}).unwrap();
    assert!(dir.join("geometry.toml").is_file() && !dir.join("geometry.param.toml").exists());
    let keys = Geometry::from_toml(&std::fs::read_to_string(dir.join("geometry.toml")).unwrap()).unwrap();
    let expect = geomsearch::board_at(&p, &vars, &res.best.values, false).unwrap();
    for (a, b) in keys.key.iter().zip(&expect.key) {
        assert_eq!((a.pos, a.rot), (b.pos, b.rot));
    }
    let _ = std::fs::remove_dir_all(&runs);
}
