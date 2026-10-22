//! G07 manufacturability (`geometry/manufacture.rs`, #73): every bundled board is buildable
//! under the default `[build]` limits, a profile's `[build]` section parses, and the geometry
//! search penalizes boards past it.

use gecko_core::geometry::manufacture::BuildLimits;
use gecko_core::geomsearch::{self, GeomParams, GeomSpec};
use gecko_core::store::Store;
use gecko_core::{Geometry, ParametricGeometry, Profile};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
const BOARDS: [&str; 8] = ["ansi-30", "ortho-30", "corne-36", "sweep-34", "lily58", "kyria", "ergodox", "dactyl-manuform-5x6"];

#[test]
fn bundled_boards_are_buildable() {
    for b in BOARDS {
        let g = Geometry::from_toml(&std::fs::read_to_string(format!("{ROOT}/data/geometries/{b}.toml")).unwrap()).unwrap();
        let r = g.manufacture(&BuildLimits::default());
        assert!(r.feasible(), "{b}: {:?}", r.violations.iter().map(|v| (&v.kind, v.value)).collect::<Vec<_>>());
    }
}

#[test]
fn profile_build_section() {
    let p = Profile::from_toml("name = \"t\"\n[build]\nmin_pitch = 0.9\nmax_plate = [10.0, 8.0]\n").unwrap();
    let b = p.build.unwrap();
    assert_eq!(b.min_pitch, 0.9);
    assert_eq!(b.max_plate, [10.0, 8.0]);
    assert_eq!(b.max_key_angle, BuildLimits::default().max_key_angle);
    assert!(Profile::from_toml("name = \"t\"\n[build]\nmin_pich = 0.9\n").is_err());
    assert!(Profile::from_toml("name = \"t\"\n[build]\nmin_pitch = -1.0\n").is_err());
    // Without [build] the profile (and its hash) is unchanged.
    assert!(Profile::from_toml("name = \"t\"\n").unwrap().build.is_none());
}

fn search(build: Option<BuildLimits>) -> geomsearch::GeomResult {
    let st = Store::open(std::path::Path::new(ROOT)).unwrap();
    let p = ParametricGeometry::from_toml(&std::fs::read_to_string(format!("{ROOT}/data/geometries/corne-36.param.toml")).unwrap()).unwrap();
    let board = st.geometry("corne-36.param").unwrap();
    let corpus = st.corpus("english").unwrap();
    let profile = Profile { build, ..st.profile("default").unwrap() };
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
    geomsearch::search(&spec, &Default::default(), |_| {}).unwrap()
}

#[test]
fn geometry_search_applies_build_limits() {
    // Corne meets the default limits: the search is unchanged by them.
    let (free, built) = (search(None), search(Some(BuildLimits::default())));
    assert_eq!(free.best.values, built.best.values);
    assert!(built.feasible && built.best.collisions == 0);
    // No candidate half fits a 5 × 5 u plate: every board is penalized, none is feasible.
    let tight = search(Some(BuildLimits { max_plate: [5.0, 5.0], ..Default::default() }));
    assert!(!tight.feasible);
    assert!(tight.best.collisions >= 2 && tight.best.depth > 1.0, "{} {}", tight.best.collisions, tight.best.depth);
    assert!(tight.best.f > tight.best.score);
}
