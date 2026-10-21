//! keyboard-layout-editor.com (KLE) JSON: import any board as a [`Geometry`] with guessed
//! fingers, rows, home keys and efforts; export a flat geometry back to KLE.
//! Heuristics: `context/representation.md#geometry`.

use serde_json::{json, Value};

use crate::geometry::{Finger, Geometry, Key};
use crate::{Error, Result};

/// One KLE key as the editor places it: unrotated top-left `x, y`, size `w, h`, then rotated by
/// `r` degrees (clockwise on screen) about `rx, ry`. Units are key units, y grows downward.
#[derive(Clone, Debug, PartialEq)]
pub struct KleKey {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub r: f32,
    pub rx: f32,
    pub ry: f32,
    pub label: String,
}

impl KleKey {
    /// Center after rotation.
    pub fn center(&self) -> [f32; 2] {
        let (px, py) = (self.x + self.w / 2.0 - self.rx, self.y + self.h / 2.0 - self.ry);
        let (s, c) = self.r.to_radians().sin_cos();
        [self.rx + px * c - py * s, self.ry + px * s + py * c]
    }
}

/// Parse KLE JSON (the downloaded file, or the editor's "Raw data" with unquoted keys and no
/// outer brackets). Returns the board name from the metadata object, if any, and the keys.
/// Decals are skipped; secondary rectangles (`x2`, `w2`, … of ISO Enter) are ignored.
pub fn parse(text: &str) -> Result<(Option<String>, Vec<KleKey>)> {
    let v: Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => serde_json::from_str(&format!("[{}]", quote_keys(text))).map_err(|e| Error::parse("kle", e))?,
    };
    let rows = v.as_array().ok_or_else(|| Error::invalid("kle", "expected an array of rows"))?;
    let bad = |m: &str| Error::invalid("kle", m.to_string());
    let num = |o: &serde_json::Map<String, Value>, k: &str| o.get(k).and_then(Value::as_f64).map(|f| f as f32);
    let (mut name, mut keys) = (None, Vec::new());
    // Editor state: current position, rotation and cluster origin persist across rows.
    let (mut x, mut y, mut r, mut rx, mut ry) = (0f32, 0f32, 0f32, 0f32, 0f32);
    let (mut w, mut h, mut decal) = (1f32, 1f32, false);
    for (ri, row) in rows.iter().enumerate() {
        let items = match row {
            Value::Array(a) => a,
            Value::Object(o) if ri == 0 => {
                name = o.get("name").and_then(Value::as_str).map(str::to_string);
                continue;
            }
            _ => return Err(bad("rows must be arrays (only the first may be a metadata object)")),
        };
        for item in items {
            match item {
                Value::Object(o) => {
                    if let Some(v) = num(o, "r") {
                        r = v;
                    }
                    if let Some(v) = num(o, "rx") {
                        rx = v;
                        (x, y) = (rx, ry);
                    }
                    if let Some(v) = num(o, "ry") {
                        ry = v;
                        (x, y) = (rx, ry);
                    }
                    x += num(o, "x").unwrap_or(0.0);
                    y += num(o, "y").unwrap_or(0.0);
                    w = num(o, "w").unwrap_or(w);
                    h = num(o, "h").unwrap_or(h);
                    decal = o.get("d").and_then(Value::as_bool).unwrap_or(decal);
                }
                Value::String(s) => {
                    if !decal {
                        keys.push(KleKey { x, y, w, h, r, rx, ry, label: s.clone() });
                    }
                    x += w;
                    (w, h, decal) = (1.0, 1.0, false);
                }
                _ => return Err(bad("row items must be strings (keys) or objects (properties)")),
            }
        }
        y += 1.0;
        x = rx;
    }
    if keys.is_empty() {
        return Err(bad("no keys"));
    }
    Ok((name, keys))
}

/// Quote bare object keys (`{x:1}` → `{"x":1}`) so the editor's raw data parses as JSON.
fn quote_keys(s: &str) -> String {
    let (mut out, mut in_str, mut esc, mut expect_key) = (String::with_capacity(s.len() + 64), false, false, false);
    let cs: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if in_str {
            out.push(c);
            (esc, in_str) = (!esc && c == '\\', esc || c != '"');
        } else if c == '"' {
            (in_str, expect_key) = (true, false);
            out.push(c);
        } else if expect_key && (c.is_ascii_alphabetic() || c == '_') {
            let start = i;
            while i < cs.len() && (cs[i].is_ascii_alphanumeric() || cs[i] == '_') {
                i += 1;
            }
            out.push('"');
            out.extend(&cs[start..i]);
            out.push('"');
            expect_key = false;
            continue;
        } else {
            if c == '{' || c == ',' {
                expect_key = true;
            } else if !c.is_whitespace() {
                expect_key = false;
            }
            out.push(c);
        }
        i += 1;
    }
    out
}

/// Base efforts by row (top, home, bottom) and column (pinky, ring, middle, index, index stretch):
/// `ortho-30`'s values.
const EFFORT: [[f32; 5]; 3] = [[3.0, 2.4, 2.0, 2.2, 3.0], [1.6, 1.3, 1.1, 1.0, 2.0], [3.2, 2.6, 2.3, 1.9, 3.2]];

/// A finger column of the home row.
struct Col {
    x: f32,
    finger: Finger,
    /// 0 pinky, 1 ring, 2 middle, 3 index, 4 index stretch.
    kind: usize,
    home: bool,
}

/// Import KLE JSON as a geometry named `name`. Physical data (position, rotation, size) is
/// exact; fingers, rows, columns, home keys and efforts are guesses to review by hand.
pub fn import(name: &str, text: &str) -> Result<Geometry> {
    let (title, keys) = parse(text)?;
    let centers: Vec<[f32; 2]> = keys.iter().map(KleKey::center).collect();
    let rows = cluster_rows(&keys, &centers);
    let n_rows = rows.len();
    let home = if n_rows >= 4 { n_rows - 3 } else { n_rows.min(2) - 1 };
    let thumb_row = (n_rows >= 4).then_some(n_rows - 1);

    // Home-row columns: its keys narrower than 1.5u, split into hands at a gap ≥ 1.5u (split
    // boards) or by count; fingers assigned from the inside out.
    let mut hk: Vec<usize> = rows[home].iter().copied().filter(|&i| keys[i].w < 1.5).collect();
    if hk.is_empty() {
        hk = rows[home].clone();
    }
    hk.sort_by(|&a, &b| centers[a][0].total_cmp(&centers[b][0]));
    let xs: Vec<f32> = hk.iter().map(|&i| centers[i][0]).collect();
    let gap = (1..xs.len()).max_by(|&a, &b| (xs[a] - xs[a - 1]).total_cmp(&(xs[b] - xs[b - 1])));
    let split = match gap {
        Some(g) if xs[g] - xs[g - 1] >= 1.5 => g,
        _ => xs.len() / 2,
    };
    let split = if xs.len() >= 2 { split.clamp(1, xs.len() - 1) } else { xs.len() };
    let mut cols: Vec<Col> = Vec::new();
    let hand = |xs: &[f32], left: bool| -> Vec<Col> {
        let m = xs.len();
        (0..m)
            .map(|j| {
                let from_inner = if left { m - 1 - j } else { j };
                let kind = if m >= 5 {
                    if from_inner == 0 {
                        4
                    } else {
                        4 - from_inner.min(4)
                    }
                } else {
                    3 - from_inner.min(3)
                };
                let ord = kind.min(3);
                let finger = Finger::ALL[if left { ord } else { 9 - ord }];
                let home = kind != 4 && from_inner <= if m >= 5 { 4 } else { 3 };
                Col { x: xs[j], finger, kind, home }
            })
            .collect()
    };
    cols.extend(hand(&xs[..split], true));
    cols.extend(hand(&xs[split..], false));
    let split_x = match (split, xs.len()) {
        (s, n) if s > 0 && s < n => (xs[s - 1] + xs[s]) / 2.0,
        _ => xs.iter().sum::<f32>() / xs.len().max(1) as f32,
    };
    let inner_index = |left: bool| {
        let idx = cols.iter().filter(|c| c.kind == 3 && c.finger.hand() == if left { crate::Hand::L } else { crate::Hand::R });
        let x = if left { idx.map(|c| c.x).fold(f32::NAN, f32::max) } else { idx.map(|c| c.x).fold(f32::NAN, f32::min) };
        if x.is_nan() {
            split_x
        } else {
            x + if left { 0.5 } else { -0.5 }
        }
    };

    let mut out: Vec<Key> = Vec::with_capacity(keys.len());
    for (ri, row) in rows.iter().enumerate() {
        let mut row = row.clone();
        row.sort_by(|&a, &b| centers[a][0].total_cmp(&centers[b][0]));
        let row_val = ri as i32 - home as i32 + 1;
        let thumb_rest = |left: bool| {
            let rest = inner_index(left);
            row.iter()
                .copied()
                .filter(|&i| (centers[i][0] < split_x) == left)
                .min_by(|&a, &b| (centers[a][0] - rest).abs().total_cmp(&(centers[b][0] - rest).abs()))
        };
        let rests = [thumb_rest(true), thumb_rest(false)];
        for (ci, &i) in row.iter().enumerate() {
            let k = &keys[i];
            let c = centers[i];
            let (finger, home_key, stretch, effort) = if Some(ri) == thumb_row {
                let left = c[0] < split_x;
                let rest = rests[!left as usize].map_or(c, |r| centers[r]);
                let d = ((c[0] - rest[0]).powi(2) + (c[1] - rest[1]).powi(2)).sqrt();
                (if left { Finger::LT } else { Finger::RT }, false, false, 1.0 + 0.5 * d)
            } else {
                // Nearest home-row column; ties go to the left one (ANSI bottom row: B → left index).
                let col = cols.iter().fold(None::<&Col>, |best, cl| match best {
                    Some(b) if (b.x - c[0]).abs() <= (cl.x - c[0]).abs() + 1e-3 => Some(b),
                    _ => Some(cl),
                });
                let Some(col) = col else { return Err(Error::invalid("kle", "no home row")) };
                let is_home = ri == home && col.home && hk.contains(&i) && (col.x - c[0]).abs() < 1e-3;
                // Outward from the pinky's home column, and rows beyond top/bottom, cost extra.
                let outward = if col.kind == 0 {
                    let left = col.finger.hand() == crate::Hand::L;
                    let home_px =
                        cols.iter().filter(|o| o.home && o.kind == 0 && o.finger == col.finger).map(|o| o.x).next().unwrap_or(col.x);
                    (if left { home_px - c[0] } else { c[0] - home_px } - 0.5).max(0.0)
                } else {
                    0.0
                };
                let extra_rows = (row_val - row_val.clamp(0, 2)).abs() as f32;
                let base = EFFORT[row_val.clamp(0, 2) as usize][col.kind];
                (col.finger, is_home, col.kind == 4, base + 0.8 * outward + extra_rows)
            };
            let tl = [c[0] - k.w / 2.0, c[1] - k.h / 2.0];
            out.push(Key {
                pos: [round(tl[0]), round(tl[1]), 0.0],
                rot: [0.0, 0.0, round(norm_deg(k.r))],
                size: [round(k.w), round(k.h)],
                finger,
                row: row_val.clamp(i8::MIN as i32, i8::MAX as i32) as i8,
                col: ci.min(i8::MAX as usize) as i8,
                home: home_key,
                stretch,
                effort: (effort * 10.0).round() / 10.0,
                fill: None,
                hrm: None,
                layer: 0,
                phys: None,
            });
        }
    }
    let src = title.map_or_else(String::new, |t| format!(" '{t}'"));
    let g = Geometry {
        name: name.to_string(),
        description: format!("Imported from KLE{src}; fingers, rows, home keys and efforts are guesses."),
        key: out,
        comfort: Default::default(),
        comfort_tables: Default::default(),
        layer_keys: Vec::new(),
        layer_modes: Vec::new(),
    };
    g.validate()?;
    Ok(g)
}

/// Group keys into rows, top to bottom. Unrotated keys that share an x center form columns;
/// within a column keys are numbered top-down, and each column's numbering is shifted to best
/// match its left neighbour (so column stagger doesn't split or merge rows). Remaining keys
/// (row-staggered boards, thumbs, rotated keys) join the row whose mean y is within 0.5u, or
/// start a new one.
fn cluster_rows(keys: &[KleKey], centers: &[[f32; 2]]) -> Vec<Vec<usize>> {
    let n = keys.len();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| centers[a][0].total_cmp(&centers[b][0]).then(centers[a][1].total_cmp(&centers[b][1])));
    // Columns: same x (±0.05), unrotated, consecutive keys 0.6..1.5u apart vertically.
    let mut columns: Vec<Vec<usize>> = Vec::new();
    let mut i = 0;
    while i < n {
        let mut j = i;
        while j < n && (centers[order[j]][0] - centers[order[i]][0]).abs() < 0.05 {
            j += 1;
        }
        let mut same: Vec<usize> = order[i..j].iter().copied().filter(|&k| norm_deg(keys[k].r) == 0.0).collect();
        same.sort_by(|&a, &b| centers[a][1].total_cmp(&centers[b][1]));
        let mut run: Vec<usize> = Vec::new();
        for k in same {
            if let Some(&last) = run.last() {
                let dy = centers[k][1] - centers[last][1];
                if !(0.6..=1.5).contains(&dy) {
                    if run.len() >= 2 {
                        columns.push(std::mem::take(&mut run));
                    }
                    run.clear();
                }
            }
            run.push(k);
        }
        if run.len() >= 2 {
            columns.push(run);
        }
        i = j;
    }
    // Align column numbering: shift s minimizing mean |dy| to the previous column's rows.
    let mut row_of = vec![i32::MIN; n];
    let mut prev: Option<(i32, &Vec<usize>)> = None;
    for col in &columns {
        let off = match prev {
            None => 0,
            // Equal-height finger columns retain their top-to-bottom numbering. A strong
            // stagger (Sweep pinky → ring is 0.62u) must not invent an extra logical row.
            Some((po, pc)) if col.len() == pc.len() && col.len() >= 3 => po,
            Some((po, pc)) => {
                let cost = |s: i32| {
                    let (mut sum, mut cnt) = (0f32, 0);
                    for (r, &k) in col.iter().enumerate() {
                        let pr = r as i32 + s - po;
                        if pr >= 0 && (pr as usize) < pc.len() {
                            sum += (centers[k][1] - centers[pc[pr as usize]][1]).abs();
                            cnt += 1;
                        }
                    }
                    if cnt == 0 {
                        f32::INFINITY
                    } else {
                        sum / cnt as f32
                    }
                };
                let mut best = po;
                for d in [1, -1, 2, -2, 3, -3] {
                    if cost(po + d) < cost(best) - 0.05 {
                        best = po + d;
                    }
                }
                if cost(best).is_infinite() {
                    po
                } else {
                    best
                }
            }
        };
        for (r, &k) in col.iter().enumerate() {
            row_of[k] = r as i32 + off;
        }
        prev = Some((off, col));
    }
    // Seed rows from the columns, then place the rest by y.
    let mut rows: Vec<(f32, Vec<usize>)> = Vec::new();
    if !columns.is_empty() {
        let (lo, hi) = (row_of.iter().filter(|&&r| r != i32::MIN).min().copied().unwrap(), row_of.iter().max().copied().unwrap());
        for r in lo..=hi {
            let ks: Vec<usize> = (0..n).filter(|&k| row_of[k] == r).collect();
            if !ks.is_empty() {
                rows.push((ks.iter().map(|&k| centers[k][1]).sum::<f32>() / ks.len() as f32, ks));
            }
        }
    }
    let mut loose: Vec<usize> = (0..n).filter(|&k| row_of[k] == i32::MIN).collect();
    loose.sort_by(|&a, &b| centers[a][1].total_cmp(&centers[b][1]).then(centers[a][0].total_cmp(&centers[b][0])));
    for k in loose {
        let y = centers[k][1];
        let near = (0..rows.len())
            .filter(|&r| (rows[r].0 - y).abs() < 0.5)
            .min_by(|&a, &b| (rows[a].0 - y).abs().total_cmp(&(rows[b].0 - y).abs()));
        match near {
            Some(r) => {
                let (m, ks) = &mut rows[r];
                *m = (*m * ks.len() as f32 + y) / (ks.len() + 1) as f32;
                ks.push(k);
            }
            None => rows.push((y, vec![k])),
        }
    }
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    rows.into_iter().map(|(_, ks)| ks).collect()
}

/// Degrees in (-180, 180].
fn norm_deg(d: f32) -> f32 {
    let d = d.rem_euclid(360.0);
    if d > 180.0 {
        d - 360.0
    } else {
        d
    }
}

fn round(v: f32) -> f32 {
    let r = (v * 1e4).round() / 1e4;
    if r == 0.0 {
        0.0
    } else {
        r
    }
}

/// A geometry as KLE JSON: one row per key, each placed by its rotation origin (the key center)
/// so any rotation survives; labels are finger names. 2D only: z, tilt and tent are dropped.
pub fn export(g: &Geometry) -> Value {
    let r4 = |v: f32| ((v as f64) * 1e4).round() / 1e4;
    let mut rows = vec![json!({"name": g.name})];
    for k in g.key.iter().filter(|k| k.layer == 0) {
        let (w, h) = (k.size[0], k.size[1]);
        let (cx, cy) = (k.pos[0] + w / 2.0, k.pos[1] + h / 2.0);
        let props = json!({"r": r4(k.rot[2]), "rx": r4(cx), "ry": r4(cy), "x": r4(-w / 2.0), "y": r4(-h / 2.0), "w": r4(w), "h": r4(h)});
        rows.push(json!([props, k.finger.name()]));
    }
    Value::Array(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_data_with_bare_keys_parses() {
        let (_, k) = parse(r#"["Q",{x:0.5,w:1.5},"W\"x"],[{y:0.25},"A"]"#).unwrap();
        assert_eq!(k.len(), 3);
        assert_eq!((k[1].x, k[1].w, k[1].label.as_str()), (1.5, 1.5, "W\"x"));
        assert_eq!((k[2].x, k[2].y), (0.0, 1.25));
    }

    #[test]
    fn rotation_origin_resets_position() {
        let (_, k) = parse(r#"[[{"r":90,"rx":2,"ry":1,"x":-0.5,"y":-0.5},"T"],["U"]]"#).unwrap();
        let c = k[0].center();
        assert!((c[0] - 2.0).abs() < 1e-5 && (c[1] - 1.0).abs() < 1e-5);
        // Next row: x back to rx, y one down: center (2.5, 2) turned 90° clockwise about (2, 1).
        let c = k[1].center();
        assert!((c[0] - 1.0).abs() < 1e-5 && (c[1] - 1.5).abs() < 1e-5, "{c:?}");
    }

    #[test]
    fn ansi_60_guesses_traditional_fingering() {
        let kle = r##"[
          ["~","!","@","#","$","%","^","&","*","(",")","_","+",{"w":2},"Bksp"],
          [{"w":1.5},"Tab","Q","W","E","R","T","Y","U","I","O","P","{","}",{"w":1.5},"|"],
          [{"w":1.75},"Caps","A","S","D","F","G","H","J","K","L",":","\"",{"w":2.25},"Enter"],
          [{"w":2.25},"Shift","Z","X","C","V","B","N","M","<",">","?",{"w":2.75},"Shift"],
          [{"w":1.25},"Ctrl",{"w":1.25},"Win",{"w":1.25},"Alt",{"a":7,"w":6.25},"",{"w":1.25},"Alt",{"w":1.25},"Win",{"w":1.25},"Menu",{"w":1.25},"Ctrl"]
        ]"##;
        let (_, ks) = parse(kle).unwrap();
        let g = import("ansi-60", kle).unwrap();
        assert_eq!(g.row_lengths(), vec![14, 14, 13, 12, 8]);
        // Keys are listed row by row, left to right; find each by its label's position.
        let at = |label: &str| {
            let c = ks.iter().find(|k| k.label == label).unwrap().center();
            g.key
                .iter()
                .find(|k| (k.pos[0] + k.size[0] / 2.0 - c[0]).abs() < 1e-3 && (k.pos[1] + k.size[1] / 2.0 - c[1]).abs() < 1e-3)
                .unwrap()
        };
        let want = [
            ("Q", "LP"),
            ("T", "LI"),
            ("Y", "RI"),
            ("P", "RP"),
            ("A", "LP"),
            ("F", "LI"),
            ("G", "LI"),
            ("H", "RI"),
            ("J", "RI"),
            ("K", "RM"),
            (":", "RP"),
            ("Z", "LP"),
            ("B", "LI"),
            ("N", "RI"),
            ("?", "RP"),
            ("Caps", "LP"),
            ("Enter", "RP"),
            ("Bksp", "RP"),
            ("~", "LP"),
        ];
        for (l, f) in want {
            assert_eq!(at(l).finger.name(), f, "{l}");
        }
        let homes: Vec<&str> = ["A", "S", "D", "F", "J", "K", "L", ":"].into_iter().filter(|l| at(l).home).collect();
        assert_eq!(homes.len(), 8);
        assert!(!at("G").home && at("G").stretch && at("B").stretch && !at("Caps").home);
        assert_eq!((at("Q").row, at("A").row, at("Z").row, at("~").row, at("Ctrl").row), (0, 1, 2, -1, 3));
        assert!(at("Ctrl").finger.is_thumb());
    }
}
