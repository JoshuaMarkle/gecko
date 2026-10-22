//! Layout import from Oxeylyzer, genkey and cmini files: real-world samples parse to the
//! reference layouts, and export → import round-trips. See `context/cli.md`.

use gecko_core::interop::import::{self, Board, Format};
use gecko_core::layout::EMPTY;
use gecko_core::{Geometry, Layout};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn read(p: &str) -> String {
    std::fs::read_to_string(format!("{ROOT}/{p}")).unwrap()
}

fn reference(name: &str) -> Layout {
    Layout::from_toml(&read(&format!("data/layouts/{name}.toml"))).unwrap()
}

// Samples copied from the upstream repositories (O-X-E-Y/oxeylyzer-2, semilin/genkey, Apsu/cmini).
const STURDY_DOF: &str = r#"{
    "name": "Sturdy",
    "board": "iso",
    "layers": {
        "main": [
            " v m l c p  x f o u j  ",
            " s t r d y  . n a e i -",
            "z k q g w ? b h ' ; ,"
        ]
    },
    "fingering": "angle"
}"#;

const QWERTY_DOF: &str = r#"{
    "name": "Qwerty",
    "board": "ansi",
    "layers": {
        "main": [
            "q w e r t  y u i o p  ",
            "a s d f g  h j k l ; '",
            "z x c v b  n m , . /  ",
            " ~ ~   spc   ] ~ ~ ~  "
        ]
    },
    "fingering": "angle"
}"#;

const SEMIMAK_GENKEY: &str = "Semimak
f l h v z q w u o y
s r n t k c d e a i ;
x ' b m j p g , . /
0 1 2 3 3 4 4 5 6 7
0 1 2 3 3 4 4 5 6 7 7
0 1 2 3 3 4 4 5 6 7
";

fn cmini_json(name: &str, board: &str, rows: &[&str], extra: &str) -> String {
    let f = ["LP", "LR", "LM", "LI", "LI", "RI", "RI", "RM", "RR", "RP"];
    let mut keys = Vec::new();
    for (r, row) in rows.iter().enumerate() {
        for (c, ch) in row.chars().enumerate() {
            let k = serde_json::to_string(&ch.to_string()).unwrap();
            keys.push(format!(r#"{k}: {{"row": {r}, "col": {c}, "finger": "{}"}}"#, f[c.min(9)]));
        }
    }
    if !extra.is_empty() {
        keys.push(extra.to_string());
    }
    format!(r#"{{"name": "{name}", "user": 1, "board": "{board}", "keys": {{{}}}, "free": []}}"#, keys.join(", "))
}

#[test]
fn dof_iso_angle_matches_reference() {
    let l = import::import(Format::Oxeylyzer, STURDY_DOF, "sturdy").unwrap();
    assert_eq!(l.chars(), reference("sturdy").chars);
    assert_eq!(l.board, Board::Stagger);
    assert!(l.warnings.iter().any(|w| w.contains("angle")));
    assert!(l.warnings.iter().any(|w| w.contains("- ?")), "{:?}", l.warnings);
}

#[test]
fn dof_thumb_row_is_dropped() {
    let l = import::import(Format::Auto, QWERTY_DOF, "x").unwrap();
    assert_eq!(l.name, "Qwerty");
    assert_eq!(l.chars(), reference("qwerty").chars);
}

#[test]
fn dof_full_board_skips_number_row_and_modifiers() {
    let text = r#"{"name": "Full", "board": "ansi", "layers": {"main": [
        "` 1 2 3 4 5 6 7 8 9 0 - = bsp",
        "tab q w e r t y u i o p [ ] \\",
        "caps a s d f g h j k l ; ' ent",
        "shift z x c v b n m , . / shift"]}}"#;
    let l = import::import(Format::Oxeylyzer, text, "x").unwrap();
    assert_eq!(l.chars(), reference("qwerty").chars);
}

#[test]
fn oxeylyzer1_grid() {
    let text = "q w e r t y u i o p\na s d f g h j k l ;\nz x c v b n m , . /\n";
    let l = import::import(Format::Auto, text, "qwerty").unwrap();
    assert_eq!((l.name.as_str(), l.chars()), ("qwerty", reference("qwerty").chars));
}

#[test]
fn genkey_matches_reference() {
    assert_eq!(import::detect(SEMIMAK_GENKEY), Format::Genkey);
    let l = import::import(Format::Genkey, SEMIMAK_GENKEY, "x").unwrap();
    assert_eq!(l.name, "Semimak");
    assert_eq!(l.chars(), reference("semimak").chars);
    assert!(l.warnings.iter().any(|w| w.contains("dropped symbols") && w.ends_with(';')), "{:?}", l.warnings);
}

#[test]
fn cmini_matches_reference_and_rejects_thumb_letters() {
    let text = cmini_json("semimak", "ortho", &["flhvzqwuoy", "srntkcdeai;", "x'bmjpg,./"], "");
    assert_eq!(import::detect(&text), Format::Cmini);
    let l = import::import(Format::Auto, &text, "x").unwrap();
    assert_eq!(l.chars(), reference("semimak").chars);
    assert_eq!(l.board, Board::Ortho);
    // A letter on a thumb key would be silently unscored: refuse.
    let thumb = cmini_json("t", "stagger", &["flhvzqwuoy", "srntkcd1ai", "x'bmjpg,./"], r#""e": {"row": 3, "col": 0, "finger": "LT"}"#);
    let err = import::import(Format::Cmini, &thumb, "x").unwrap_err().to_string();
    assert!(err.contains("letter 'e'"), "{err}");
}

#[test]
fn short_rows_pad_with_empty_and_duplicates_fail() {
    let l = import::import(Format::Oxeylyzer, "q w e\na s d f g h j k l ;\nz x c v b n m , . /", "x").unwrap();
    assert_eq!(l.rows[0][3], EMPTY);
    assert!(import::import(Format::Oxeylyzer, "q w e r t y u i o p\nq s d f g h j k l ;\nz x c v b n m , . /", "x").is_err());
}

/// Every reference layout survives export → import in every format, on both boards.
#[test]
fn export_import_round_trips() {
    for gname in ["ansi-30", "ortho-30"] {
        let g = Geometry::from_toml(&read(&format!("data/geometries/{gname}.toml"))).unwrap();
        for entry in std::fs::read_dir(format!("{ROOT}/data/layouts")).unwrap() {
            let p = entry.unwrap().path();
            if p.extension().is_none_or(|e| e != "toml") {
                continue;
            }
            let lay = Layout::from_toml(&std::fs::read_to_string(&p).unwrap()).unwrap();
            for f in [Format::Oxeylyzer, Format::Genkey, Format::Cmini] {
                let text = import::export(f, &lay, &g).unwrap();
                assert_eq!(import::detect(&text), f, "{} {f:?}", lay.name);
                let back = import::import(Format::Auto, &text, "x").unwrap();
                assert_eq!(back.name, lay.name, "{f:?}");
                assert_eq!(back.chars(), lay.chars, "{} via {f:?} on {gname}", lay.name);
                if f != Format::Genkey {
                    // genkey files carry no board; they import as stagger.
                    assert_eq!(back.board.geometry(), gname, "{} via {f:?}", lay.name);
                }
                assert!(back.warnings.is_empty(), "{} via {f:?}: {:?}", lay.name, back.warnings);
                // The written layout file parses back to the same layout.
                let file = toml::to_string(&back.to_file(None)).unwrap();
                assert_eq!(Layout::from_toml(&file).unwrap().chars, lay.chars);
            }
        }
    }
}

#[test]
fn specials_round_trip_through_dof() {
    let g = Geometry::from_toml(&read("data/geometries/ansi-30.toml")).unwrap();
    let lay = Layout::from_str_flat("s", "ansi-30", "qwertyuiop asdfghjkl{space} z~*{none}bnm,./").unwrap();
    let text = import::export(Format::Oxeylyzer, &lay, &g).unwrap();
    assert_eq!(import::import(Format::Oxeylyzer, &text, "x").unwrap().chars(), lay.chars);
}
