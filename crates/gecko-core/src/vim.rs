//! Vim keystrokes: a synthetic editing-session model and an importer for real `nvim -w` /
//! `vim -w` keystroke logs. Text corpora have no editor commands, so a Vim user's motions,
//! operators, `⎋` and `:` commands come from here. See `context/vim.md`.
//!
//! The model is a weighted grammar of normal-mode commands (templates below; weights are
//! assumptions informed by two published single-user keystroke logs, not a population
//! study). Inserts type short text from a text corpus' bigram chain, then `⎋`. Ctrl chords
//! (`C-d`, `C-r`, `C-w`…) count as their letter key; Ctrl itself is not in corpora.

use crate::corpus::{Chain, CODE_CHARSET};
use crate::layout::{BACKSPACE, ENTER, ESC, SPACE, TAB};
use crate::rng::Rng;
use crate::{Corpus, Error, Result};

/// Charset of Vim corpora: the code charset plus Esc.
pub fn charset() -> String {
    format!("{CODE_CHARSET}{ESC}")
}

/// Charset of imported logs: the Vim charset plus Tab, Backspace and the arrows.
pub fn log_charset() -> String {
    format!("{}{TAB}{BACKSPACE}{}", charset(), crate::layout::ARROWS.iter().collect::<String>())
}

/// Normal-mode commands per command (not per key) with their relative weights. Placeholders:
/// `{n}` optional count, `{c}` a text character, `{a}` a register/mark letter, `{m}` a motion
/// after an operator, `{o}` a text object, `{t}` inserted text then `⎋`, `{p}` a search
/// pattern, `{f}` a file name, `{d}` a line number, `{v}` visual motions then an operator,
/// `{l}` a leader mapping, `⏎` Enter. Ctrl chords are written as their letter.
pub const COMMANDS: &[(f64, &str)] = &[
    // Motions (≈ 60 %).
    (14.0, "{n}j"),
    (12.0, "{n}k"),
    (4.0, "{n}h"),
    (5.0, "{n}l"),
    (8.0, "{n}w"),
    (5.0, "{n}b"),
    (3.0, "{n}e"),
    (0.7, "W"),
    (0.5, "B"),
    (0.3, "E"),
    (1.0, "0"),
    (0.7, "^"),
    (1.5, "$"),
    (1.0, "gg"),
    (1.2, "G"),
    (2.5, "f{c}"),
    (1.0, "t{c}"),
    (0.7, "F{c}"),
    (0.3, "T{c}"),
    (1.0, ";"),
    (0.4, ","),
    (1.0, "}"),
    (0.8, "{"),
    (0.4, "%"),
    (1.0, "d"), // C-d
    (0.8, "u"), // C-u
    (0.4, "o"), // C-o
    (2.5, "n"),
    (0.8, "N"),
    (0.8, "*"),
    (0.2, "#"),
    (2.0, "/{p}⏎"),
    (0.3, "?{p}⏎"),
    (0.5, "zz"),
    (0.1, "zt"),
    (0.3, "H"),
    (0.3, "L"),
    (0.2, "m{a}"),
    (0.2, "'{a}"),
    (0.1, "`{a}"),
    (0.3, "gd"),
    // Operators and edits (≈ 22 %).
    (2.0, "dd"),
    (1.5, "d{m}"),
    (0.4, "D"),
    (0.6, "d{o}"),
    (1.2, "c{m}{t}"),
    (1.0, "c{o}{t}"),
    (0.3, "cc{t}"),
    (0.3, "C{t}"),
    (1.2, "yy"),
    (0.6, "y{m}"),
    (0.5, "y{o}"),
    (2.5, "p"),
    (0.6, "P"),
    (2.5, "x"),
    (0.2, "X"),
    (0.8, "r{c}"),
    (0.2, "~"),
    (0.4, ">>"),
    (0.2, "<<"),
    (0.3, "=ap"),
    (0.3, "gcc"),
    (0.4, "J"),
    (2.5, "."),
    (3.0, "u"),
    (0.8, "r"), // C-r
    (0.1, "\"+y{m}"),
    (0.1, "\"+p"),
    (0.1, "@{a}"),
    // Visual mode.
    (1.2, "v{v}"),
    (1.0, "V{v}"),
    (0.3, "v{v}"), // C-v
    // Insert (≈ 12 %).
    (3.5, "i{t}"),
    (2.0, "a{t}"),
    (2.5, "o{t}"),
    (1.0, "O{t}"),
    (1.5, "A{t}"),
    (0.5, "I{t}"),
    (0.3, "s{t}"),
    (0.2, "S{t}"),
    // Ex commands and mappings.
    (3.0, ":w⏎"),
    (0.6, ":q⏎"),
    (0.5, ":wq⏎"),
    (0.2, ":q!⏎"),
    (0.4, ":%s/{p}/{p}/g⏎"),
    (0.4, ":e {f}⏎"),
    (0.5, ":{d}⏎"),
    (0.3, ":noh⏎"),
    (0.1, ":vs⏎"),
    (3.0, " {l}"),
    (0.5, "w{w}"), // C-w + direction
];

/// Motions after an operator (`{m}`).
const MOTIONS: &[(f64, &str)] = &[
    (30.0, "w"),
    (8.0, "e"),
    (6.0, "b"),
    (8.0, "$"),
    (6.0, "j"),
    (3.0, "k"),
    (3.0, "}"),
    (5.0, "f{c}"),
    (6.0, "t{c}"),
    (1.0, "G"),
    (1.0, "gg"),
    (2.0, "l"),
    (1.0, "h"),
    (1.0, "^"),
    (1.0, "0"),
    (3.0, "W"),
    (1.0, "/{p}⏎"),
];

const OBJECTS: &[(f64, &str)] = &[
    (10.0, "iw"),
    (3.0, "aw"),
    (4.0, "i\""),
    (3.0, "i("),
    (2.0, "ip"),
    (2.0, "ap"),
    (2.0, "i{"),
    (1.0, "it"),
    (1.0, "a("),
    (1.0, "i'"),
    (1.0, "i["),
];

const VISUAL_MOTIONS: &[(f64, &str)] =
    &[(10.0, "j"), (4.0, "k"), (4.0, "w"), (3.0, "e"), (3.0, "$"), (2.0, "}"), (2.0, "iw"), (1.0, "ip"), (1.0, "G")];
const VISUAL_OPS: &[(f64, &str)] =
    &[(5.0, "d"), (6.0, "y"), (3.0, "c{t}"), (2.0, ">"), (1.0, "<"), (2.0, "p"), (1.0, "gc"), (1.0, ":s/{p}/{p}/⏎"), (1.0, "⎋")];
const LEADER: &[(f64, &str)] =
    &[(4.0, "ff"), (3.0, "fg"), (2.0, "fb"), (2.0, "w"), (1.0, "e"), (1.0, "q"), (1.0, "ca"), (1.0, "rn"), (1.0, "gs"), (1.0, "h")];

fn pick<'a>(rng: &mut Rng, table: &'a [(f64, &'a str)]) -> &'a str {
    let tot: f64 = table.iter().map(|t| t.0).sum();
    let mut u = rng.f64() * tot;
    for (w, s) in table {
        u -= w;
        if u < 0.0 {
            return s;
        }
    }
    table.last().unwrap().1
}

/// Generator state: the text chain inserts and patterns are typed from.
struct Gen<'a> {
    text: &'a Corpus,
    chain: Chain,
    /// Share of each text symbol typed with Shift (capitals).
    upper: Vec<f64>,
}

impl Gen<'_> {
    fn sym(&self, i: u8) -> char {
        match self.text.charset[i as usize] {
            SPACE => ' ',
            c => c,
        }
    }

    fn typed(&self, rng: &mut Rng, i: u8) -> char {
        let c = self.sym(i);
        if c.is_ascii_lowercase() && rng.f64() < self.upper[i as usize] {
            c.to_ascii_uppercase()
        } else {
            c
        }
    }

    /// Text from the chain: `len` symbols, starting after a space.
    fn text(&self, rng: &mut Rng, len: usize, out: &mut String) {
        let sp = self.text.index_of(SPACE);
        let mut prev: Option<u8> = None;
        let mut last = sp.map_or_else(|| self.chain.mono(rng), |s| s as u8);
        for _ in 0..len {
            let x = self.chain.after(rng, prev, last);
            out.push(self.typed(rng, x));
            (prev, last) = (Some(last), x);
        }
    }

    /// A letter or punctuation symbol from the text (targets of `f`, `t`, `r`).
    fn char(&self, rng: &mut Rng) -> char {
        loop {
            let c = self.sym(self.chain.mono(rng));
            if c != ' ' && c != ENTER {
                return c;
            }
        }
    }

    fn word(&self, rng: &mut Rng, out: &mut String) {
        let n = 2 + rng.below(5);
        let mut s = String::new();
        while s.len() < n {
            let c = self.char(rng);
            if c.is_ascii_alphabetic() {
                s.push(c);
            }
        }
        out.push_str(&s);
    }

    fn expand(&self, rng: &mut Rng, t: &str, out: &mut String) {
        let mut chars = t.chars();
        while let Some(ch) = chars.next() {
            if ch != '{' || t.len() == 1 || chars.as_str().is_empty() || !chars.as_str().contains('}') {
                out.push(ch);
                continue;
            }
            let name: String = chars.by_ref().take_while(|&c| c != '}').collect();
            match name.as_str() {
                "n" => {
                    if rng.f64() < 0.15 {
                        let span = if rng.f64() < 0.8 { 8 } else { 19 };
                        let k = 2 + rng.below(span);
                        out.push_str(&k.to_string());
                    }
                }
                "c" => out.push(self.char(rng)),
                "a" => out.push((b'a' + rng.below(26) as u8) as char),
                "m" => {
                    let t = pick(rng, MOTIONS);
                    self.expand(rng, t, out)
                }
                "o" => out.push_str(pick(rng, OBJECTS)),
                "t" => {
                    // Geometric length, mean 12; the inserted text ends with ⎋.
                    let mut len = 1;
                    while rng.f64() < 11.0 / 12.0 {
                        len += 1;
                    }
                    self.text(rng, len, out);
                    out.push(ESC);
                }
                "p" => self.word(rng, out),
                "f" => {
                    self.word(rng, out);
                    out.push('.');
                    out.push_str(["rs", "py", "ts", "md", "lua", "toml"][rng.below(6)]);
                }
                "d" => out.push_str(&(1 + rng.below(400)).to_string()),
                "v" => {
                    if rng.f64() < 0.2 {
                        out.push_str(&(2 + rng.below(8)).to_string());
                    }
                    for _ in 0..1 + rng.below(3) {
                        {
                            let t = pick(rng, VISUAL_MOTIONS);
                            self.expand(rng, t, out)
                        };
                    }
                    {
                        let t = pick(rng, VISUAL_OPS);
                        self.expand(rng, t, out)
                    };
                }
                "l" => out.push_str(pick(rng, LEADER)),
                "w" => out.push(['h', 'j', 'k', 'l', 'w'][rng.below(5)]),
                _ => {
                    out.push('{');
                    out.push_str(&name);
                    out.push('}');
                }
            }
        }
    }
}

/// A synthetic Vim editing session of about `keystrokes` keys: commands drawn from
/// [`COMMANDS`], inserts typed from `text` (a corpus with letters and `␣`; capitals at its
/// Shift rate). Seeded, deterministic.
pub fn synthetic(name: &str, text: &Corpus, keystrokes: usize, seed: u64) -> Result<Corpus> {
    let chain = Chain::new(text).ok_or_else(|| Error::invalid("corpus vim", "empty text corpus"))?;
    let upper = (0..text.size()).map(|i| text.shift[i] as f64 / text.mono[i].max(1) as f64).collect();
    let g = Gen { text, chain, upper };
    let mut rng = Rng::new(seed);
    let mut s = String::with_capacity(keystrokes + 64);
    let mut count = 0;
    while count < keystrokes {
        let before = s.len();
        let t = pick(&mut rng, COMMANDS);
        g.expand(&mut rng, t, &mut s);
        count += s[before..].chars().count();
    }
    let mut c = Corpus::new(name, &charset());
    c.add_text(&s);
    Ok(c)
}

/// One `-w` log (`nvim -w <file>`): raw keys as typed, in every mode. Esc is `⎋`, CR `⏎`,
/// Tab `⇥`, Backspace (`^H`, `<80>kb`) `⌫`, `<80>ku/kd/kl/kr` the arrows, a Ctrl chord its
/// letter; other special keys (`<80>` sequences, mouse, focus events) break the n-grams.
pub fn log_text(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        i += 1;
        match b {
            0x80 => {
                let code = bytes.get(i..i + 2).unwrap_or_default();
                i += 2;
                out.push(match code {
                    b"kb" => BACKSPACE,
                    b"ku" => crate::layout::UP,
                    b"kd" => crate::layout::DOWN,
                    b"kl" => crate::layout::LEFT,
                    b"kr" => crate::layout::RIGHT,
                    _ => '\u{FFFD}',
                });
            }
            0x1b => out.push(ESC),
            0x0d | 0x0a => out.push(ENTER),
            0x09 => out.push(TAB),
            0x08 | 0x7f => out.push(BACKSPACE),
            0x01..=0x1a => out.push((b + 0x60) as char),
            0x20..=0x7e => out.push(b as char),
            _ => out.push('\u{FFFD}'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_session_has_commands_and_escape() {
        let mut t = Corpus::new("t", crate::corpus::DEFAULT_CHARSET);
        t.add_text("The quick brown fox jumps over the lazy dog. A cat sat on the mat, then left.");
        let v = synthetic("vim", &t, 20_000, 1).unwrap();
        let share = |c: char| v.mono[v.index_of(c).unwrap()] as f64 / v.total_chars() as f64;
        assert!(share('j') > share('q') && share('k') > 0.03, "j {} k {}", share('j'), share('k'));
        assert!(share(ESC) > 0.005 && share(':') > 0.003);
        assert!(v.has_shift(), "G, A, O… are shifted");
        assert_eq!(synthetic("vim", &t, 5000, 7).unwrap().hash(), synthetic("vim", &t, 5000, 7).unwrap().hash());
    }

    #[test]
    fn log_bytes_map_to_keys() {
        assert_eq!(log_text(b"ihi\x1b:w\r\x80kbjj\x04"), format!("ihi{ESC}:w{ENTER}{BACKSPACE}jjd"));
        let mut c = Corpus::new("l", &log_charset());
        c.add_text(&log_text(b"dd\x1bp\x80ku"));
        assert_eq!(c.mono[c.index_of(ESC).unwrap()], 1);
        assert_eq!(c.mono[c.index_of(crate::layout::UP).unwrap()], 1);
    }
}
