//! Community layouts from cmini (the AKL Discord bot): import into one bundle, detect duplicates
//! and mirrors, and test how GECKO's ranking agrees with community likes. See
//! `context/representation.md` (Community layouts) and `context/objective.md` (Calibration).
//!
//! A cmini layout lists `keys: {symbol: {row, col, finger}}` on a board (`ortho`, `stagger`,
//! `angle`). GECKO keeps the 30-key core (rows 0–2, columns 0–9) as written; layouts with a
//! letter off the core (thumb letters, a 31st column), a missing letter, or non-standard fingering
//! are skipped with a reason. Angle-mod layouts keep their key positions (the bottom-left fingers
//! differ only in cmini's fingermap).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::calibrate::spearman;
use crate::layout::EMPTY;
use crate::metrics::{Dir, METRICS};
use crate::report::breakdown;
use crate::{Corpus, Error, Geometry, Layout, Profile, Result, Scorer};

pub const ROWS: usize = 3;
pub const COLS: usize = 10;

/// cmini's standard fingering per column, and the angle-mod bottom row.
const STANDARD: [&str; COLS] = ["LP", "LR", "LM", "LI", "LI", "RI", "RI", "RM", "RR", "RP"];
const ANGLE_BOTTOM: [&str; COLS] = ["LR", "LM", "LI", "LI", "LI", "RI", "RI", "RM", "RR", "RP"];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CommunityLayout {
    pub name: String,
    #[serde(default)]
    pub author: String,
    /// cmini board: `ortho`, `stagger`, `angle` or `mini`.
    pub board: String,
    pub likes: u32,
    /// Three rows of ten symbols; [`EMPTY`] where cmini has no key.
    pub rows: Vec<String>,
    /// Name of the group's representative when this layout repeats another one (same 30 keys, or
    /// their left–right mirror). The representative has the most likes, then the shortest name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dup_of: Option<String>,
    /// The duplicate is the mirror image of `dup_of`, not an exact copy.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub mirror: bool,
}

impl CommunityLayout {
    pub fn chars(&self) -> Vec<char> {
        self.rows.concat().chars().collect()
    }

    /// The layout on a 30-key geometry (named `g`).
    pub fn layout(&self, geometry: &str) -> Result<Layout> {
        Layout::from_chars(&self.name, geometry, &format!("cmini ({}, {})", self.author, self.board), self.chars())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Skipped {
    pub name: String,
    /// `<kind>: <detail>`, e.g. `missing letters: qz`.
    pub reason: String,
}

impl Skipped {
    pub fn kind(&self) -> &str {
        self.reason.split(':').next().unwrap_or_default()
    }
}

/// `data/community/cmini.json`: every importable layout plus the skip list.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Bundle {
    pub schema: u32,
    pub source: String,
    pub layouts: Vec<CommunityLayout>,
    pub skipped: Vec<Skipped>,
}

impl Bundle {
    pub fn from_json(s: &str) -> Result<Self> {
        serde_json::from_str(s).map_err(|e| Error::parse("cmini bundle", e))
    }

    /// One layout per line: compact, and diffs stay readable.
    pub fn to_json(&self) -> String {
        fn list<T: Serialize>(items: &[T]) -> String {
            let lines: Vec<String> = items.iter().map(|v| serde_json::to_string(v).unwrap_or_default()).collect();
            if lines.is_empty() {
                "[]".into()
            } else {
                format!("[\n    {}\n  ]", lines.join(",\n    "))
            }
        }
        format!(
            "{{\n  \"schema\": {},\n  \"source\": {},\n  \"layouts\": {},\n  \"skipped\": {}\n}}\n",
            self.schema,
            serde_json::to_string(&self.source).unwrap_or_default(),
            list(&self.layouts),
            list(&self.skipped),
        )
    }

    pub fn get(&self, name: &str) -> Option<&CommunityLayout> {
        self.layouts.iter().find(|l| l.name == name).or_else(|| self.layouts.iter().find(|l| l.name.eq_ignore_ascii_case(name)))
    }

    /// Layouts that are not duplicates or mirrors of another one.
    pub fn unique(&self) -> impl Iterator<Item = &CommunityLayout> {
        self.layouts.iter().filter(|l| l.dup_of.is_none())
    }
}

/// One cmini layout file → its 30-key core, or why it can't be used.
pub fn parse_cmini(json: &str, likes: u32, author: &str) -> std::result::Result<CommunityLayout, Skipped> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| Skipped { name: String::new(), reason: format!("bad JSON: {e}") })?;
    let name = v["name"].as_str().unwrap_or_default().to_string();
    let skip = |reason: String| Skipped { name: name.clone(), reason };
    let board = v["board"].as_str().unwrap_or_default().to_string();
    let keys = v["keys"].as_object().ok_or_else(|| skip("no keys".into()))?;
    let mut grid = [[EMPTY; COLS]; ROWS];
    for (sym, k) in keys {
        let mut cs = sym.chars();
        let (Some(c), None) = (cs.next(), cs.next()) else { continue };
        let c = c.to_ascii_lowercase();
        let (row, col) = (k["row"].as_u64().unwrap_or(u64::MAX) as usize, k["col"].as_u64().unwrap_or(u64::MAX) as usize);
        let finger = k["finger"].as_str().unwrap_or_default();
        if row >= ROWS || col >= COLS || matches!(finger, "LT" | "RT" | "TB") {
            if c.is_ascii_lowercase() {
                return Err(skip(format!("letter off the 30-key core: '{c}' at row {row} col {col} ({finger})")));
            }
            continue;
        }
        let want = if board == "angle" && row == 2 { ANGLE_BOTTOM[col] } else { STANDARD[col] };
        if finger != want {
            return Err(skip(format!("non-standard fingering: {finger} at row {row} col {col}")));
        }
        if grid[row][col] != EMPTY {
            return Err(skip(format!("two symbols on one key: row {row} col {col}")));
        }
        grid[row][col] = c;
    }
    let present: String = grid.iter().flatten().collect();
    let missing: String = ('a'..='z').filter(|c| !present.contains(*c)).collect();
    if !missing.is_empty() {
        return Err(skip(format!("missing letters: {missing}")));
    }
    if present.chars().filter(|&c| c != EMPTY).count()
        != present.chars().filter(|&c| c != EMPTY).collect::<std::collections::BTreeSet<_>>().len()
    {
        return Err(skip("duplicate symbol: a symbol appears twice".into()));
    }
    Ok(CommunityLayout {
        name,
        author: author.to_string(),
        board,
        likes,
        rows: grid.iter().map(|r| r.iter().collect()).collect(),
        dup_of: None,
        mirror: false,
    })
}

/// The left–right mirror of a 3×10 core.
pub fn mirrored(chars: &[char]) -> Vec<char> {
    chars.chunks(COLS).flat_map(|r| r.iter().rev().copied()).collect()
}

/// Mark duplicates and mirrors: layouts with the same 30 keys (or their mirror) form a group;
/// all but the representative (most likes, then shortest name, then name) get `dup_of`.
/// Returns the number of groups with more than one layout.
pub fn mark_duplicates(layouts: &mut [CommunityLayout]) -> usize {
    let mut groups: BTreeMap<Vec<char>, Vec<usize>> = BTreeMap::new();
    for (i, l) in layouts.iter().enumerate() {
        let c = l.chars();
        let m = mirrored(&c);
        groups.entry(c.min(m)).or_default().push(i);
    }
    let mut multi = 0;
    for idx in groups.values().filter(|v| v.len() > 1) {
        multi += 1;
        let rep = *idx
            .iter()
            .min_by(|&&a, &&b| {
                let (la, lb) = (&layouts[a], &layouts[b]);
                lb.likes.cmp(&la.likes).then(la.name.len().cmp(&lb.name.len())).then(la.name.cmp(&lb.name))
            })
            .unwrap();
        let (rep_name, rep_chars) = (layouts[rep].name.clone(), layouts[rep].chars());
        for &i in idx.iter().filter(|&&i| i != rep) {
            layouts[i].mirror = layouts[i].chars() != rep_chars;
            layouts[i].dup_of = Some(rep_name.clone());
        }
    }
    multi
}

/// Read a cmini checkout (`layouts/*.json`, `likes.json`, `authors.json`) into a bundle, sorted
/// by name, duplicates marked.
pub fn import_cmini_dir(dir: &std::path::Path) -> Result<Bundle> {
    let read = |p: std::path::PathBuf| std::fs::read_to_string(&p).map_err(|e| Error::invalid("cmini", format!("{}: {e}", p.display())));
    let likes: BTreeMap<String, Vec<serde_json::Value>> =
        serde_json::from_str(&read(dir.join("likes.json")).unwrap_or_else(|_| "{}".into())).map_err(|e| Error::parse("likes.json", e))?;
    let authors: BTreeMap<String, serde_json::Value> =
        serde_json::from_str(&read(dir.join("authors.json")).unwrap_or_else(|_| "{}".into()))
            .map_err(|e| Error::parse("authors.json", e))?;
    let by_id: BTreeMap<String, String> = authors.into_iter().map(|(name, id)| (id.to_string(), name)).collect();
    let mut files: Vec<_> = std::fs::read_dir(dir.join("layouts"))
        .map_err(|e| Error::invalid("cmini", format!("{}: {e}", dir.join("layouts").display())))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    let (mut layouts, mut skipped) = (Vec::new(), Vec::new());
    for f in files {
        let text = read(f.clone())?;
        let v: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
        let name = v["name"].as_str().map(str::to_string).unwrap_or_else(|| f.file_stem().unwrap().to_string_lossy().into_owned());
        let n_likes = likes.get(&name).map_or(0, |v| v.len() as u32);
        let author = by_id.get(&v["user"].to_string()).cloned().unwrap_or_default();
        match parse_cmini(&text, n_likes, &author) {
            Ok(l) => layouts.push(l),
            Err(s) => skipped.push(Skipped { name, reason: s.reason }),
        }
    }
    layouts.sort_by(|a, b| a.name.cmp(&b.name));
    mark_duplicates(&mut layouts);
    Ok(Bundle {
        schema: crate::SCHEMA,
        source: "cmini layouts, likes and authors (https://github.com/Apsu/cmini, GPL-3.0; layout data by its authors)".into(),
        layouts,
        skipped,
    })
}

/// One scored community layout.
#[derive(Clone, Debug, Serialize)]
pub struct Scored {
    pub name: String,
    pub board: String,
    pub likes: u32,
    pub score: f64,
    /// GECKO rank among the scored layouts (1 = best).
    pub rank: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct Agreement {
    /// Layouts in this slice.
    pub n: usize,
    /// Spearman ρ of GECKO goodness (−score) vs likes; > 0 means GECKO agrees with the likes.
    pub rho: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct LikesReport {
    pub geometry: String,
    /// Geometry the `angle`-board layouts were scored on (angle-mod fingering), if not `geometry`.
    pub angle_geometry: Option<String>,
    pub profile: String,
    pub corpus: String,
    /// All scored layouts (duplicates and mirrors left out).
    pub all: Agreement,
    /// Only layouts with at least one like.
    pub liked: Agreement,
    /// Per cmini board.
    pub boards: BTreeMap<String, Agreement>,
    /// Mean GECKO rank percentile (0 = best) of liked and unliked layouts.
    pub liked_percentile: f64,
    pub unliked_percentile: f64,
    /// Spearman ρ of each metric's "goodness" (value, sign-flipped for Min metrics) vs likes over
    /// all layouts: which metrics the community's likes reward.
    pub metrics: Vec<(String, f64)>,
    pub layouts: Vec<Scored>,
}

fn agreement<'a>(xs: impl Iterator<Item = &'a Scored>) -> Agreement {
    let (good, likes): (Vec<f64>, Vec<f64>) = xs.map(|s| (-s.score, s.likes as f64)).unzip();
    Agreement { n: good.len(), rho: spearman(&good, &likes) }
}

/// Score every unique community layout on `g` and correlate GECKO's ranking with likes.
/// Layouts from cmini's `angle` board are scored on `angle` instead when given (the same keys
/// with angle-mod fingering, e.g. `ansi-30-angle`).
pub fn likes_report<'a>(
    g: &Geometry,
    angle: Option<&Geometry>,
    corpus: &Corpus,
    profile: &Profile,
    layouts: impl Iterator<Item = &'a CommunityLayout>,
) -> Result<LikesReport> {
    let mut scored = Vec::new();
    let mut values: Vec<Vec<f64>> = Vec::new();
    for c in layouts {
        let g = angle.filter(|_| c.board == "angle").unwrap_or(g);
        let l = c.layout(&g.name)?.fit(g)?;
        let sc = Scorer::new(g, corpus, profile, &l)?;
        let b = breakdown(&sc, &sc.identity());
        values.push(b.metrics.iter().map(|m| m.value).collect());
        scored.push(Scored { name: c.name.clone(), board: c.board.clone(), likes: c.likes, score: b.score, rank: 0 });
    }
    let mut order: Vec<usize> = (0..scored.len()).collect();
    order.sort_by(|&a, &b| scored[a].score.total_cmp(&scored[b].score).then(scored[a].name.cmp(&scored[b].name)));
    for (r, &i) in order.iter().enumerate() {
        scored[i].rank = r + 1;
    }
    let n = scored.len().max(1) as f64;
    let pct = |liked: bool| {
        let v: Vec<f64> = scored.iter().filter(|s| (s.likes > 0) == liked).map(|s| (s.rank - 1) as f64 / n).collect();
        if v.is_empty() {
            f64::NAN
        } else {
            v.iter().sum::<f64>() / v.len() as f64
        }
    };
    let likes: Vec<f64> = scored.iter().map(|s| s.likes as f64).collect();
    let metrics = METRICS
        .iter()
        .enumerate()
        .filter_map(|(i, d)| {
            let sign = if d.dir == Dir::Max { 1.0 } else { -1.0 };
            let x: Vec<f64> = values.iter().map(|v| sign * v[i]).collect();
            let rho = spearman(&x, &likes);
            rho.is_finite().then(|| (d.id.to_string(), rho))
        })
        .collect();
    let mut boards = BTreeMap::new();
    for b in scored.iter().map(|s| s.board.clone()).collect::<std::collections::BTreeSet<_>>() {
        boards.insert(b.clone(), agreement(scored.iter().filter(|s| s.board == b)));
    }
    let (liked_percentile, unliked_percentile) = (pct(true), pct(false));
    let layouts: Vec<Scored> = order.iter().map(|&i| scored[i].clone()).collect();
    Ok(LikesReport {
        geometry: g.name.clone(),
        angle_geometry: angle.map(|a| a.name.clone()),
        profile: profile.name.clone(),
        corpus: corpus.name.clone(),
        all: agreement(layouts.iter()),
        liked: agreement(layouts.iter().filter(|s| s.likes > 0)),
        boards,
        liked_percentile,
        unliked_percentile,
        metrics,
        layouts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmini(name: &str, board: &str, rows: [&str; 3]) -> String {
        let mut keys = serde_json::Map::new();
        for (r, row) in rows.iter().enumerate() {
            for (c, ch) in row.chars().enumerate() {
                let f = if board == "angle" && r == 2 { ANGLE_BOTTOM[c] } else { STANDARD[c] };
                keys.insert(ch.to_string(), serde_json::json!({"row": r, "col": c, "finger": f}));
            }
        }
        serde_json::json!({"name": name, "user": 1, "board": board, "keys": keys}).to_string()
    }

    #[test]
    fn parses_core_and_skips_what_does_not_fit() {
        let l = parse_cmini(&cmini("qwerty", "stagger", ["qwertyuiop", "asdfghjkl;", "zxcvbnm,./"]), 3, "x").unwrap();
        assert_eq!(l.rows, ["qwertyuiop", "asdfghjkl;", "zxcvbnm,./"]);
        assert_eq!(l.likes, 3);
        assert!(parse_cmini(&cmini("canary", "angle", ["wlypkzxou;", "crstbfneia", "jvdgqmh/,."]), 0, "").is_ok());
        let mut v: serde_json::Value = serde_json::from_str(&cmini("bad", "ortho", ["qwertyuiop", "asdfghjkl;", "zxcvbnm,./"])).unwrap();
        v["keys"]["q"]["col"] = 10.into();
        assert!(parse_cmini(&v.to_string(), 0, "").unwrap_err().reason.contains("off the 30-key core"));
        v["keys"]["q"] = serde_json::json!({"row": 0, "col": 0, "finger": "LI"});
        assert!(parse_cmini(&v.to_string(), 0, "").unwrap_err().reason.contains("fingering"));
        v["keys"].as_object_mut().unwrap().remove("q");
        assert!(parse_cmini(&v.to_string(), 0, "").unwrap_err().reason.contains("missing letters: q"));
        // A missing punctuation key leaves an empty slot.
        v["keys"].as_object_mut().unwrap().remove("/");
        v["keys"]["q"] = serde_json::json!({"row": 0, "col": 0, "finger": "LP"});
        assert_eq!(parse_cmini(&v.to_string(), 0, "").unwrap().rows[2], "zxcvbnm,.·");
    }

    #[test]
    fn duplicates_and_mirrors() {
        let mk = |name: &str, likes, rows: [&str; 3]| CommunityLayout {
            name: name.into(),
            author: String::new(),
            board: "ortho".into(),
            likes,
            rows: rows.iter().map(|s| s.to_string()).collect(),
            dup_of: None,
            mirror: false,
        };
        let q = ["qwertyuiop", "asdfghjkl;", "zxcvbnm,./"];
        let m = ["poiuytrewq", ";lkjhgfdsa", "/.,mnbvcxz"];
        let mut v = vec![
            mk("qwerty-copy", 0, q),
            mk("qwerty", 5, q),
            mk("ytrewq", 1, m),
            mk("other", 0, ["qwertyuiop", "asdfghjkl;", "zxcvbnm./,"]),
        ];
        assert_eq!(mark_duplicates(&mut v), 1);
        assert_eq!(v[0].dup_of.as_deref(), Some("qwerty"));
        assert!(!v[0].mirror);
        assert!(v[1].dup_of.is_none());
        assert_eq!(v[2].dup_of.as_deref(), Some("qwerty"));
        assert!(v[2].mirror);
        assert!(v[3].dup_of.is_none());
        let b = Bundle { schema: 1, source: "t".into(), layouts: v, skipped: vec![Skipped { name: "s".into(), reason: "r".into() }] };
        assert_eq!(Bundle::from_json(&b.to_json()).unwrap(), b);
        assert_eq!(b.unique().count(), 2);
    }
}
