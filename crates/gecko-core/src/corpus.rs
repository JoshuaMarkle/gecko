//! Text → n-gram count tables over a fixed charset. Binary cache format with header + hash.
//!
//! Tokenization: lowercase; whitespace collapses to one space token, which is the charset
//! symbol `␣` (`layout::SPACE`) when the charset has it; characters outside the charset are
//! "other" tokens. With `⏎` (`layout::ENTER`) in the charset, a newline is that symbol and
//! the indentation after it is dropped (editors indent), as are spaces before it; `\r` is
//! ignored. Bigrams and trigrams count only runs of charset symbols;
//! skipgrams with gap g (x, g tokens, z; g = 1..3) count when x and z are charset symbols,
//! whatever sits between.
//!
//! Word-frequency imports (`add_word` + `add_word_boundaries`) count every n-gram inside a word
//! exactly, including those with the space before or after it; letter n-grams never span
//! words, as with text. N-grams across a word boundary that need both words (`x ␣ z`,
//! skipgrams) are estimated by assuming consecutive words are independent (see
//! `add_word_boundaries`).

use crate::layout::{ENTER as ENTER_SYM, SPACE as SPACE_SYM};
use crate::{hash, Error, Result};

pub const DEFAULT_CHARSET: &str = "abcdefghijklmnopqrstuvwxyz,./;'-\u{2423}";
/// Charset for programming corpora: the default plus digits, the other ASCII punctuation and
/// Enter (`--charset code`).
pub const CODE_CHARSET: &str = "abcdefghijklmnopqrstuvwxyz,./;'-\u{2423}0123456789()[]{}<>=+*&|!?:\"_#$%^~`@\\\u{23CE}";

/// A charset argument: `default`, `code`, or the literal symbols.
pub fn charset_named(s: &str) -> &str {
    match s {
        "default" => DEFAULT_CHARSET,
        "code" => CODE_CHARSET,
        _ => s,
    }
}
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
}

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

    /// Text → tokens (charset index, `SPACE` or `OTHER`), with whitespace runs collapsed.
    fn tokens(&self, text: &str) -> Vec<u8> {
        let space = self.space_token();
        let enter = self.index_of(ENTER_SYM).map(|i| i as u8);
        let mut lut = [OTHER; 128];
        for (i, &ch) in self.charset.iter().enumerate() {
            if (ch as u32) < 128 {
                lut[ch as usize] = i as u8;
            }
        }
        let mut out = Vec::with_capacity(text.len());
        let mut line_start = false;
        for raw in text.chars() {
            if let Some(e) = enter {
                if raw == '\r' {
                    continue;
                }
                if raw == '\n' {
                    if out.last() == Some(&space) {
                        out.pop();
                    }
                    out.push(e);
                    line_start = true;
                    continue;
                }
                if line_start && raw.is_whitespace() {
                    continue;
                }
                line_start = false;
            }
            let ch = fold_char(raw);
            let t = if ch.is_whitespace() {
                space
            } else if (ch as u32) < 128 {
                lut[ch as usize]
            } else {
                OTHER
            };
            if t == space && out.last() == Some(&space) {
                continue;
            }
            out.push(t);
        }
        out
    }

    /// Add every n-gram of a token sequence `w` times (skipgrams `skip_w` times).
    fn count(&mut self, t: &[u8], w: u64, skip_w: u64) {
        let c = self.size();
        let sym = |i: usize| -> Option<usize> { t.get(i).map(|&x| x as usize).filter(|&x| x < c) };
        for i in 0..t.len() {
            let Some(x) = sym(i) else { continue };
            self.mono[x] += w;
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
        let t = self.tokens(strip_gutenberg(text));
        self.count(&t, 1, 1);
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
        let mut t = vec![space];
        t.extend(self.tokens(word.trim()));
        if t.len() < 2 {
            return;
        }
        self.count(&t, count, count * SKIP_SCALE as u64);
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
    /// stay empty; prefer `add_word` when the source has word frequencies.
    pub fn add_ngram(&mut self, gram: &str, count: u64) {
        let t = self.tokens(gram);
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
            [x] => self.mono[x as usize] += count,
            [x, y] => self.bi[x as usize * c + y as usize] += count,
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

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(MAGIC);
        let name = self.name.as_bytes();
        let cs: String = self.charset.iter().collect();
        for s in [name, cs.as_bytes()] {
            out.extend_from_slice(&(s.len() as u32).to_le_bytes());
            out.extend_from_slice(s);
        }
        for t in [&self.mono, &self.bi, &self.skip, &self.skip2, &self.skip3, &self.tri] {
            for &v in t.iter() {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        out
    }

    pub fn from_bytes(b: &[u8]) -> Result<Self> {
        let bad = || Error::parse("corpus", "truncated or corrupt corpus file");
        let v1 = b.len() >= 8 && &b[..8] == MAGIC_V1;
        if b.len() < 8 || (&b[..8] != MAGIC && !v1) {
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
        // GKCORP1 has no skip2/skip3; they stay zero.
        let tables = if v1 {
            vec![&mut c.mono, &mut c.bi, &mut c.skip, &mut c.tri]
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

    /// Weighted mix of corpora with the same charset (each normalized first).
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
        }
        Ok(out)
    }
}

#[cfg(test)]
#[allow(clippy::identity_op, clippy::erasing_op)]
mod tests {
    use super::*;

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
