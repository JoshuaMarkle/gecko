//! Hand model validation (#57, `context/hand-model.md`): home keys cheapest, mirrored hands,
//! size ordering, unreachable keys, and the board ranking ergonomic boards should win.

use std::path::Path;

use gecko_core::hand::{costs, costs_with, natural_tip, solve, split, Body, HandSize, HandSpec, AVERAGE, BODY};
use gecko_core::metrics::{hand_spread, stretch_at, SPREAD};
use gecko_core::report::breakdown;
use gecko_core::store::Store;
use gecko_core::{Geometry, Profile, Scorer};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn store() -> Store {
    Store::open(Path::new(ROOT)).unwrap()
}

fn mean(v: &[f32]) -> f32 {
    v.iter().sum::<f32>() / v.len() as f32
}

#[test]
fn ik_reaches_the_natural_tip_and_flags_far_targets() {
    let l = AVERAGE.segments(2);
    let [fwd, drop] = natural_tip(&l);
    let c = solve(fwd, -drop, &l, 0.7, 0.2);
    assert!(c.miss < 0.5, "natural tip reachable: {c:?}");
    assert!((c.angles[0] - 0.2).abs() < 0.05 && (c.angles[1] - 0.95).abs() < 0.05, "natural curl recovered: {c:?}");
    let total: f64 = l.iter().sum();
    let far = solve(total + 12.0, 0.0, &l, 0.7, 0.2);
    assert!((far.miss - 12.0).abs() < 1e-9 && far.angles[1] == 0.0, "straight finger, 12 mm short: {far:?}");
}

#[test]
fn home_keys_are_cheapest_per_finger() {
    let s = store();
    for name in ["ansi-30", "ortho-30", "corne-36", "sweep-34", "kyria", "dactyl-manuform-5x6"] {
        let g = s.geometry(name).unwrap();
        let c = costs(&g, &AVERAGE);
        assert_eq!(c.placed, [true, true], "{name}");
        for (i, k) in g.key.iter().enumerate().filter(|(_, k)| k.home && !k.stretch && !k.finger.is_thumb()) {
            assert!(c.reach[i] < 0.3, "{name}: home key {i} costs {}", c.reach[i]);
            for (j, o) in g.key.iter().enumerate().filter(|(j, o)| o.finger == k.finger && *j != i) {
                // The short pinky rests a little below its home key, so a bottom-row key can tie.
                assert!(c.reach[j] >= c.reach[i] - 0.2, "{name}: {:?} key {j} (row {} col {}) cheaper than home", o.finger, o.row, o.col);
            }
        }
    }
}

#[test]
fn mirrored_boards_give_mirrored_costs() {
    let s = store();
    for name in ["ortho-30", "corne-36"] {
        let g = s.geometry(name).unwrap();
        let c = costs(&g, &AVERAGE);
        let xs = g.key.iter().map(|k| k.pos[0] + k.size[0] / 2.0);
        let cx = (xs.clone().fold(f32::MAX, f32::min) + xs.fold(f32::MIN, f32::max)) / 2.0;
        for (i, k) in g.key.iter().enumerate() {
            let x = k.pos[0] + k.size[0] / 2.0;
            let j =
                g.key.iter().position(|o| ((o.pos[0] + o.size[0] / 2.0) + x - 2.0 * cx).abs() < 1e-3 && (o.pos[1] - k.pos[1]).abs() < 1e-3);
            let j = j.unwrap_or_else(|| panic!("{name}: no mirror for key {i}"));
            assert!((c.reach[i] - c.reach[j]).abs() < 1e-4 && (c.align[i] - c.align[j]).abs() < 1e-4, "{name}: {i} vs {j}");
        }
    }
}

#[test]
fn smaller_hands_reach_less() {
    let g = store().geometry("ansi-30").unwrap();
    let r: Vec<f32> = [HandSize::Small, HandSize::Average, HandSize::Large]
        .iter()
        .map(|s| mean(&costs(&g, &HandSpec { size: *s, dims: None, body: None }.dims()).reach))
        .collect();
    assert!(r[0] > r[1] && r[1] > r[2], "{r:?}");
}

#[test]
fn a_key_moved_away_is_unreachable_and_costly() {
    let mut g = store().geometry("ortho-30").unwrap();
    let i = g.key.iter().position(|k| k.row == 0 && k.col == 2).unwrap();
    let before = costs(&g, &AVERAGE);
    g.key[i].pos[1] -= 2.0;
    let after = costs(&g, &AVERAGE);
    assert_eq!(before.miss[i], 0.0);
    assert!(after.miss[i] > 15.0 && after.reach[i] > before.reach[i] + 5.0, "{} mm, {}", after.miss[i], after.reach[i]);
}

/// Validation (`research/sources/hand-models.md`): on the same layout, ergonomic boards
/// cost less reach than flat ones, and the Dactyl keywell aligns presses best.
#[test]
fn ergonomic_boards_beat_flat_ones() {
    let s = store();
    let corpus = s.corpus("english").unwrap();
    let profile = s.profile("default").unwrap();
    let value = |board: &str, id: &str| {
        let g: Geometry = s.geometry(board).unwrap();
        let l = s.layout("qwerty", board).unwrap().fit(&g).unwrap();
        let sc = Scorer::new(&g, &corpus, &profile, &l).unwrap();
        breakdown(&sc, &sc.identity()).metrics.iter().find(|m| m.id == id).unwrap().value
    };
    let (ansi, ortho, corne, dactyl) =
        (value("ansi-30", "reach"), value("ortho-30", "reach"), value("corne-36", "reach"), value("dactyl-manuform-5x6", "reach"));
    assert!(dactyl < corne && corne < ortho && corne < ansi, "reach ansi {ansi} ortho {ortho} corne {corne} dactyl {dactyl}");
    let (corne, dactyl) = (value("corne-36", "press_angle"), value("dactyl-manuform-5x6", "press_angle"));
    assert!(dactyl < 0.6 * corne, "press_angle corne {corne} dactyl {dactyl}");
}

#[test]
fn hand_profile_round_trips_and_scales_stretch() {
    let mut p = Profile::from_toml(&std::fs::read_to_string(format!("{ROOT}/data/profiles/default.toml")).unwrap()).unwrap();
    let base = p.table_hash();
    assert_eq!(hand_spread(&p), SPREAD);
    p.hand = Some(HandSpec { size: HandSize::Average, dims: None, body: None });
    assert_eq!(hand_spread(&p), SPREAD, "the average hand keeps stretch unchanged");
    assert_ne!(p.table_hash(), base, "a hand shapes the tables");
    p.hand = Some(HandSpec { size: HandSize::Large, dims: None, body: None });
    let back: Profile = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
    assert_eq!(back, p);
    let g = store().geometry("ansi-30").unwrap();
    let pts = g.metric_points();
    let at = |r: i8, c: i8| g.key.iter().position(|k| k.row == r && k.col == c).unwrap();
    let (i, j) = (at(1, 2), at(1, 4));
    assert!(stretch_at(&g, &pts, hand_spread(&p), i, j) < stretch_at(&g, &pts, SPREAD, i, j), "a large hand stretches less");
    let bad = "name = \"x\"\n[hand]\nsize = \"average\"\n[hand.dims]\nthumb = [0, 1, 1]\nindex = [1, 1, 1]\nmiddle = [1, 1, 1]\nring = [1, 1, 1]\npinky = [1, 1, 1]\npalm_width = 80\npalm_length = 100\nspan = 200\nreach = 40\n";
    assert!(Profile::from_toml(bad).is_err());
}

/// The 4-DoF thumb: the keys beside its rest point are cheap, a key pushed out of the cluster is
/// out of reach, and keywell thumb keys tilted toward the thumb press better than flat ones.
#[test]
fn thumbs_reach_their_cluster_and_tilted_keys_press_better() {
    let s = store();
    let thumbs = |g: &Geometry| -> Vec<usize> { (0..g.len()).filter(|&i| g.key[i].phys.is_none() && g.key[i].finger.is_thumb()).collect() };
    let mut g = s.geometry("corne-36").unwrap();
    let c = costs(&g, &AVERAGE);
    let t = thumbs(&g);
    let best = t.iter().map(|&i| c.reach[i]).fold(f32::MAX, f32::min);
    assert!(best < 0.3, "a thumb key near rest: {best}");
    assert!(
        t.iter().all(|&i| c.align[i] > 0.1 && c.align[i] < 0.6),
        "flat thumb keys ~25° off: {:?}",
        t.iter().map(|&i| c.align[i]).collect::<Vec<_>>()
    );
    let i = t.iter().copied().min_by(|&a, &b| c.reach[a].total_cmp(&c.reach[b])).unwrap();
    g.key[i].pos[1] += 3.0;
    let far = costs(&g, &AVERAGE);
    assert!(far.miss[i] > 10.0 && far.reach[i] > c.reach[i] + 2.0, "{} mm, {}", far.miss[i], far.reach[i]);
    let d = s.geometry("dactyl-manuform-5x6").unwrap();
    let dc = costs(&d, &AVERAGE);
    let mean_align = |c: &[f32], ks: &[usize]| ks.iter().map(|&i| c[i]).sum::<f32>() / ks.len() as f32;
    let (flat, well) = (mean_align(&c.align, &t), mean_align(&dc.align, &thumbs(&d)));
    assert!(well < 0.5 * flat, "thumb press: dactyl {well} vs corne {flat}");
}

/// wrist (`research/sources/split-keyboard-ergonomics.md`): a flat one-piece board deviates the wrist
/// ≈ 12° ulnar with the palms flat; split halves remove the deviation and a tent the pronation.
#[test]
fn wrist_posture_ranks_split_and_tented_boards() {
    let s = store();
    let ansi = s.geometry("ansi-30").unwrap();
    assert!(!split(&ansi) && !split(&s.geometry("ortho-30").unwrap()));
    let c = costs(&ansi, &AVERAGE);
    for p in c.posture {
        assert!((9.0..15.0).contains(&p.deviation), "ANSI ulnar deviation {p:?}");
        assert!((80.0..95.0).contains(&p.pronation), "palms flat {p:?}");
    }
    let mean = |c: &[f32]| mean(c);
    let corne = s.geometry("corne-36").unwrap();
    let dactyl = s.geometry("dactyl-manuform-5x6").unwrap();
    assert!(split(&corne) && split(&dactyl));
    let (cc, dc) = (costs(&corne, &AVERAGE), costs(&dactyl, &AVERAGE));
    assert!(cc.posture[1].deviation.abs() < 1.0, "{:?}", cc.posture);
    assert!(dc.posture[1].pronation < cc.posture[1].pronation - 10.0, "tent: {:?} vs {:?}", dc.posture, cc.posture);
    let (a, co, da) = (mean(&c.wrist), mean(&cc.wrist), mean(&dc.wrist));
    assert!(da < co && co < a, "wrist ansi {a} corne {co} dactyl {da}");
    // Elbows farther apart angle the forearms inward more: more ulnar deviation on a flat board.
    let wide = costs_with(&ansi, &AVERAGE, &Body { elbow_width: BODY.elbow_width + 60.0, ..BODY });
    assert!(wide.posture[1].deviation > c.posture[1].deviation + 5.0);
    // A key the planted hand reaches sideways for adds deviation.
    let mut g = ansi.clone();
    let i = g.key.iter().position(|k| k.row == 0 && k.col == 9).unwrap();
    g.key[i].pos[0] += 2.5;
    let out = costs(&g, &AVERAGE);
    assert!(out.miss[i] > 3.0 && out.wrist[i] > c.wrist[i] + 0.1, "{} mm: {} vs {}", out.miss[i], out.wrist[i], c.wrist[i]);
    let bad = "name = \"x\"\n[hand]\nsize = \"average\"\n[hand.body]\nelbow_width = 50\nforearm = 250\n";
    assert!(Profile::from_toml(bad).is_err());
}
