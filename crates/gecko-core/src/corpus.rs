//! Text → n-gram count tables over a fixed charset. Binary cache format with header + hash.
//!
//! Tokenization: lowercase (an uppercase letter is its lowercase symbol typed with Shift);
//! a character outside the charset whose US-QWERTY unshifted key is in it (`:` → `;`, `!` →
//! `1`, …) is that key with Shift; whitespace collapses to one space token, which is the charset
//! symbol `␣` (`layout::SPACE`) when the charset has it; characters outside the charset are
//! "other" tokens. With `⏎` (`layout::ENTER`) in the charset, a newline is that symbol and
//! the indentation after it is dropped (editors indent), as are spaces before it; `\r` is
//! ignored. Bigrams and trigrams count only runs of charset symbols;
//! skipgrams with gap g (x, g tokens, z; g = 1..3) count when x and z are charset symbols,
//! whatever sits between.
//!
//! Shift (hold model): `shift[y]` counts Shift presses before symbol y, one per run of shifted
//! symbols; `shift_bi[x, y]` the subset typed right after symbol x (unshifted). Files store them
//! only when non-zero (`GKCORP3`), so corpora without case counts keep their bytes and hash.
//!
//! Word-frequency imports (`add_word` + `add_word_boundaries`) count every n-gram inside a word
//! exactly, including those with the space before or after it; letter n-grams never span
//! words, as with text. N-grams across a word boundary that need both words (`x ␣ z`,
//! skipgrams) are estimated by assuming consecutive words are independent (see
//! `add_word_boundaries`).

use crate::layout::{ARROWS, BACKSPACE, ENTER as ENTER_SYM, SPACE as SPACE_SYM};
use crate::rng::Rng;
use crate::{hash, Error, Result};

pub const DEFAULT_CHARSET: &str = "abcdefghijklmnopqrstuvwxyz,./;'-\u{2423}";
/// Charset for programming corpora: the default plus digits, the other ASCII punctuation and
/// Enter (`--charset code`).
pub const CODE_CHARSET: &str = "abcdefghijklmnopqrstuvwxyz,./;'-\u{2423}0123456789()[]{}<>=+*&|!?:\"_#$%^~`@\\\u{23CE}";

/// The code charset plus the arrow keys `←↓↑→` (`--charset nav`).
pub const NAV_CHARSET: &str =
    "abcdefghijklmnopqrstuvwxyz,./;'-\u{2423}0123456789()[]{}<>=+*&|!?:\"_#$%^~`@\\\u{23CE}\u{2190}\u{2193}\u{2191}\u{2192}";

/// A charset argument: `default`, `code`, `nav`, or the literal symbols.
pub fn charset_named(s: &str) -> &str {
    match s {
        "default" => DEFAULT_CHARSET,
        "code" => CODE_CHARSET,
        "nav" => NAV_CHARSET,
        _ => s,
    }
}
/// Fold Latin diacritics and ligatures to ASCII letters (`é` → `e`, `œ` → `oe`, `ß` → `ss`),
/// for non-English text on an ASCII charset: an accented letter then counts as its base key
/// (typed with a dead key or compose) instead of breaking the n-gram. Also repairs the
/// cp1252 apostrophe and `œ` that leak into some corpora as C1 controls (U+0092, U+009C).
/// Opt-in (`--fold`), so existing corpora rebuild byte-identically.
pub fn fold_diacritics(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if ch.is_ascii() {
            out.push(ch);
            continue;
        }
        let lower = ch.to_lowercase().next().unwrap_or(ch);
        let base = match lower {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => "a",
            'æ' => "ae",
            'ç' | 'ć' | 'č' => "c",
            'ď' | 'đ' => "d",
            'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ė' | 'ę' | 'ě' => "e",
            'ğ' => "g",
            'ì' | 'í' | 'î' | 'ï' | 'ī' | 'į' | 'ı' => "i",
            'ł' => "l",
            'ñ' | 'ń' | 'ň' => "n",
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ő' => "o",
            'œ' | '\u{9c}' => "oe",
            'ř' => "r",
            'ś' | 'š' | 'ş' | 'ș' => "s",
            'ß' => "ss",
            'ť' | 'ţ' | 'ț' => "t",
            'ù' | 'ú' | 'û' | 'ü' | 'ū' | 'ů' | 'ű' => "u",
            'ý' | 'ÿ' => "y",
            'ź' | 'ż' | 'ž' => "z",
            '\u{92}' => "'",
            _ => {
                out.push(ch);
                continue;
            }
        };
        out.push_str(base);
    }
    out
}

const MAGIC_V3: &[u8; 8] = b"GKCORP3\0";
const MAGIC: &[u8; 8] = b"GKCORP2\0";
const MAGIC_V1: &[u8; 8] = b"GKCORP1\0";
/// Word imports scale skipgram counts so cross-word estimates (fractions) survive rounding.
/// Harmless: every table is normalized on its own.
const SKIP_SCALE: f64 = 1000.0;

#[derive(Clone, Debug)]
pub struct Corpus {
    pub name: String,
    pub charset: Vec<char>,
    pub mono: Vec<u64>,
    pub bi: Vec<u64>,
    /// Skipgrams with gap 1 (x _ z).
    pub skip: Vec<u64>,
    /// Gap 2 (x _ _ z).
    pub skip2: Vec<u64>,
    /// Gap 3 (x _ _ _ z).
    pub skip3: Vec<u64>,
    pub tri: Vec<u64>,
    /// Shift presses before symbol y (start of a run of shifted symbols).
    pub shift: Vec<u64>,
    /// Shift presses between symbols x and y (`x` unshifted, `y` starts a shifted run).
    pub shift_bi: Vec<u64>,
}

/// US-QWERTY shifted characters and the key they are typed on.
pub const US_SHIFT: &[(char, char)] = &[
    ('!', '1'),
    ('@', '2'),
    ('#', '3'),
    ('$', '4'),
    ('%', '5'),
    ('^', '6'),
    ('&', '7'),
    ('*', '8'),
    ('(', '9'),
    (')', '0'),
    ('_', '-'),
    ('+', '='),
    ('{', '['),
    ('}', ']'),
    ('|', '\\'),
    (':', ';'),
    ('"', '\''),
    ('<', ','),
    ('>', '.'),
    ('?', '/'),
    ('~', '`'),
];

const SPACE: u8 = 254;
const OTHER: u8 = 255;

/// The tokenizer's per-character folding: lowercase, curly quotes → `'`, dashes → `-`.
pub fn fold_char(c: char) -> char {
    match c {
        '\u{2018}' | '\u{2019}' => '\'',
        '\u{2013}' | '\u{2014}' => '-',
        _ => c.to_ascii_lowercase(),
    }
}

/// Words kept in a `.words.tsv` side file (most frequent first). The tail carries almost no
/// frequency, so it never reaches a frequency × cost top list.
pub const WORDS_KEEP: usize = 20_000;

/// Word counts of a word-import corpus, stored beside it as `<name>.words.tsv` (`word\tcount`
/// per line, most frequent first). Not part of the `.gkc`, so corpus hashes are unchanged.
#[derive(Clone, Debug, Default)]
pub struct WordList {
    /// Folded words (see `fold_char`) with their counts, most frequent first.
    pub words: Vec<(String, u64)>,
    /// Occurrences of every imported word, including those cut by `WORDS_KEEP`.
    pub total: u64,
}

impl WordList {
    /// Fold and merge `(word, count)` pairs (`I` and `i` are one word), sort by count
    /// (ties alphabetical) and keep the `keep` most frequent.
    pub fn from_counts(counts: impl IntoIterator<Item = (String, u64)>, keep: usize) -> Self {
        let mut merged: std::collections::BTreeMap<String, u64> = Default::default();
        let mut total = 0;
        for (w, n) in counts {
            let w: String = w.trim().chars().map(fold_char).collect();
            if w.is_empty() || w.chars().any(char::is_whitespace) {
                continue;
            }
            total += n;
            *merged.entry(w).or_default() += n;
        }
        let mut words: Vec<(String, u64)> = merged.into_iter().collect();
        words.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        words.truncate(keep);
        WordList { words, total }
    }

    /// TSV with a `#total\t<n>` header line.
    pub fn to_tsv(&self) -> String {
        let mut s = format!("#total\t{}\n", self.total);
        for (w, n) in &self.words {
            s.push_str(&format!("{w}\t{n}\n"));
        }
        s
    }

    pub fn from_tsv(text: &str) -> Result<Self> {
        let mut l = WordList::default();
        for line in text.lines().filter(|l| !l.is_empty()) {
            let (w, n) = line.rsplit_once('\t').ok_or_else(|| Error::parse("words", format!("expected word<TAB>count: '{line}'")))?;
            let n: u64 = n.trim().parse().map_err(|e| Error::parse("words", format!("'{line}': {e}")))?;
            if w == "#total" {
                l.total = n;
            } else {
                l.words.push((w.to_string(), n));
            }
        }
        if l.total == 0 {
            l.total = l.words.iter().map(|w| w.1).sum();
        }
        Ok(l)
    }
}

/// Strip Project Gutenberg header/footer when present.
fn strip_gutenberg(text: &str) -> &str {
    let start = text.find("*** START OF").and_then(|i| text[i..].find('\n').map(|j| i + j + 1)).unwrap_or(0);
    let end = text[start..].find("*** END OF").map(|i| start + i).unwrap_or(text.len());
    &text[start..end]
}

/// Hard-wrapped text → one line per paragraph: lines join with a space, blank lines (one or
/// more) become one `\n`.
pub fn join_paragraphs(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut blank = false;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            blank = true;
            continue;
        }
        if !out.is_empty() {
            out.push(if blank { '\n' } else { ' ' });
        }
        out.push_str(line);
        blank = false;
    }
    out
}

/// Edge statistics of imported words, each counted as the segment `␣word` (a space token,
/// which is "other" without `␣` in the charset, then the word): `ends[i][x]` = occurrences of
/// segments whose token i from the end is x; `starts[j][z]` likewise from the start (token 0
/// is the space); `words` = occurrences.
#[derive(Clone, Debug)]
pub struct WordEdges {
    ends: [Vec<f64>; 4],
    starts: [Vec<f64>; 4],
    words: f64,
}

impl Corpus {
    pub fn new(name: &str, charset: &str) -> Self {
        let charset: Vec<char> = charset.chars().collect();
        let c = charset.len();
        Corpus {
            name: name.to_string(),
            charset,
            mono: vec![0; c],
            bi: vec![0; c * c],
            skip: vec![0; c * c],
            skip2: vec![0; c * c],
            skip3: vec![0; c * c],
            tri: vec![0; c * c * c],
            shift: vec![0; c],
            shift_bi: vec![0; c * c],
        }
    }

    pub fn size(&self) -> usize {
        self.charset.len()
    }

    pub fn index_of(&self, ch: char) -> Option<usize> {
        self.charset.iter().position(|&c| c == ch)
    }

    /// Token for whitespace: the `␣` symbol's index, else `SPACE` (not a symbol).
    fn space_token(&self) -> u8 {
        self.index_of(SPACE_SYM).map_or(SPACE, |i| i as u8)
    }

    /// Text → tokens (charset index, `SPACE` or `OTHER`), with whitespace runs collapsed, and
    /// whether each token is typed with Shift.
    fn tokens(&self, text: &str) -> (Vec<u8>, Vec<bool>) {
        let space = self.space_token();
        let enter = self.index_of(ENTER_SYM).map(|i| i as u8);
        let mut lut = [OTHER; 128];
        for (i, &ch) in self.charset.iter().enumerate() {
            if (ch as u32) < 128 {
                lut[ch as usize] = i as u8;
            }
        }
        let mut shifted_lut = [OTHER; 128];
        for &(s, base) in US_SHIFT {
            if lut[s as usize] == OTHER {
                shifted_lut[s as usize] = lut[base as usize];
            }
        }
        let mut out = Vec::with_capacity(text.len());
        let mut shift = Vec::with_capacity(text.len());
        let mut line_start = false;
        for raw in text.chars() {
            if let Some(e) = enter {
                if raw == '\r' {
                    continue;
                }
                if raw == '\n' {
                    if out.last() == Some(&space) {
                        out.pop();
                        shift.pop();
                    }
                    out.push(e);
                    shift.push(false);
                    line_start = true;
                    continue;
                }
                if line_start && raw.is_whitespace() {
                    continue;
                }
                line_start = false;
            }
            let ch = match raw {
                '\u{2018}' | '\u{2019}' => '\'',
                '\u{2013}' | '\u{2014}' => '-',
                _ => raw,
            };
            let (t, sh) = if ch.is_whitespace() {
                (space, false)
            } else if (ch as u32) < 128 {
                let lower = lut[ch.to_ascii_lowercase() as usize];
                if ch.is_ascii_uppercase() || lower != OTHER {
                    (lower, ch.is_ascii_uppercase() && lower != OTHER)
                } else {
                    let s = shifted_lut[ch as usize];
                    (s, s != OTHER)
                }
            } else {
                (self.index_of(ch).map_or(OTHER, |i| i as u8), false)
            };
            if t == space && out.last() == Some(&space) {
                continue;
            }
            out.push(t);
            shift.push(sh);
        }
        (out, shift)
    }

    /// Add every n-gram of a token sequence `w` times (skipgrams `skip_w` times); `sh` marks
    /// shifted tokens.
    fn count(&mut self, t: &[u8], sh: &[bool], w: u64, skip_w: u64) {
        let c = self.size();
        let sym = |i: usize| -> Option<usize> { t.get(i).map(|&x| x as usize).filter(|&x| x < c) };
        for i in 0..t.len() {
            let Some(x) = sym(i) else { continue };
            self.mono[x] += w;
            if sh[i] && (i == 0 || !sh[i - 1]) {
                self.shift[x] += w;
                if let Some(p) = i.checked_sub(1).and_then(sym) {
                    self.shift_bi[p * c + x] += w;
                }
            }
            if let Some(y) = sym(i + 1) {
                self.bi[x * c + y] += w;
                if let Some(z) = sym(i + 2) {
                    self.tri[(x * c + y) * c + z] += w;
                }
            }
            for (g, table) in [(1, &mut self.skip), (2, &mut self.skip2), (3, &mut self.skip3)] {
                if let Some(z) = sym(i + g + 1) {
                    table[x * c + z] += skip_w;
                }
            }
        }
    }

    /// Count n-grams of `text` into this corpus.
    pub fn add_text(&mut self, text: &str) {
        let (t, sh) = self.tokens(strip_gutenberg(text));
        self.count(&t, &sh, 1, 1);
    }

    /// Count hard-wrapped prose (`corpus build --paragraphs`): a single line break is a space
    /// (the source's wrapping, not a typed Enter) and a blank line one paragraph break, which is
    /// `⏎` when the charset has it.
    pub fn add_prose(&mut self, text: &str) {
        let (t, sh) = self.tokens(&join_paragraphs(strip_gutenberg(text)));
        self.count(&t, &sh, 1, 1);
    }

    pub fn word_edges(&self) -> WordEdges {
        let z = || vec![0.0; self.size()];
        WordEdges { ends: [z(), z(), z(), z()], starts: [z(), z(), z(), z()], words: 0.0 }
    }

    /// Count one word (no whitespace inside) that occurs `count` times, as the segment `␣word`
    /// plus, when `␣` is a symbol, the n-grams that run into the next segment's space
    /// (`x ␣`, `y x ␣`), which every word has whatever follows.
    pub fn add_word(&mut self, word: &str, count: u64, edges: &mut WordEdges) {
        let space = self.space_token();
        let (w, wsh) = self.tokens(word.trim());
        let (mut t, mut sh) = (vec![space], vec![false]);
        t.extend(w);
        sh.extend(wsh);
        if t.len() < 2 {
            return;
        }
        self.count(&t, &sh, count, count * SKIP_SCALE as u64);
        let c = self.size();
        let sym = |i: usize| t.get(i).map(|&x| x as usize).filter(|&x| x < c);
        let n = t.len();
        if (space as usize) < c {
            let sp = space as usize;
            if let Some(x) = sym(n - 1) {
                self.bi[x * c + sp] += count;
                if let Some(y) = sym(n - 2) {
                    self.tri[(y * c + x) * c + sp] += count;
                }
            }
        }
        for k in 0..4.min(n) {
            if let Some(x) = sym(n - 1 - k) {
                edges.ends[k][x] += count as f64;
            }
            if let Some(z) = sym(k) {
                edges.starts[k][z] += count as f64;
            }
        }
        edges.words += count as f64;
    }

    /// Add one precomputed 1-, 2- or 3-gram (no spaces) that occurs `count` times. Without word
    /// counts, skip-1 comes from the trigrams' outer symbols (inside words only) and skip-2/3
    /// stay empty; prefer `add_word` when the source has word frequencies. Shift: a shifted
    /// 1-gram counts one press, a 2-gram `x Y` a press after x.
    pub fn add_ngram(&mut self, gram: &str, count: u64) {
        let (t, sh) = self.tokens(gram);
        let c = self.size();
        if t.iter().any(|&x| x as usize >= c) {
            // Mirrors text: a non-charset symbol breaks the n-gram, but a trigram's ends still form a skipgram.
            if let [x, _, z] = t[..] {
                if (x as usize) < c && (z as usize) < c {
                    self.skip[x as usize * c + z as usize] += count;
                }
            }
            return;
        }
        match t[..] {
            [x] => {
                self.mono[x as usize] += count;
                if sh[0] {
                    self.shift[x as usize] += count;
                }
            }
            [x, y] => {
                self.bi[x as usize * c + y as usize] += count;
                if sh[1] && !sh[0] {
                    self.shift_bi[x as usize * c + y as usize] += count;
                }
            }
            [x, y, z] => {
                self.tri[(x as usize * c + y as usize) * c + z as usize] += count;
                self.skip[x as usize * c + z as usize] += count;
            }
            _ => {}
        }
    }

    /// Estimate n-grams that need two consecutive `␣word` segments, assuming consecutive words
    /// are independent: x at offset i from the end of one segment and z at offset j from the
    /// start of the next are i + j tokens apart. With `␣` a symbol that adds the trigrams
    /// `x ␣ z`; skipgrams always. Spans across three or more segments are ignored.
    pub fn add_word_boundaries(&mut self, e: &WordEdges) {
        if e.words <= 0.0 {
            return;
        }
        let c = self.size();
        let sp = self.space_token() as usize;
        if sp < c {
            // x ␣ z: the next segment starts `␣ z`. (`y x ␣` is exact, counted in `add_word`.)
            for x in 0..c {
                for z in 0..c {
                    self.tri[(x * c + sp) * c + z] += (e.ends[0][x] * e.starts[1][z] / e.words).round() as u64;
                }
            }
        }
        for (g, table) in [(1, &mut self.skip), (2, &mut self.skip2), (3, &mut self.skip3)] {
            for i in 0..=g {
                let j = g - i;
                for x in 0..c {
                    for z in 0..c {
                        let v = e.ends[i][x] * e.starts[j][z] / e.words * SKIP_SCALE;
                        table[x * c + z] += v.round() as u64;
                    }
                }
            }
        }
    }

    pub fn total_chars(&self) -> u64 {
        self.mono.iter().sum()
    }

    /// The corpus has Shift counts (written as `GKCORP3`).
    pub fn has_shift(&self) -> bool {
        self.shift.iter().any(|&v| v > 0)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let v3 = self.has_shift();
        out.extend_from_slice(if v3 { MAGIC_V3 } else { MAGIC });
        let name = self.name.as_bytes();
        let cs: String = self.charset.iter().collect();
        for s in [name, cs.as_bytes()] {
            out.extend_from_slice(&(s.len() as u32).to_le_bytes());
            out.extend_from_slice(s);
        }
        let shift = if v3 { [&self.shift, &self.shift_bi] } else { [&vec![], &vec![]] };
        for t in [&self.mono, &self.bi, &self.skip, &self.skip2, &self.skip3, &self.tri].into_iter().chain(shift) {
            for &v in t.iter() {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        out
    }

    pub fn from_bytes(b: &[u8]) -> Result<Self> {
        let bad = || Error::parse("corpus", "truncated or corrupt corpus file");
        let magic = b.get(..8).unwrap_or_default();
        let (v1, v3) = (magic == MAGIC_V1, magic == MAGIC_V3);
        if magic != MAGIC && !v1 && !v3 {
            return Err(Error::parse("corpus", "not a GECKO corpus file (bad magic)"));
        }
        let mut i = 8;
        let take_str = |i: &mut usize| -> Result<String> {
            let n = u32::from_le_bytes(b.get(*i..*i + 4).ok_or_else(bad)?.try_into().unwrap()) as usize;
            *i += 4;
            let s = std::str::from_utf8(b.get(*i..*i + n).ok_or_else(bad)?).map_err(|e| Error::parse("corpus", e))?;
            *i += n;
            Ok(s.to_string())
        };
        let name = take_str(&mut i)?;
        let cs = take_str(&mut i)?;
        let mut c = Corpus::new(&name, &cs);
        // GKCORP1 has no skip2/skip3; before GKCORP3 no Shift counts. Missing tables stay zero.
        let tables = if v1 {
            vec![&mut c.mono, &mut c.bi, &mut c.skip, &mut c.tri]
        } else if v3 {
            vec![&mut c.mono, &mut c.bi, &mut c.skip, &mut c.skip2, &mut c.skip3, &mut c.tri, &mut c.shift, &mut c.shift_bi]
        } else {
            vec![&mut c.mono, &mut c.bi, &mut c.skip, &mut c.skip2, &mut c.skip3, &mut c.tri]
        };
        for t in tables {
            for v in t.iter_mut() {
                *v = u64::from_le_bytes(b.get(i..i + 8).ok_or_else(bad)?.try_into().unwrap());
                i += 8;
            }
        }
        Ok(c)
    }

    pub fn hash(&self) -> u64 {
        hash::fnv1a(&self.to_bytes())
    }

    /// This corpus over `charset ∪ extra` (extra symbols appended, zero counts).
    pub fn extended(&self, name: &str, extra: &[char]) -> Result<Self> {
        let mut cs = self.charset.clone();
        cs.extend(extra.iter().filter(|c| !self.charset.contains(c)));
        if cs.len() >= SPACE as usize {
            return Err(Error::invalid("corpus", "too many symbols"));
        }
        let (c, n) = (self.size(), cs.len());
        let mut out = Corpus::new(name, &cs.iter().collect::<String>());
        out.mono[..c].copy_from_slice(&self.mono);
        out.shift[..c].copy_from_slice(&self.shift);
        for x in 0..c {
            for y in 0..c {
                out.bi[x * n + y] = self.bi[x * c + y];
                out.shift_bi[x * n + y] = self.shift_bi[x * c + y];
                out.skip[x * n + y] = self.skip[x * c + y];
                out.skip2[x * n + y] = self.skip2[x * c + y];
                out.skip3[x * n + y] = self.skip3[x * c + y];
                for z in 0..c {
                    out.tri[(x * n + y) * n + z] = self.tri[(x * c + y) * c + z];
                }
            }
        }
        Ok(out)
    }

    /// This corpus plus arrow keys `←↓↑→` making up `share` of all keystrokes. Text corpora
    /// have no cursor movement, so the arrows come from a model (seeded, deterministic): a
    /// stream typed from this corpus' bigram chain where, before each keystroke, a run of
    /// arrows starts with probability 0.1; runs continue with probability 2/3 (mean length 3)
    /// and keep their direction with probability 0.85; directions ← 25 %, ↓ 30 %, ↑ 20 %,
    /// → 25 %. After a run the next symbol is drawn from the monograms (the cursor moved). The
    /// result mixes this corpus (weight 1 − w) with that stream (w = share / arrow share of
    /// the stream), so all non-arrow statistics stay mostly exact.
    pub fn with_nav(&self, name: &str, share: f64, seed: u64) -> Result<Self> {
        const LEN: usize = 2_000_000;
        const START: f64 = 0.1;
        const CONT: f64 = 2.0 / 3.0;
        const SAME: f64 = 0.85;
        const DIR: [f64; 4] = [0.25, 0.30, 0.20, 0.25];
        let base = self.extended(name, &ARROWS)?;
        let arrow: Vec<u8> = ARROWS.iter().map(|&a| base.index_of(a).unwrap() as u8).collect();
        let c = self.size();
        // Cumulative rows for sampling: row c is the monograms.
        let mut cum = vec![0u64; (c + 1) * c];
        for x in 0..=c {
            let mut acc = 0;
            for y in 0..c {
                acc += if x == c { self.mono[y] } else { self.bi[x * c + y] };
                cum[x * c + y] = acc;
            }
        }
        if cum[c * c + c - 1] == 0 {
            return Err(Error::invalid("corpus", "empty corpus"));
        }
        let mut rng = Rng::new(seed);
        let draw = |rng: &mut Rng, row: usize| -> Option<u8> {
            let r = &cum[row * c..row * c + c];
            let tot = r[c - 1];
            (tot > 0).then(|| r.partition_point(|&v| v <= (rng.f64() * tot as f64) as u64).min(c - 1) as u8)
        };
        let pick_dir = |rng: &mut Rng, not: Option<usize>| -> usize {
            loop {
                let mut u = rng.f64();
                let d = DIR
                    .iter()
                    .position(|&p| {
                        u -= p;
                        u < 0.0
                    })
                    .unwrap_or(3);
                if Some(d) != not {
                    return d;
                }
            }
        };
        let mut t: Vec<u8> = Vec::with_capacity(LEN + 64);
        let mut prev: Option<u8> = None;
        while t.len() < LEN {
            if rng.f64() < START {
                let mut d = pick_dir(&mut rng, None);
                loop {
                    t.push(arrow[d]);
                    if rng.f64() >= CONT {
                        break;
                    }
                    if rng.f64() >= SAME {
                        d = pick_dir(&mut rng, Some(d));
                    }
                }
                prev = None;
                continue;
            }
            let x = prev.and_then(|p| draw(&mut rng, p as usize)).or_else(|| draw(&mut rng, c)).unwrap();
            t.push(x);
            prev = Some(x);
        }
        let mut stream = Corpus::new(name, &base.charset.iter().collect::<String>());
        stream.count(&t, &vec![false; t.len()], 1, 1);
        let a = arrow.iter().map(|&i| stream.mono[i as usize]).sum::<u64>() as f64 / stream.total_chars().max(1) as f64;
        if !(share > 0.0 && share < a) {
            return Err(Error::invalid("corpus nav", format!("share must be in (0, {a:.2})")));
        }
        let w = share / a;
        Corpus::mix(name, &[(&base, 1.0 - w), (&stream, w)])
    }

    /// This corpus as typed on keys that carry Shift pairs `(base, shifted)`: every n-gram of a
    /// shifted symbol moves to its base key (added to the base's counts; the base joins the
    /// charset in the shifted symbol's place when missing) and Shift presses are added for it.
    /// A press is counted before each shifted symbol except right after another symbol of a
    /// pair (`::` holds Shift): `mono[s] − Σ bi[t, s]` over pair-shifted t, of which `bi[x, s]`
    /// with x not pair-shifted are presses after x. Estimates where case counts meet pairs: a
    /// capital right after a pair-shifted symbol still counts its own press. Pairs whose
    /// shifted symbol is not in the charset change nothing.
    pub fn keyed(&self, pairs: &[(char, char)]) -> Result<Self> {
        let pair_of = |ch: char| pairs.iter().find(|p| p.1 == ch);
        if !self.charset.iter().any(|&ch| pair_of(ch).is_some()) {
            return Ok(self.clone());
        }
        let mut cs: Vec<char> = Vec::new();
        for &ch in &self.charset {
            let k = pair_of(ch).map_or(ch, |p| p.0);
            if !cs.contains(&k) && (k == ch || !self.charset.contains(&k)) {
                cs.push(k);
            }
        }
        let c = self.size();
        let n = cs.len();
        let map: Vec<usize> =
            self.charset.iter().map(|&ch| cs.iter().position(|&k| k == pair_of(ch).map_or(ch, |p| p.0)).unwrap()).collect();
        let sh: Vec<bool> = self.charset.iter().map(|&ch| pair_of(ch).is_some()).collect();
        let mut out = Corpus::new(&self.name, &cs.iter().collect::<String>());
        for x in 0..c {
            let mx = map[x];
            out.mono[mx] += self.mono[x];
            out.shift[mx] += self.shift[x];
            for y in 0..c {
                let (xy, my) = (x * c + y, mx * n + map[y]);
                out.bi[my] += self.bi[xy];
                out.skip[my] += self.skip[xy];
                out.skip2[my] += self.skip2[xy];
                out.skip3[my] += self.skip3[xy];
                out.shift_bi[my] += self.shift_bi[xy];
                for z in 0..c {
                    out.tri[my * n + map[z]] += self.tri[xy * c + z];
                }
            }
        }
        for y in (0..c).filter(|&y| sh[y]) {
            let held: u64 = (0..c).filter(|&x| sh[x]).map(|x| self.bi[x * c + y]).sum();
            out.shift[map[y]] += self.mono[y].saturating_sub(held);
            for x in (0..c).filter(|&x| !sh[x]) {
                out.shift_bi[map[x] * n + map[y]] += self.bi[x * c + y];
            }
        }
        Ok(out)
    }

    /// This corpus plus Backspace (`⌫`) making up `share` of all keystrokes, from a typing-error
    /// model (seeded, deterministic): a stream typed from the bigram chain where, before each
    /// keystroke, an error starts with probability 0.1; the typist types k wrong symbols (the
    /// first drawn from the monograms, the rest continuing the chain), presses `⌫` k times and
    /// retypes from where the error began. k − 1 is geometric with continuation 0.45 (mean k
    /// 1.82). The stream is mixed in like [`Corpus::with_nav`] at the weight that gives `share`,
    /// so other statistics stay mostly exact. Fitted on 4,000 participants of the Aalto 136M
    /// keystrokes study (2.8M keys): `⌫` is 5.6 % of keystrokes, 3.0 runs per 100 keys, runs of
    /// 1/2/3 keys 55/25/10 %.
    pub fn with_edits(&self, name: &str, share: f64, seed: u64) -> Result<Self> {
        const LEN: usize = 2_000_000;
        const START: f64 = 0.1;
        const CONT: f64 = 0.45;
        let base = self.extended(name, &[BACKSPACE])?;
        let bs = base.index_of(BACKSPACE).unwrap() as u8;
        let c = self.size();
        let mut cum = vec![0u64; (c + 1) * c];
        for x in 0..=c {
            let mut acc = 0;
            for y in 0..c {
                acc += if x == c { self.mono[y] } else { self.bi[x * c + y] };
                cum[x * c + y] = acc;
            }
        }
        if cum[c * c + c - 1] == 0 {
            return Err(Error::invalid("corpus", "empty corpus"));
        }
        let mut rng = Rng::new(seed);
        let draw = |rng: &mut Rng, row: usize| -> Option<u8> {
            let r = &cum[row * c..row * c + c];
            let tot = r[c - 1];
            (tot > 0).then(|| r.partition_point(|&v| v <= (rng.f64() * tot as f64) as u64).min(c - 1) as u8)
        };
        let next = |rng: &mut Rng, prev: Option<u8>| prev.and_then(|p| draw(rng, p as usize)).or_else(|| draw(rng, c)).unwrap();
        let mut t: Vec<u8> = Vec::with_capacity(LEN + 64);
        let mut prev: Option<u8> = None;
        while t.len() < LEN {
            if rng.f64() < START {
                let mut k = 1;
                let mut wrong = draw(&mut rng, c).unwrap();
                t.push(wrong);
                while rng.f64() < CONT {
                    wrong = next(&mut rng, Some(wrong));
                    t.push(wrong);
                    k += 1;
                }
                t.extend(std::iter::repeat_n(bs, k));
                continue;
            }
            let x = next(&mut rng, prev);
            t.push(x);
            prev = Some(x);
        }
        let mut stream = Corpus::new(name, &base.charset.iter().collect::<String>());
        stream.count(&t, &vec![false; t.len()], 1, 1);
        let a = stream.mono[bs as usize] as f64 / stream.total_chars().max(1) as f64;
        if !(share > 0.0 && share < a) {
            return Err(Error::invalid("corpus edits", format!("share must be in (0, {a:.2})")));
        }
        let w = share / a;
        Corpus::mix(name, &[(&base, 1.0 - w), (&stream, w)])
    }

    /// Weighted mix of corpora with the same charset (each normalized first; Shift counts
    /// scale with mono, so presses per keystroke are kept).
    pub fn mix(name: &str, parts: &[(&Corpus, f64)]) -> Result<Self> {
        let first = parts.first().ok_or_else(|| Error::invalid("corpus mix", "empty"))?.0;
        let cs: String = first.charset.iter().collect();
        let mut out = Corpus::new(name, &cs);
        const SCALE: f64 = 1e12;
        for (c, w) in parts {
            if c.charset != first.charset {
                return Err(Error::invalid("corpus mix", "charsets differ"));
            }
            for (dst, src) in [
                (&mut out.mono, &c.mono),
                (&mut out.bi, &c.bi),
                (&mut out.skip, &c.skip),
                (&mut out.skip2, &c.skip2),
                (&mut out.skip3, &c.skip3),
                (&mut out.tri, &c.tri),
            ] {
                let tot: u64 = src.iter().sum::<u64>().max(1);
                for (d, &s) in dst.iter_mut().zip(src.iter()) {
                    *d += (s as f64 / tot as f64 * w * SCALE) as u64;
                }
            }
            let tot = c.mono.iter().sum::<u64>().max(1) as f64;
            for (dst, src) in [(&mut out.shift, &c.shift), (&mut out.shift_bi, &c.shift_bi)] {
                for (d, &s) in dst.iter_mut().zip(src.iter()) {
                    *d += (s as f64 / tot * w * SCALE) as u64;
                }
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
#[allow(clippy::identity_op, clippy::erasing_op)]
mod tests {
    use super::*;

    #[test]
    fn nav_adds_arrows_at_share() {
        let mut c = Corpus::new("t", "abc\u{2423}");
        c.add_text("abc cab bca abba cc a b c");
        let e = c.extended("e", &ARROWS).unwrap();
        assert_eq!(e.size(), 8);
        let (ia, ib) = (e.index_of('a').unwrap(), e.index_of('b').unwrap());
        assert_eq!(e.bi[ia * 8 + ib], c.bi[0 * 4 + 1]);
        let n = c.with_nav("n", 0.05, 1).unwrap();
        let arrows: u64 = ARROWS.iter().map(|&a| n.mono[n.index_of(a).unwrap()]).sum();
        let share = arrows as f64 / n.total_chars() as f64;
        assert!((share - 0.05).abs() < 1e-4, "{share}");
        let (l, r) = (n.index_of(ARROWS[0]).unwrap(), n.index_of(ARROWS[3]).unwrap());
        assert!(n.bi[r * 8 + r] > n.bi[r * 8 + l], "runs keep their direction");
        assert!(c.with_nav("n", 0.9, 1).is_err());
        let mut t = Corpus::new("t", NAV_CHARSET);
        t.add_text("a\u{2192}\u{2192}b");
        assert_eq!(t.bi[t.index_of(ARROWS[3]).unwrap() * t.size() + t.index_of('b').unwrap()], 1);
    }

    #[test]
    fn counts_and_round_trip() {
        let mut c = Corpus::new("t", "abc");
        c.add_text("abc ab  Ca\n");
        // tokens: a b c _ a b _ c a
        assert_eq!(c.mono, vec![3, 2, 2]);
        assert_eq!(c.bi[0 * 3 + 1], 2); // ab
        assert_eq!(c.bi[1 * 3 + 2], 1); // bc
        assert_eq!(c.bi[2 * 3 + 0], 1); // ca
        assert_eq!(c.tri[(0 * 3 + 1) * 3 + 2], 1); // abc
        assert_eq!(c.skip[0 * 3 + 2], 1); // a_c from abc
        assert_eq!(c.skip[2 * 3 + 0], 1); // c _ a
        assert_eq!(c.skip2[1 * 3 + 0], 2); // b..a twice
        assert_eq!(c.skip2[2 * 3 + 1], 1); // c..b
        assert_eq!(c.skip2[0 * 3 + 2], 1); // a.._c
        assert_eq!(c.skip3[0 * 3 + 0], 2); // a...a twice
        assert_eq!(c.skip3[1 * 3 + 1], 1); // b...b
        assert_eq!(c.skip3.iter().sum::<u64>(), 3);
        let back = Corpus::from_bytes(&c.to_bytes()).unwrap();
        assert_eq!(back.tri, c.tri);
        assert_eq!(back.skip3, c.skip3);
        assert_eq!(back.name, "t");
    }

    #[test]
    fn reads_v1_files() {
        let mut c = Corpus::new("t", "abc");
        c.add_text("abc ab ca");
        // A GKCORP1 file is the v2 layout without skip2/skip3.
        let v2 = c.to_bytes();
        let n = 3 * 3 * 8;
        let head = v2.len() - (3 + 9 * 4 + 27) * 8;
        let mut v1 = MAGIC_V1.to_vec();
        v1.extend_from_slice(&v2[8..head + (3 + 9 + 9) * 8]);
        v1.extend_from_slice(&v2[head + (3 + 9 + 9) * 8 + 2 * n..]);
        let back = Corpus::from_bytes(&v1).unwrap();
        assert_eq!(back.skip, c.skip);
        assert_eq!(back.tri, c.tri);
        assert!(back.skip2.iter().all(|&v| v == 0));
    }

    #[test]
    fn shift_counts_runs_and_round_trips() {
        let mut c = Corpus::new("t", "ab;\u{2423}");
        // tokens: A b ␣ B A ␣ a : b — runs start at A, B (after ␣) and : (folded onto ;).
        c.add_text("Ab BA a: b");
        assert_eq!(c.mono, vec![3, 3, 1, 3]);
        assert_eq!(c.shift, vec![1, 1, 1, 0]);
        assert_eq!(c.shift_bi[3 * 4 + 1], 1); // ␣ B
        assert_eq!(c.shift_bi[0 * 4 + 2], 1); // a :
        assert_eq!(c.shift_bi.iter().sum::<u64>(), 2);
        assert_eq!(c.bi[0 * 4 + 2], 1); // a; (the ':' key)
        let b = c.to_bytes();
        assert_eq!(&b[..8], MAGIC_V3);
        let back = Corpus::from_bytes(&b).unwrap();
        assert_eq!((back.shift, back.shift_bi), (c.shift.clone(), c.shift_bi.clone()));
        // Without case counts the file stays GKCORP2 (same bytes and hash as before).
        let mut l = Corpus::new("t", "ab;\u{2423}");
        l.add_text("ab ba");
        assert_eq!(&l.to_bytes()[..8], MAGIC);
        // Mixing keeps presses per keystroke.
        let m = Corpus::mix("m", &[(&c, 1.0), (&l, 1.0)]).unwrap();
        let per_key = |c: &Corpus| c.shift.iter().sum::<u64>() as f64 / c.mono.iter().sum::<u64>() as f64;
        assert!((per_key(&m) - per_key(&c) / 2.0).abs() < 1e-9);
    }

    #[test]
    fn words_count_inside_and_estimate_across() {
        let mut c = Corpus::new("t", "abc");
        let mut e = c.word_edges();
        c.add_word("ab", 2, &mut e);
        c.add_word("Ca", 1, &mut e);
        c.add_word_boundaries(&e);
        assert_eq!(c.mono, vec![3, 2, 1]);
        assert_eq!(c.bi[0 * 3 + 1], 2);
        assert_eq!(c.bi[1 * 3 + 0], 0); // never across words
                                        // b ends 2 of 3 words, a starts 2 of 3: 3 · (2/3) · (2/3) = 4/3, scaled by 1000.
        assert_eq!(c.skip[1 * 3 + 0], 1333);
        assert_eq!(c.skip[0 * 3 + 2], 333);
    }

    #[test]
    fn folds_diacritics_to_ascii() {
        assert_eq!(fold_diacritics("Été à l\u{92}œuvre, Straße"), "ete a l'oeuvre, Strasse");
        assert_eq!(fold_diacritics("naïve 😂"), "naive 😂");
        let mut c = Corpus::new("t", "et");
        c.add_text(&fold_diacritics("été"));
        assert_eq!(c.bi[0 * 2 + 1], 1); // et, not broken by é
        assert_eq!(c.mono, vec![2, 1]);
    }

    #[test]
    fn word_list_folds_merges_and_round_trips() {
        let l =
            WordList::from_counts([("I".into(), 3), ("i".into(), 2), ("you\u{2019}re".into(), 4), ("a b".into(), 9), ("zz".into(), 1)], 2);
        assert_eq!(l.words, vec![("i".to_string(), 5), ("you're".to_string(), 4)]);
        assert_eq!(l.total, 10);
        let back = WordList::from_tsv(&l.to_tsv()).unwrap();
        assert_eq!(back.words, l.words);
        assert_eq!(back.total, 10);
    }

    #[test]
    fn enter_ends_lines_and_drops_indentation() {
        let mut c = Corpus::new("t", "ab\u{2423}\u{23CE}");
        c.add_text("a b \r\n    b\n\na");
        // tokens: a ␣ b ⏎ b ⏎ ⏎ a
        assert_eq!(c.mono, vec![2, 2, 1, 3]);
        assert_eq!(c.bi[1 * 4 + 3], 2); // b⏎ twice
        assert_eq!(c.bi[3 * 4 + 1], 1); // ⏎b
        assert_eq!(c.bi[3 * 4 + 3], 1); // ⏎⏎
        assert_eq!(c.bi[1 * 4 + 2], 0); // no trailing space
    }

    #[test]
    fn space_is_a_symbol_when_in_charset() {
        let mut c = Corpus::new("t", "ab\u{2423}");
        c.add_text("ab  b\na");
        // tokens: a b ␣ b ␣ a
        assert_eq!(c.mono, vec![2, 2, 2]);
        assert_eq!(c.bi[1 * 3 + 2], 2); // b␣
        assert_eq!(c.tri[(1 * 3 + 2) * 3 + 1], 1); // b␣b
        assert_eq!(c.skip[2 * 3 + 2], 1); // ␣ b ␣

        // Words: every n-gram touching one word's spaces is exact; x ␣ z is estimated.
        let mut w = Corpus::new("w", "ab\u{2423}");
        let mut e = w.word_edges();
        w.add_word("ab", 3, &mut e);
        w.add_word("b", 1, &mut e);
        w.add_word_boundaries(&e);
        assert_eq!(w.mono, vec![3, 4, 4]);
        assert_eq!(w.bi[2 * 3 + 0], 3); // ␣a
        assert_eq!(w.bi[1 * 3 + 2], 4); // b␣
        assert_eq!(w.tri[(0 * 3 + 1) * 3 + 2], 3); // ab␣
        assert_eq!(w.tri[(2 * 3 + 1) * 3 + 2], 1); // ␣b␣
        assert_eq!(w.tri[(1 * 3 + 2) * 3 + 0], 3); // b␣a: 4 · 3/4
                                                   // ␣ _ ␣ around "b": exact, as the segment end meets the next segment's space.
        assert_eq!(w.skip[2 * 3 + 2], 1000);
    }
}
