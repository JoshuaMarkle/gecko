//! Layouts from other analyzers' files, and back (for round-trip tests and sharing).
//!
//! Formats ([`Format`]):
//! - **oxeylyzer**: Oxeylyzer 2 / libdof `.dof` JSON (`layers.main` rows of space-separated
//!   keys, `board`, `fingering`), or the plain-text grid of Oxeylyzer 1 (`.kb`: three rows of
//!   space-separated keys, name from the file stem).
//! - **genkey**: name line, three key rows, then three finger rows (`0`–`7`, optional).
//! - **cmini**: JSON `{name, board, keys: {char: {row, col, finger}}}`.
//!
//! Every format maps onto the 30 alpha keys by physical position: rows 0–2, columns 0–9.
//! Extra columns, number and thumb rows are dropped with a warning; dropping a letter is an
//! error (the layout would be scored without it). Fingering (angle mod, custom finger rows) is
//! not imported: keys keep their position and the target geometry's fingers, with a warning.

use serde_json::{json, Map, Value};

use crate::layout::{LayoutFile, BACKSPACE, EMPTY, ENTER, ESC, SPACE, TAB};
use crate::{Error, Finger, Geometry, Layout, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// Guess from the text ([`detect`]).
    Auto,
    Oxeylyzer,
    Genkey,
    Cmini,
}

impl Format {
    pub fn parse(s: &str) -> Result<Self> {
        Ok(match s.to_ascii_lowercase().as_str() {
            "auto" => Format::Auto,
            "oxeylyzer" | "oxey" | "dof" | "kb" => Format::Oxeylyzer,
            "genkey" => Format::Genkey,
            "cmini" => Format::Cmini,
            _ => return Err(Error::invalid("format", format!("'{s}' (expected oxeylyzer, genkey, cmini or auto)"))),
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Format::Auto => "auto",
            Format::Oxeylyzer => "oxeylyzer",
            Format::Genkey => "genkey",
            Format::Cmini => "cmini",
        }
    }
}

/// Row stagger of the source board; picks the default target geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Board {
    Stagger,
    Ortho,
}

impl Board {
    pub fn geometry(self) -> &'static str {
        match self {
            Board::Stagger => "ansi-30",
            Board::Ortho => "ortho-30",
        }
    }
}

/// A parsed layout: the 30 alpha keys plus what did not fit.
#[derive(Clone, Debug, PartialEq)]
pub struct Imported {
    pub name: String,
    pub board: Board,
    pub source: String,
    /// `rows[row][col]`, [`EMPTY`] where the source has no symbol.
    pub rows: [[char; 10]; 3],
    pub warnings: Vec<String>,
}

impl Imported {
    /// The layout file (geometry from the source board unless `geometry` is given).
    pub fn to_file(&self, geometry: Option<&str>) -> LayoutFile {
        LayoutFile {
            name: self.name.clone(),
            geometry: geometry.unwrap_or(self.board.geometry()).to_string(),
            source: self.source.clone(),
            rows: self.rows.iter().map(|r| r.iter().collect()).collect(),
            layer: Vec::new(),
            magic: Vec::new(),
            shifted: Vec::new(),
        }
    }

    pub fn chars(&self) -> Vec<char> {
        self.rows.concat()
    }
}

/// Traditional finger of each alpha column.
const FINGERS: [Finger; 10] =
    [Finger::LP, Finger::LR, Finger::LM, Finger::LI, Finger::LI, Finger::RI, Finger::RI, Finger::RM, Finger::RR, Finger::RP];
const GENKEY_FINGERS: &str = "0 1 2 3 3 4 4 5 6 7";

/// Guess the format: JSON with `keys` is cmini, other JSON is `.dof`; text whose rows 5–7 are
/// finger digits is genkey; anything else an Oxeylyzer 1 grid.
pub fn detect(text: &str) -> Format {
    let t = text.trim_start();
    if t.starts_with('{') {
        return match serde_json::from_str::<Value>(t) {
            Ok(v) if v.get("keys").is_some() => Format::Cmini,
            _ => Format::Oxeylyzer,
        };
    }
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let digits = |l: &&str| l.split_whitespace().all(|t| t.len() == 1 && t.as_bytes()[0].is_ascii_digit());
    if lines.len() >= 7 && lines[4..7].iter().all(digits) {
        Format::Genkey
    } else {
        Format::Oxeylyzer
    }
}

/// Parse `text` in `format`; `stem` (the file name without extension) names layouts whose
/// format has no name field.
pub fn import(format: Format, text: &str, stem: &str) -> Result<Imported> {
    match format {
        Format::Auto => import(detect(text), text, stem),
        Format::Oxeylyzer if text.trim_start().starts_with('{') => dof(text, stem),
        Format::Oxeylyzer => kb(text, stem),
        Format::Genkey => genkey(text),
        Format::Cmini => cmini(text, stem),
    }
}

/// Collects the 30 alpha keys and the symbols that did not fit.
struct Grid {
    rows: [[char; 10]; 3],
    dropped: Vec<char>,
    warnings: Vec<String>,
}

impl Grid {
    fn new() -> Self {
        Grid { rows: [[EMPTY; 10]; 3], dropped: Vec::new(), warnings: Vec::new() }
    }

    /// Place one row of keys: the first 10 on the alpha keys, the rest dropped.
    fn row(&mut self, r: usize, keys: &[char]) {
        if keys.len() < 10 {
            self.warnings.push(format!("row {} has {} keys; the rest are empty", r + 1, keys.len()));
        }
        for (c, &k) in keys.iter().enumerate() {
            if c < 10 {
                self.rows[r][c] = k;
            } else {
                self.dropped.push(k);
            }
        }
    }

    fn finish(mut self, name: String, board: Board, source: String) -> Result<Imported> {
        self.dropped.retain(|&c| c != EMPTY);
        if let Some(c) = self.dropped.iter().find(|c| c.is_alphabetic()) {
            return Err(Error::invalid(
                "layout",
                format!("'{name}' puts letter '{c}' outside the 30 alpha keys (thumb, number row or extra column); not supported"),
            ));
        }
        if !self.dropped.is_empty() {
            let s: Vec<String> = self.dropped.iter().map(|c| c.to_string()).collect();
            self.warnings.push(format!("dropped symbols outside the 30 alpha keys: {}", s.join(" ")));
        }
        let out = Imported { name, board, source, rows: self.rows, warnings: self.warnings };
        Layout::from_chars(&out.name, out.board.geometry(), "", out.chars())?; // duplicate check
        Ok(out)
    }
}

/// One `.dof` key: `~` empty, `*` transparent, `\x` the escaped char, named specials.
fn dof_key(tok: &str, warnings: &mut Vec<String>) -> char {
    let mut cs = tok.chars();
    match (cs.next(), cs.next(), cs.next()) {
        (Some('~' | '*'), None, _) => EMPTY,
        (Some(c), None, _) => c,
        (Some('\\'), Some(c), None) => c,
        _ => match tok.to_ascii_lowercase().as_str() {
            "spc" | "space" => SPACE,
            "ent" | "enter" | "ret" | "rt" | "return" => ENTER,
            "bsp" | "bksp" | "backspace" => BACKSPACE,
            "tb" | "tab" => TAB,
            "esc" | "escape" => ESC,
            _ => {
                warnings.push(format!("key '{tok}' has no GECKO equivalent; left empty"));
                EMPTY
            }
        },
    }
}

fn dof_token(c: char) -> String {
    match c {
        EMPTY => "~".into(),
        SPACE => "spc".into(),
        ENTER => "ent".into(),
        BACKSPACE => "bsp".into(),
        TAB => "tab".into(),
        ESC => "esc".into(),
        '~' | '*' => format!("\\{c}"),
        c => c.to_string(),
    }
}

fn dof(text: &str, stem: &str) -> Result<Imported> {
    let v: Value = serde_json::from_str(text).map_err(|e| Error::parse("dof layout", e))?;
    let name = v["name"].as_str().unwrap_or(stem).to_string();
    let board_name = v["board"].as_str().unwrap_or("").to_ascii_lowercase();
    let board = if matches!(board_name.as_str(), "ortho" | "colstag" | "ortholinear") { Board::Ortho } else { Board::Stagger };
    let mut g = Grid::new();
    let angle = match &v["fingering"] {
        Value::String(f) if f.eq_ignore_ascii_case("angle") => {
            g.warnings.push("source fingering 'angle' not imported; keys keep their position".into());
            true
        }
        Value::String(f) if !matches!(f.to_ascii_lowercase().as_str(), "traditional" | "standard") => {
            g.warnings.push(format!("source fingering '{f}' not imported; keys keep their position"));
            false
        }
        Value::Object(_) | Value::Array(_) => {
            g.warnings.push("custom source fingering not imported; keys keep their position".into());
            false
        }
        _ => false,
    };
    let main = v["layers"]["main"].as_array().ok_or_else(|| Error::invalid("dof layout", format!("'{name}' has no layers.main")))?;
    let mut rows: Vec<Vec<&str>> = main.iter().map(|r| r.as_str().unwrap_or("").split_whitespace().collect()).collect();
    // A full board: skip the number row and the modifier columns left of the alpha keys.
    if rows.first().is_some_and(|r| r.iter().filter(|t| t.len() == 1 && t.as_bytes()[0].is_ascii_digit()).count() >= 3) {
        g.dropped.extend(rows.remove(0).iter().map(|t| dof_key(t, &mut Vec::new())));
    }
    for r in rows.iter_mut() {
        while r.first().is_some_and(|t| t.chars().count() > 1 && !t.starts_with('\\')) {
            r.remove(0);
        }
    }
    for (ri, r) in rows.iter().enumerate() {
        let mut keys: Vec<char> = r.iter().map(|t| dof_key(t, &mut g.warnings)).collect();
        if ri >= 3 {
            g.dropped.extend(keys); // thumb row
            continue;
        }
        // ISO bottom row has a 6th left key: the ISO key takes the pinky column (dropped
        // under traditional fingering; under angle mod the key at B's position is dropped).
        if ri == 2 && board_name == "iso" && keys.len() >= 11 {
            g.dropped.push(keys.remove(if angle { 5 } else { 0 }));
        }
        g.row(ri, &keys);
    }
    let mut who: Vec<String> = v["authors"].as_array().into_iter().flatten().filter_map(Value::as_str).map(String::from).collect();
    who.extend(v["year"].as_u64().map(|y| y.to_string()));
    let source =
        if who.is_empty() { "imported from Oxeylyzer .dof".into() } else { format!("{} (imported from Oxeylyzer .dof)", who.join(", ")) };
    g.finish(name, board, source)
}

/// Keys of a plain-text row: space-separated, or one char per key when not separated.
fn text_keys(line: &str) -> Vec<char> {
    let toks: Vec<&str> = line.split_whitespace().collect();
    if toks.len() == 1 {
        return toks[0].chars().collect();
    }
    toks.iter().flat_map(|t| t.chars()).collect()
}

fn kb(text: &str, stem: &str) -> Result<Imported> {
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.len() < 3 {
        return Err(Error::invalid("layout", format!("'{stem}': expected 3 rows of keys, found {}", lines.len())));
    }
    let mut g = Grid::new();
    for (r, l) in lines.iter().enumerate() {
        let keys = text_keys(l);
        if r < 3 {
            g.row(r, &keys);
        } else {
            g.dropped.extend(keys);
        }
    }
    g.finish(stem.to_string(), Board::Stagger, "imported from Oxeylyzer 1".into())
}

fn genkey(text: &str) -> Result<Imported> {
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.len() < 4 {
        return Err(Error::invalid("genkey layout", "expected a name line and 3 rows of keys"));
    }
    let name = lines[0].trim().to_string();
    let mut g = Grid::new();
    for r in 0..3 {
        g.row(r, &text_keys(lines[1 + r]));
    }
    if lines.len() >= 7 && (4..7).any(|i| !lines[i].split_whitespace().take(10).eq(GENKEY_FINGERS.split_whitespace())) {
        g.warnings.push("genkey finger map differs from traditional (angle mod?); not imported, keys keep their position".into());
    }
    g.finish(name, Board::Stagger, "imported from genkey".into())
}

fn cmini(text: &str, stem: &str) -> Result<Imported> {
    let v: Value = serde_json::from_str(text).map_err(|e| Error::parse("cmini layout", e))?;
    let name = v["name"].as_str().unwrap_or(stem).to_string();
    let board_name = v["board"].as_str().unwrap_or("");
    let board = if board_name == "ortho" { Board::Ortho } else { Board::Stagger };
    let keys = v["keys"].as_object().ok_or_else(|| Error::invalid("cmini layout", format!("'{name}' has no keys")))?;
    let mut g = Grid::new();
    let mut filled = [[false; 10]; 3];
    let mut odd_finger = board_name == "angle";
    for (k, pos) in keys {
        let mut cs = k.chars();
        let c = match (cs.next(), cs.next()) {
            (Some(c), None) => c,
            _ => {
                g.warnings.push(format!("key '{k}' is not one symbol; skipped"));
                continue;
            }
        };
        let (row, col) = (pos["row"].as_i64().unwrap_or(-1), pos["col"].as_i64().unwrap_or(-1));
        if !(0..3).contains(&row) || !(0..10).contains(&col) {
            g.dropped.push(c);
            continue;
        }
        let (r, cl) = (row as usize, col as usize);
        if filled[r][cl] {
            return Err(Error::invalid("cmini layout", format!("'{name}': two symbols at row {row} col {col}")));
        }
        filled[r][cl] = true;
        g.rows[r][cl] = c;
        if pos["finger"].as_str().is_some_and(|f| f != format!("{:?}", FINGERS[cl])) {
            odd_finger = true;
        }
    }
    g.dropped.sort_unstable();
    if odd_finger {
        g.warnings.push("cmini fingering differs from traditional (angle mod?); not imported, keys keep their position".into());
    }
    g.finish(name, board, "imported from cmini".into())
}

/// The 30 alpha keys of `layout` on `g` (rows 0–2, columns 0–9) and the board's stagger.
fn alpha_grid(layout: &Layout, g: &Geometry) -> Result<(Board, [[char; 10]; 3])> {
    layout.check_geometry(g)?;
    let mut rows = [[None; 10]; 3];
    let mut x0 = [None; 3];
    for (k, &c) in g.key.iter().zip(&layout.chars) {
        if (0..3).contains(&k.row) && (0..10).contains(&k.col) {
            rows[k.row as usize][k.col as usize] = Some(c);
            if k.col == 0 {
                x0[k.row as usize] = Some(k.pos[0]);
            }
        }
    }
    let mut out = [[EMPTY; 10]; 3];
    for r in 0..3 {
        for c in 0..10 {
            out[r][c] = rows[r][c].ok_or_else(|| Error::invalid("geometry", format!("'{}' has no key at row {r} col {c}", g.name)))?;
        }
    }
    let ortho = x0.iter().all(|&x| x.is_some() && x == x0[0]);
    Ok((if ortho { Board::Ortho } else { Board::Stagger }, out))
}

/// `layout`'s 30 alpha keys written in `format` (not [`Format::Auto`]). Oxeylyzer output is
/// `.dof` JSON.
pub fn export(format: Format, layout: &Layout, g: &Geometry) -> Result<String> {
    let (board, rows) = alpha_grid(layout, g)?;
    let half = |r: &[char; 10], tok: &dyn Fn(char) -> String| {
        let t: Vec<String> = r.iter().map(|&c| tok(c)).collect();
        format!("{}  {}", t[..5].join(" "), t[5..].join(" "))
    };
    Ok(match format {
        Format::Auto => return Err(Error::invalid("format", "export needs a concrete format")),
        Format::Oxeylyzer => {
            let main: Vec<String> = rows.iter().map(|r| half(r, &dof_token)).collect();
            let board = if board == Board::Ortho { "ortho" } else { "ansi" };
            let v = json!({"name": layout.name, "board": board, "layers": {"main": main}, "fingering": "traditional"});
            serde_json::to_string_pretty(&v).unwrap()
        }
        Format::Genkey => {
            let mut s = format!("{}\n", layout.name);
            for r in &rows {
                s += &half(r, &|c| c.to_string());
                s.push('\n');
            }
            for _ in 0..3 {
                s += GENKEY_FINGERS;
                s.push('\n');
            }
            s
        }
        Format::Cmini => {
            let mut keys = Map::new();
            for (r, row) in rows.iter().enumerate() {
                for (c, &ch) in row.iter().enumerate().filter(|(_, &ch)| ch != EMPTY) {
                    keys.insert(ch.to_string(), json!({"row": r, "col": c, "finger": format!("{:?}", FINGERS[c])}));
                }
            }
            let board = if board == Board::Ortho { "ortho" } else { "stagger" };
            serde_json::to_string_pretty(&json!({"name": layout.name, "board": board, "keys": keys})).unwrap()
        }
    })
}
