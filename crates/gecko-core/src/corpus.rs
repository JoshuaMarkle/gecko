//! Text → n-gram count tables over a fixed charset. Binary cache format with header + hash.
//!
//! Tokenization: lowercase; whitespace collapses to one space token; characters outside the
//! charset are "other" tokens. Bigrams and trigrams count only runs of charset symbols;
//! skipgrams (x _ z) count when x and z are charset symbols, whatever sits between.

use crate::{hash, Error, Result};

pub const DEFAULT_CHARSET: &str = "abcdefghijklmnopqrstuvwxyz,./;'-";
const MAGIC: &[u8; 8] = b"GKCORP1\0";

#[derive(Clone, Debug)]
pub struct Corpus {
    pub name: String,
    pub charset: Vec<char>,
    pub mono: Vec<u64>,
    pub bi: Vec<u64>,
    pub skip: Vec<u64>,
    pub tri: Vec<u64>,
}

const SPACE: u8 = 254;
const OTHER: u8 = 255;

/// Strip Project Gutenberg header/footer when present.
fn strip_gutenberg(text: &str) -> &str {
    let start = text.find("*** START OF").and_then(|i| text[i..].find('\n').map(|j| i + j + 1)).unwrap_or(0);
    let end = text[start..].find("*** END OF").map(|i| start + i).unwrap_or(text.len());
    &text[start..end]
}

impl Corpus {
    pub fn new(name: &str, charset: &str) -> Self {
        let charset: Vec<char> = charset.chars().collect();
        let c = charset.len();
        Corpus { name: name.to_string(), charset, mono: vec![0; c], bi: vec![0; c * c], skip: vec![0; c * c], tri: vec![0; c * c * c] }
    }

    pub fn size(&self) -> usize {
        self.charset.len()
    }

    pub fn index_of(&self, ch: char) -> Option<usize> {
        self.charset.iter().position(|&c| c == ch)
    }

    /// Count n-grams of `text` into this corpus.
    pub fn add_text(&mut self, text: &str) {
        let text = strip_gutenberg(text);
        let c = self.size();
        let mut lut = [OTHER; 128];
        for (i, &ch) in self.charset.iter().enumerate() {
            if (ch as u32) < 128 {
                lut[ch as usize] = i as u8;
            }
        }
        let mut p2 = OTHER;
        let mut p1 = OTHER;
        for raw in text.chars() {
            let ch = match raw {
                '\u{2018}' | '\u{2019}' => '\'',
                '\u{2013}' | '\u{2014}' => '-',
                _ => raw.to_ascii_lowercase(),
            };
            let t = if ch.is_whitespace() {
                SPACE
            } else if (ch as u32) < 128 {
                lut[ch as usize]
            } else {
                OTHER
            };
            if t == SPACE && p1 == SPACE {
                continue;
            }
            if (t as usize) < c {
                let x = t as usize;
                self.mono[x] += 1;
                if (p1 as usize) < c {
                    self.bi[p1 as usize * c + x] += 1;
                    if (p2 as usize) < c {
                        self.tri[(p2 as usize * c + p1 as usize) * c + x] += 1;
                    }
                }
                if (p2 as usize) < c {
                    self.skip[p2 as usize * c + x] += 1;
                }
            }
            p2 = p1;
            p1 = t;
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
        for t in [&self.mono, &self.bi, &self.skip, &self.tri] {
            for &v in t.iter() {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        out
    }

    pub fn from_bytes(b: &[u8]) -> Result<Self> {
        let bad = || Error::parse("corpus", "truncated or corrupt corpus file");
        if b.len() < 8 || &b[..8] != MAGIC {
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
        for t in [&mut c.mono, &mut c.bi, &mut c.skip, &mut c.tri] {
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
            for (dst, src) in [(&mut out.mono, &c.mono), (&mut out.bi, &c.bi), (&mut out.skip, &c.skip), (&mut out.tri, &c.tri)] {
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
        let back = Corpus::from_bytes(&c.to_bytes()).unwrap();
        assert_eq!(back.tri, c.tri);
        assert_eq!(back.name, "t");
    }
}
