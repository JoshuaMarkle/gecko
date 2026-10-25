//! Parametric boards must preserve the explicit board's slots, metadata, and scores.

use std::path::{Path, PathBuf};

use gecko_core::geometry::parametric::{Column, ParametricGeometry, Split, Zone};
use gecko_core::report::breakdown;
use gecko_core::store::Store;
use gecko_core::{Finger, Geometry, Scorer};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn parameters(name: &str) -> ParametricGeometry {
    ParametricGeometry::from_toml(&std::fs::read_to_string(format!("{ROOT}/data/geometries/{name}.param.toml")).unwrap()).unwrap()
}

fn close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 1e-5, "{actual} != {expected}");
}

fn center(geometry: &Geometry, i: usize) -> [f32; 2] {
    let key = &geometry.key[i];
    [key.pos[0] + key.size[0] / 2.0, key.pos[1] + key.size[1] / 2.0]
}

#[test]
fn bundled_parameters_reproduce_every_key_and_every_reference_score_exactly() {
    let store = Store::open(Path::new(ROOT)).unwrap();
    let corpus = store.corpus("english").unwrap();
    for name in ["ortho-30", "corne-36"] {
        let generated = parameters(name).generate().unwrap();
        let explicit = store.geometry(name).unwrap();
        // Includes every pose, slot order, metadata flag, thumb fill and description.
        assert_eq!(serde_json::to_value(&generated).unwrap(), serde_json::to_value(&explicit).unwrap(), "{name}");
        assert_eq!(generated.hash(), explicit.hash(), "{name}");
        assert_eq!(store.geometry(&format!("{name}.param")).unwrap().hash(), explicit.hash());
        for profile in store.list("profiles") {
            let profile = store.profile(&profile).unwrap();
            for layout in store.list("layouts") {
                let layout = store.layout(&layout, name).unwrap().fit(&explicit).unwrap();
                let a = Scorer::new(&explicit, &corpus, &profile, &layout).unwrap();
                let b = Scorer::new(&generated, &corpus, &profile, &layout).unwrap();
                let (a, b) = (breakdown(&a, &a.identity()), breakdown(&b, &b.identity()));
                assert_eq!(a.score, b.score, "{name} / {} / {}", profile.name, layout.name);
                for (a, b) in a.metrics.iter().zip(&b.metrics) {
                    assert_eq!((a.id, a.value, a.weighted), (b.id, b.value, b.weighted));
                }
            }
        }
        // Parameter serialization and resolved explicit export both keep the same content.
        let saved = toml::to_string(&parameters(name)).unwrap();
        assert_eq!(ParametricGeometry::from_toml(&saved).unwrap().generate().unwrap().hash(), explicit.hash());
        assert_eq!(Geometry::from_toml(&generated.to_toml()).unwrap().hash(), explicit.hash());
    }
}

fn tiny() -> ParametricGeometry {
    ParametricGeometry {
        name: "tiny".into(),
        description: String::new(),
        comfort: Default::default(),
        split: None,
        zone: vec![Zone {
            name: "fingers".into(),
            anchor: [0.5, 0.5],
            rotate: 0.0,
            mirror: false,
            rows: vec![0, 1, 2],
            row_pitch: 1.0,
            home_row: 1,
            home_stretch: false,
            col_start: 0,
            arc_radius: None,
            row_curve: 0.0,
            col_curve: 0.0,
            center_col: 0.0,
            tent: 0.0,
            tilt: 0.0,
            lift: 0.0,
            column: vec![Column {
                finger: Finger::LP,
                spread: 1.0,
                stagger: 0.0,
                splay: 0.0,
                lift: 0.0,
                size: [1.0, 1.0],
                stretch: false,
                effort: vec![1.0; 3],
                fill: None,
                mirror_fill: None,
            }],
        }],
    }
}

#[test]
fn stagger_and_column_splay_rotate_around_the_home_key() {
    let mut p = tiny();
    p.zone[0].column[0].stagger = 1.0;
    p.zone[0].column[0].splay = 90.0;
    let g = p.generate().unwrap();
    // Home moves down by stagger, then stays fixed during the column rotation.
    assert_eq!(g.key[1].pos, [0.0, 2.0, 0.0]);
    assert!(g.key[1].home);
    for (key, [x, y]) in g.key.iter().zip([[1.0, 2.0], [0.0, 2.0], [-1.0, 2.0]]) {
        close(key.pos[0], x);
        close(key.pos[1], y);
        assert_eq!(key.rot, [0.0, 0.0, 90.0]);
    }
}

#[test]
fn split_angle_pivot_and_spread_mirror_centers_and_fingers() {
    let mut p = tiny();
    p.zone[0].mirror = true;
    p.split = Some(Split { axis: 2.0, spread: 1.0, angle: 90.0, pivot: [0.5, 1.5], tent: 0.0, tilt: 0.0, lift: 0.0 });
    let g = p.generate().unwrap();
    assert_eq!(g.row_lengths(), [2, 2, 2]);
    // Top center rotates -45° about the home center, then the right hand reflects
    // across x=(2*axis+spread)/2. Home remains at the split pivot on each half.
    let top = center(&g, 0);
    close(top[0], 0.5 - 0.5f32.sqrt());
    close(top[1], 1.5 - 0.5f32.sqrt());
    assert_eq!(center(&g, 2), [0.5, 1.5]);
    assert_eq!(center(&g, 3), [4.5, 1.5]);
    for row in 0..3 {
        let (left, right) = (row * 2, row * 2 + 1);
        close(center(&g, left)[0] + center(&g, right)[0], 5.0);
        close(center(&g, left)[1], center(&g, right)[1]);
        assert_eq!((g.key[left].finger, g.key[right].finger), (Finger::LP, Finger::RP));
        assert_eq!((g.key[left].rot[2], g.key[right].rot[2]), (-45.0, 45.0));
    }
}

#[test]
fn thumb_arc_uses_arc_length_and_zone_pose() {
    let mut p = tiny();
    let zone = &mut p.zone[0];
    zone.name = "thumbs".into();
    zone.rows = vec![3];
    zone.anchor = [2.0, 3.0];
    zone.rotate = 90.0;
    zone.arc_radius = Some(1.0);
    zone.column[0].finger = Finger::LT;
    zone.column[0].effort = vec![1.0];
    zone.column[0].spread = std::f64::consts::FRAC_PI_2;
    zone.column = vec![zone.column[0].clone(); 3];
    let g = p.generate().unwrap();
    // Unit-radius circle at arc lengths 0, π/2, π gives (0,0),(1,1),(0,2),
    // then rotating the whole zone 90° about its anchor gives these centers.
    for (i, [x, y]) in [[2.0, 3.0], [1.0, 4.0], [0.0, 3.0]].into_iter().enumerate() {
        close(center(&g, i)[0], x);
        close(center(&g, i)[1], y);
        close(g.key[i].rot[2], 90.0 + i as f32 * 90.0);
        assert!(!g.key[i].home);
    }
}

#[test]
fn invalid_parameters_fail_instead_of_producing_bad_poses_or_slots() {
    let mut cases = Vec::new();
    let p = tiny();
    let mut bad = p.clone();
    bad.zone[0].row_pitch = f64::NAN;
    cases.push(bad);
    let mut bad = p.clone();
    bad.zone[0].arc_radius = Some(0.0);
    cases.push(bad);
    let mut bad = p.clone();
    bad.zone[0].column[0].size[0] = -1.0;
    cases.push(bad);
    let mut bad = p.clone();
    bad.zone[0].column[0].size[0] = 1e-300;
    cases.push(bad);
    let mut bad = p.clone();
    bad.zone[0].column[0].effort.pop();
    cases.push(bad);
    let mut bad = p.clone();
    bad.zone[0].mirror = true;
    cases.push(bad);
    let mut bad = p.clone();
    bad.zone.push(bad.zone[0].clone());
    bad.zone[1].name = "overlapping".into();
    cases.push(bad);
    let mut bad = p.clone();
    bad.zone[0].rows = vec![1, 0, 2];
    cases.push(bad);
    let mut bad = p.clone();
    bad.zone[0].anchor[0] = f64::MAX;
    cases.push(bad);
    let mut bad = p.clone();
    bad.zone[0].column = vec![bad.zone[0].column[0].clone(); 50];
    cases.push(bad);
    for case in cases {
        assert!(case.generate().is_err(), "accepted invalid parameters: {case:?}");
    }
    let text = toml::to_string(&p).unwrap().replace("row_pitch = 1.0", "row_pich = 1.0");
    assert!(ParametricGeometry::from_toml(&text).is_err(), "unknown fields must not silently ignore a parameter");
}

struct Temp(PathBuf);
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn store_discovers_params_resolves_comfort_and_preserves_explicit_precedence() {
    let temp = Temp(std::env::temp_dir().join(format!(
        "gecko-parametric-{}-{}",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    )));
    let root = &temp.0;
    std::fs::create_dir_all(root.join("data/geometries")).unwrap();
    std::fs::create_dir_all(root.join("data/comfort")).unwrap();
    std::fs::write(root.join("gecko.toml"), "").unwrap();
    let mut p = tiny();
    p.comfort.insert("engram_formula".into(), "tiny".into());
    let path = root.join("data/geometries/tiny.param.toml");
    std::fs::write(&path, toml::to_string(&p).unwrap()).unwrap();
    std::fs::write(root.join("data/comfort/tiny.csv"), "a_row,a_col,b_row,b_col,score\n0,0,1,0,0.25\n").unwrap();
    let store = Store::open(root).unwrap();
    assert_eq!(store.list("geometries"), ["tiny"]);
    let generated = store.geometry("tiny").unwrap();
    assert_eq!(generated.comfort_tables["engram_formula"][1], 0.25);
    assert_eq!(store.geometry(path.to_str().unwrap()).unwrap().hash(), generated.hash());
    let mut explicit = p.generate().unwrap();
    explicit.description = "explicit wins".into();
    std::fs::write(root.join("data/geometries/tiny.toml"), explicit.to_toml()).unwrap();
    assert_eq!(store.list("geometries"), ["tiny"]);
    assert_eq!(store.geometry("tiny").unwrap().description, "explicit wins");
    assert_eq!(store.geometry("tiny.param").unwrap().hash(), generated.hash());
}
