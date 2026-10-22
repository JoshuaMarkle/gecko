//! G4 derived costs (`derive.rs`, #42): board files may leave out `effort` and `stretch`.
//! Derived stretch flags equal the hand-entered ones on every bundled board, derived efforts
//! track the hand-entered ones, and with nothing hand-entered a column-staggered board beats
//! a row-staggered one for the same (non-QWERTY) layouts. See `context/testing.md`.

use gecko_core::{Corpus, Geometry, Layout, ParametricGeometry, Profile, Scorer};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
const BOARDS: [&str; 8] = ["ansi-30", "ortho-30", "corne-36", "sweep-34", "lily58", "kyria", "ergodox", "dactyl-manuform-5x6"];

fn read(p: &str) -> String {
    std::fs::read_to_string(format!("{ROOT}/{p}")).unwrap()
}

/// A board file with every `effort = …` and `stretch = …` line removed.
fn bare(text: &str) -> String {
    text.lines().filter(|l| !l.starts_with("effort") && !l.starts_with("stretch")).collect::<Vec<_>>().join("\n")
}

fn pearson(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len() as f32;
    let (ma, mb) = (a.iter().sum::<f32>() / n, b.iter().sum::<f32>() / n);
    let cov: f32 = a.iter().zip(b).map(|(x, y)| (x - ma) * (y - mb)).sum();
    let va: f32 = a.iter().map(|x| (x - ma).powi(2)).sum();
    let vb: f32 = b.iter().map(|y| (y - mb).powi(2)).sum();
    cov / (va * vb).sqrt()
}

#[test]
fn derived_stretch_matches_bundled_boards() {
    let mut mismatches = Vec::new();
    for name in BOARDS {
        let text = read(&format!("data/geometries/{name}.toml"));
        let given = Geometry::from_toml(&text).unwrap();
        let derived = Geometry::from_toml(&bare(&text)).unwrap();
        for (i, (a, b)) in given.key.iter().zip(&derived.key).enumerate() {
            if a.stretch != b.stretch {
                mismatches.push((name, i));
            }
        }
    }
    // ANSI labels its whole centre column stretch, but `N` sits 0.5 u inward of `J`, exactly
    // as `V` (not stretch) sits from `F`: geometry cannot tell them apart.
    assert_eq!(mismatches, vec![("ansi-30", 25)]);
}

#[test]
fn derived_efforts_track_hand_entered_ones() {
    for name in BOARDS {
        let text = read(&format!("data/geometries/{name}.toml"));
        let (given, derived) = (Geometry::from_toml(&text).unwrap(), Geometry::from_toml(&bare(&text)).unwrap());
        let fingers: Vec<usize> = (0..given.len()).filter(|&i| !given.key[i].finger.is_thumb()).collect();
        let pick = |g: &Geometry| fingers.iter().map(|&i| g.key[i].effort).collect::<Vec<_>>();
        let r = pearson(&pick(&given), &pick(&derived));
        assert!(r > 0.8, "{name}: r = {r}");
        for k in derived.key.iter().filter(|k| k.home && !k.stretch && !k.finger.is_thumb()) {
            assert_eq!(k.effort, gecko_core::geometry::derive::HOME_EFFORT[k.finger.ord() as usize], "{name}");
        }
        // The cheapest thumb key of each hand is its rest key.
        for t in [gecko_core::Finger::LT, gecko_core::Finger::RT] {
            let low = derived.key.iter().filter(|k| k.finger == t).map(|k| k.effort).fold(f32::INFINITY, f32::min);
            assert!(low.is_infinite() || (low - 1.0).abs() < 1e-6, "{name} {t:?}: {low}");
        }
    }
    // An explicit effort wins over the derived one.
    let mut g = Geometry::from_toml(&bare(&read("data/geometries/ansi-30.toml"))).unwrap();
    let text = g.to_toml().replacen("effort = 1.6", "effort = 9.0", 1);
    g = Geometry::from_toml(&text).unwrap();
    assert_eq!(g.key.iter().filter(|k| k.effort == 9.0).count(), 1);
}

/// A one-hand 3 × 5 parametric grid (mirrored), the given column staggers, no efforts.
fn grid(stagger: [f64; 5], index_spread: f64) -> Geometry {
    let mut text = String::from("name = \"grid\"\n[split]\naxis = 5.0\npivot = [4.5, 1.5]\n[[zone]]\nname = \"fingers\"\nanchor = [0.5, 0.5]\nmirror = true\nrows = [0, 1, 2]\n");
    for (i, (finger, st)) in ["LP", "LR", "LM", "LI", "LI"].iter().zip(stagger).enumerate() {
        text += &format!("[[zone.column]]\nfinger = \"{finger}\"\nstagger = {st}\n");
        if i == 4 {
            text += &format!("stretch = true\nspread = {index_spread}\n");
        }
    }
    ParametricGeometry::from_toml(&text).unwrap().generate().unwrap()
}

#[test]
fn parametric_boards_derive_efforts_as_keys_move() {
    let flat = grid([0.0; 5], 1.0);
    assert!(flat.key.iter().all(|k| k.effort.is_finite()));
    let effort = |g: &Geometry| g.key.iter().find(|k| k.stretch && k.row == 1).unwrap().effort;
    // Staggering the index stretch column away from the home row makes its home-row key costlier.
    let wide = grid([0.0, 0.0, 0.0, 0.0, 0.5], 1.0);
    assert!(effort(&wide) > effort(&flat), "{} vs {}", effort(&wide), effort(&flat));
}

fn english() -> Option<Corpus> {
    std::fs::read(format!("{ROOT}/data/corpora/english.gkc")).ok().map(|b| Corpus::from_bytes(&b).unwrap())
}

#[test]
fn column_stagger_beats_row_stagger() {
    let Some(corpus) = english() else { return };
    let profile = Profile::from_toml(&read("data/profiles/default.toml")).unwrap();
    let row = Geometry::from_toml(&bare(&read("data/geometries/ansi-30.toml"))).unwrap();
    // Corne's column stagger (pinky lowest) on the 30-key grid.
    let col = grid([0.3, 0.1, 0.0, 0.1, 0.2], 1.0);
    assert_eq!(col.key.iter().filter(|k| k.stretch).count(), 6);
    // QWERTY is the exception (+0.56): its bottom-row bigrams were laid out for row stagger
    // and stretch more (B14) without it. See `context/geometry.md`.
    for name in ["colemak-dh", "graphite", "semimak", "sturdy", "dvorak"] {
        let l = Layout::from_toml(&read(&format!("data/layouts/{name}.toml"))).unwrap();
        let score = |g: &Geometry| {
            let sc = Scorer::new(g, &corpus, &profile, &l.fit(g).unwrap()).unwrap();
            sc.identity().score
        };
        let (r, c) = (score(&row), score(&col));
        assert!(c < r, "{name}: column stagger {c} vs row stagger {r}");
    }
}
