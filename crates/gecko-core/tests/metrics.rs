//! Every implemented metric on a tiny board against a value computed by hand. Each check goes
//! through the report path (`breakdown`) and the folded scorer (`full` with only that metric
//! weighted), so both must agree with the arithmetic in the comments.
//!
//! Corpora are built from explicit counts, so every table is normalized over a known total
//! (usually 100, so counts read as %).

use std::collections::BTreeMap;

use gecko_core::metrics::{def, Dir, Order};
use gecko_core::report::breakdown;
use gecko_core::{Corpus, Finger, Geometry, Key, Layout, Profile, Scorer};

// ---- builders ----

fn key(finger: Finger, row: i8, col: i8, x: f32, y: f32) -> Key {
    Key {
        pos: [x, y, 0.0],
        rot: [0.0; 3],
        size: [1.0, 1.0],
        finger,
        row,
        col,
        home: false,
        stretch: false,
        effort: 1.0,
        fill: None,
        layer: 0,
        phys: None,
    }
}

fn board(key: Vec<Key>) -> Geometry {
    let g = Geometry {
        name: "tiny".into(),
        description: String::new(),
        key,
        comfort: BTreeMap::new(),
        comfort_tables: BTreeMap::new(),
        layer_keys: Vec::new(),
    };
    g.validate().unwrap();
    g
}

/// Counts by n-gram: `x` mono, `xy` bigram, `xyz` trigram, `x_z` / `x__z` / `x___z` skipgrams
/// with 1 / 2 / 3 keys between (`_` is never a symbol here).
fn corpus(charset: &str, grams: &[(&str, u64)]) -> Corpus {
    let mut c = Corpus::new("tiny", charset);
    let n = c.size();
    for &(g, count) in grams {
        let s: Vec<usize> = g.chars().filter(|&ch| ch != '_').map(|ch| c.index_of(ch).unwrap()).collect();
        let gap = g.chars().filter(|&ch| ch == '_').count();
        match (&s[..], gap) {
            (&[x], 0) => c.mono[x] += count,
            (&[x, y], 0) => c.bi[x * n + y] += count,
            (&[x, y, z], 0) => c.tri[(x * n + y) * n + z] += count,
            (&[x, z], 1) => c.skip[x * n + z] += count,
            (&[x, z], 2) => c.skip2[x * n + z] += count,
            (&[x, z], 3) => c.skip3[x * n + z] += count,
            _ => panic!("bad n-gram '{g}'"),
        }
    }
    c
}

/// No trigram truncation, default `sfb` parameters unless `extra` sets them.
fn profile(extra: &str) -> Profile {
    Profile::from_toml(&format!("name = \"t\"\ntrigram_coverage = 1.0\n{extra}")).unwrap()
}

/// Score `layout` with only `id` weighted (1.0) and return (breakdown value, full score).
fn eval(g: &Geometry, c: &Corpus, p: &Profile, layout: &Layout, id: &str) -> (f64, f64) {
    let mut p = p.clone();
    p.weights = BTreeMap::from([(id.to_string(), 1.0)]);
    let sc = Scorer::new(g, c, &p, layout).unwrap();
    let st = sc.identity();
    let v = breakdown(&sc, &st).metrics.iter().find(|m| m.id == id).unwrap().value;
    (v, sc.full(&st))
}

fn close(id: &str, got: f64, want: f64) {
    assert!((got - want).abs() < 1e-4 * want.abs().max(1.0), "{id}: got {got}, want {want}");
}

/// Assert metric `id` == `want`; for linear metrics also that the folded score is ±`want`.
fn check(g: &Geometry, c: &Corpus, p: &Profile, layout: &Layout, id: &str, want: f64) {
    let (v, score) = eval(g, c, p, layout, id);
    close(id, v, want);
    let d = def(id).unwrap();
    if d.order != Order::Load {
        let signed = match d.dir {
            Dir::Min => want,
            Dir::Max => -want,
            Dir::Info => 0.0,
        };
        close(id, score, signed);
    }
}

// ---- the tiny board ----
//
//   slot  sym  finger  row col  pos (x, y)  flags
//   0     a    LM      0   2    (2, 0)      effort 3
//   1     b    LI      1   3    (3, 1)      home
//   2     c    LI      1   4    (4, 1)      stretch, effort 2
//   3     d    LR      2   1    (1, 2)      effort 4
//   4     e    RI      1   6    (6, 1)      home
//   5     f    RM      1   7    (7, 1)      home
//
// Same finger: b–c (LI, 1u apart). LSB: a–c (middle + index on its stretch key).
// Scissor: a–d (LM row 0, LR row 2). Other hand: e, f.

const ABC: &str = "abcdef";

fn tiny() -> Geometry {
    let mut k = vec![
        key(Finger::LM, 0, 2, 2.0, 0.0),
        key(Finger::LI, 1, 3, 3.0, 1.0),
        key(Finger::LI, 1, 4, 4.0, 1.0),
        key(Finger::LR, 2, 1, 1.0, 2.0),
        key(Finger::RI, 1, 6, 6.0, 1.0),
        key(Finger::RM, 1, 7, 7.0, 1.0),
    ];
    k[0].effort = 3.0;
    k[1].home = true;
    k[2].stretch = true;
    k[2].effort = 2.0;
    k[3].effort = 4.0;
    k[4].home = true;
    k[5].home = true;
    board(k)
}

fn tiny_layout() -> Layout {
    Layout::from_str_flat("t", "tiny", ABC).unwrap()
}

/// Every table sums to 100 (far skipgrams to 100 per gap), so counts read as %.
fn tiny_corpus() -> Corpus {
    corpus(
        ABC,
        &[
            // mono: left a+b+c+d = 60, right e+f = 40
            ("a", 10),
            ("b", 20),
            ("c", 10),
            ("d", 20),
            ("e", 25),
            ("f", 15),
            // bigrams
            ("bc", 10), // SFB
            ("cb", 5),  // SFB
            ("ac", 8),  // LSB
            ("ca", 2),  // LSB
            ("ad", 6),  // scissor
            ("da", 4),  // scissor
            ("ae", 20), // alternate
            ("fb", 15), // alternate
            ("ef", 10), // same hand, neighbours on one row
            ("bb", 5),  // same key
            ("ab", 15), // same hand, one row apart
            // skipgrams, gap 1
            ("b_c", 30),
            ("c_b", 10),
            ("a_e", 60),
            // skipgrams, gap 2 and 3 (each normalized on its own)
            ("b__c", 20),
            ("a__e", 80),
            ("c___b", 50),
            ("a___f", 50),
            // trigrams, one per class
            ("aeb", 10), // alternate (LM R LI)
            ("beb", 5),  // alternate: same finger but the same key, so not T02
            ("bec", 8),  // alternate SFS (LI R LI, different keys)
            ("dae", 12), // roll in (LR → LM, then right)
            ("aef", 9),  // roll out (left, then RI → RM)
            ("dab", 11), // one-hand roll (LR → LM → LI)
            ("dba", 7),  // redirect (LR → LI → LM, index involved)
            ("ada", 6),  // bad redirect (LM → LR → LM, no index)
            ("bcb", 4),  // same-finger trigram (LI LI LI)
            ("bce", 28), // other (same-finger bigram inside)
        ],
    )
}

fn check_tiny(id: &str, want: f64) {
    check(&tiny(), &tiny_corpus(), &profile(""), &tiny_layout(), id, want);
}

// ---- mono and load ----

#[test]
fn m01_effort() {
    // (10·3 + 20·1 + 10·2 + 20·4 + 25·1 + 15·1) / 100 = 190 / 100
    check_tiny("M01", 1.9);
}

#[test]
fn m05_home_row() {
    // Home row = row 1 (b's row): b + c + e + f = 20 + 10 + 25 + 15 (c counts by row, not flag)
    check_tiny("M05", 70.0);
}

#[test]
fn m06_center_columns() {
    // Stretch keys: c = 10
    check_tiny("M06", 10.0);
}

#[test]
fn m02_finger_load_error() {
    // Loads LP 0, LR d=20, LM a=10, LI b+c=30, RI e=25, RM f=15, RR 0, RP 0; targets
    // 8 11 15 16 | 16 15 11 8. Squared gaps: 64 + 81 + 25 + 196 + 81 + 0 + 121 + 64 = 632.
    // Cost = 632 / 8 = 79 (%²); value = √79.
    let (v, score) = eval(&tiny(), &tiny_corpus(), &profile(""), &tiny_layout(), "M02");
    close("M02", v, 79f64.sqrt());
    close("M02", score, 79.0);
}

#[test]
fn m04_left_hand() {
    // Left = 60 of 100 → 60%; cost (60 − 50)² / 10 = 10.
    let (v, score) = eval(&tiny(), &tiny_corpus(), &profile(""), &tiny_layout(), "M04");
    close("M04", v, 60.0);
    close("M04", score, 10.0);
}

// ---- bigrams ----

#[test]
fn b01_sfb() {
    // bc + cb = 10 + 5 (bb is the same key, not an SFB)
    check_tiny("B01", 15.0);
}

#[test]
fn b02_same_key() {
    // bb = 5
    check_tiny("B02", 5.0);
}

#[test]
fn b03_finger_speed() {
    // bc + cb = 15% at distance 1u, index factor 1.0: 0.15 · 100 · 1 · 1.0
    check_tiny("B03", 15.0);
    // Squared distance, lateral ×2, index factor 1.5: d = 2·1 = 2 → 2² · 1.5 = 6 → 0.15 · 100 · 6
    let p = profile("[sfb]\ndistance = \"squared\"\nlateral = 2.0\nfinger_factor = [2.0, 1.538, 1.111, 1.5, 1.0]");
    check(&tiny(), &tiny_corpus(), &p, &tiny_layout(), "B03", 90.0);
}

#[test]
fn b04_lsb() {
    // ac + ca = 8 + 2
    check_tiny("B04", 10.0);
}

#[test]
fn b05_scissors() {
    // ad + da = 6 + 4 (LM row 0 ↔ LR row 2); ab is only one row apart
    check_tiny("B05", 10.0);
}

#[test]
fn b08_alternation() {
    // ae + fb = 20 + 15
    check_tiny("B08", 35.0);
}

#[test]
fn b14_stretch() {
    // Oxeylyzer 2 stretch, same hand / different fingers only. y += finger length offset
    // (ring +0.35, middle +0.25, index −0.30); s = hypot(dx, dy) + overlap − 1.35 · |Δfinger|.
    //   a–b: dx 1, dy |0.25 − 0.70| = 0.45 → √1.2025 − 1.35 < 0 → 0
    //   a–c: dx 2, dy 0.45 → √4.2025 = 2.05 → 2.05 − 1.35 = 0.70
    //   a–d: dx 1, dy |0.25 − 2.35| = 2.1, x-overlap 0.4: 0.4 − 1.3 + 0.7 < 0 → no overlap term;
    //        √5.41 − 1.35 = 0.97594
    //   e–f: dx 1, dy |0.70 − 1.25| = 0.55 → √1.3025 − 1.35 < 0 → 0
    // B14 = 100 · (0.10 · 0.70 + 0.10 · 0.97594)  (ac + ca = 10%, ad + da = 10%)
    check_tiny("B14", 10.0 * 0.70 + 10.0 * (5.41f64.sqrt() - 1.35));
}

#[test]
fn b14_stretch_crossed_fingers() {
    // Middle finger on (3, 1), index on (2, 1): the middle sits right of the index on the left
    // hand, so the fingers cross. dx = −1, dy = |1.25 − 0.70| = 0.55, x-overlap 0.1:
    // overlap = 0.1 + 1.3 + 0.55 / 3 = 1.58333; s = √1.3025 + 1.58333 − 1.35 = 1.37461.
    let g = board(vec![key(Finger::LM, 1, 3, 3.0, 1.0), key(Finger::LI, 1, 2, 2.0, 1.0)]);
    let c = corpus("ab", &[("a", 1), ("b", 1), ("ab", 1)]);
    let l = Layout::from_str_flat("t", "tiny", "ab").unwrap();
    check(&g, &c, &profile(""), &l, "B14", 100.0 * (1.3025f64.sqrt() + 0.1 + 1.3 + 0.55 / 3.0 - 1.35));
}

#[test]
fn e04_e05_comfort() {
    // Comfort tables by (row, col); unlisted pairs score 1.0; same-key pairs are forced to 1.0.
    let mut g = tiny();
    g.load_comfort("E04", "a_row,a_col,b_row,b_col,score\n# b→c\n1,3,1,4,0.2\n# a→e\n0,2,1,6,0.9\n# b→b (ignored)\n1,3,1,3,0.0")
        .unwrap();
    g.load_comfort("E05", "a_row,a_col,b_row,b_col,score\n# e→f\n1,6,1,7,0.5").unwrap();
    let (c, p, l) = (tiny_corpus(), profile(""), tiny_layout());
    // E04: bc 10 · (1 − 0.2) + ae 20 · (1 − 0.9) = 8 + 2
    check(&g, &c, &p, &l, "E04", 10.0);
    // E05: ef 10 · (1 − 0.5)
    check(&g, &c, &p, &l, "E05", 5.0);
}

// ---- skipgrams ----

#[test]
fn s01_sfs() {
    // b_c + c_b = 30 + 10
    check_tiny("S01", 40.0);
}

#[test]
fn s02_far_sfs() {
    // gap 2: b__c = 20%; gap 3: c___b = 50%; decay 0.5 → 0.5 · 20 + 0.5² · 50 = 10 + 12.5
    check_tiny("S02", 22.5);
}

#[test]
fn s03_sfs_distance() {
    // b_c + c_b = 40% at 1u, index factor 1.0
    check_tiny("S03", 40.0);
}

// ---- trigrams (counts sum to 100) ----

#[test]
fn t01_alternates() {
    // aeb + beb = 10 + 5
    check_tiny("T01", 15.0);
}

#[test]
fn t02_alternate_sfs() {
    // bec
    check_tiny("T02", 8.0);
}

#[test]
fn t03_rolls_in() {
    // dae
    check_tiny("T03", 12.0);
}

#[test]
fn t04_rolls_out() {
    // aef
    check_tiny("T04", 9.0);
}

#[test]
fn t05_onehand_rolls() {
    // dab (reported only: the folded score is 0)
    check_tiny("T05", 11.0);
}

#[test]
fn t06_redirects() {
    // dba
    check_tiny("T06", 7.0);
}

#[test]
fn t07_bad_redirects() {
    // ada
    check_tiny("T07", 6.0);
}

#[test]
fn t08_sft() {
    // bcb
    check_tiny("T08", 4.0);
}

// ---- layers ----
//
//   slot  base  layer 1  finger  pos
//   0     ;     (        LI      (3, 1)
//   1     b     )        LM      (2, 1)
//   2     ,     =        RI      (6, 1)
//   3     ①     ·        LT      (4, 3)   holds layer 1
//   4     d     ·        LT      (5, 3)
//   5     .     ·        LI      (3, 0)
//
// The virtual geometry adds layer-1 copies of the non-LT keys: slots 6 `(`, 7 `)`, 8 `=`, 9 `·`.

fn layered() -> (Geometry, Layout) {
    let g = board(vec![
        key(Finger::LI, 1, 3, 3.0, 1.0),
        key(Finger::LM, 1, 2, 2.0, 1.0),
        key(Finger::RI, 1, 6, 6.0, 1.0),
        key(Finger::LT, 3, 4, 4.0, 3.0),
        key(Finger::LT, 3, 5, 5.0, 3.0),
        key(Finger::LI, 0, 3, 3.0, 0.0),
    ]);
    let l = Layout::from_toml("name = \"t\"\nrows = [\";b,{layer1}d.\"]\n[[layer]]\nrows = [\"()={none}{none}{none}\"]").unwrap();
    (g, l)
}

fn layered_corpus() -> Corpus {
    corpus(
        ";b,d.()=",
        &[
            // mono: thumb d = 40; non-thumb ; b , = 60
            (";", 20),
            ("b", 20),
            (",", 20),
            ("d", 40),
            // bigrams (sum 100; symbol bigrams ; ( ) , = . sum 80)
            (";(", 10), // base → layer: 1 press
            ("d(", 5),  // from the holding thumb's other key: 2 presses
            ("()", 35), // stays on the layer: no press
            ("(;", 15), // back to base: no press
            ("b;", 15), // base → base
            (",=", 10), // base → layer: 1 press
            (";.", 10), // base, same finger (LI), different keys
        ],
    )
}

#[test]
fn y01_layer_switches() {
    let (g, l) = layered();
    // ;( 10 · 1 + d( 5 · 2 + ,= 10 · 1 = 30 presses per 100 bigrams
    check(&g, &layered_corpus(), &profile(""), &l, "Y01", 30.0);
}

#[test]
fn y02_symbol_bigrams() {
    let (g, l) = layered();
    // Bad symbol bigrams: ;( 10 (layer change) + (; 15 (layer change) + ,= 10 (layer change)
    // + ;. 10 (same finger); () is fine. (10 + 15 + 10 + 10) / 80 = 56.25%
    check(&g, &layered_corpus(), &profile(""), &l, "Y02", 56.25);
}

#[test]
fn m02_m04_with_thumbs() {
    let (g, l) = layered();
    let (c, p) = (layered_corpus(), profile(""));
    // M02 uses shares of non-thumb keystrokes (60): LI ; = 20/60, LM b = 20/60, RI , = 20/60,
    // i.e. 100/3 % each. Squared gaps: LP 8² + LR 11² + LM (100/3 − 15)² + LI (100/3 − 16)²
    // + RI (100/3 − 16)² + RM 15² + RR 11² + RP 8² = 64 + 121 + 3025/9 + 2 · 2704/9 + 225 + 121 + 64
    // = 595 + 937 = 1532; cost 1532 / 8 = 191.5.
    let (v, score) = eval(&g, &c, &p, &l, "M02");
    close("M02", v, 191.5f64.sqrt());
    close("M02", score, 191.5);
    // M04 counts thumbs: left = ; + b + d = 80% → (80 − 50)² / 10 = 90.
    let (v, score) = eval(&g, &c, &p, &l, "M04");
    close("M04", v, 80.0);
    close("M04", score, 90.0);
}
