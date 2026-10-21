//! Adaptive keys (magic / repeat): a key whose output depends on the previous symbol.
//!
//! A [`Magic`] key has rules `previous → output`; with `repeat` it also outputs the previous
//! symbol when it has no rule for it (a plain repeat key is `repeat` with no rules). The typist
//! always uses the key where a rule applies: every occurrence of `x` right after `w` in the text
//! is typed with the key when `w → x` is a rule. Rules read the previous *output* symbol, so
//! the decision depends only on the original text (a second magic press after `c★` = `cy`
//! looks up `y`; QMK-style "magic after magic" overrides are not modeled). With several keys,
//! the first key listed that produces `x` after `w` is used. Multi-symbol outputs (briefs such
//! as `␣ → the`) are not modeled.
//!
//! [`rewrite`] applies the rules to a corpus's count tables before scoring: the key becomes one
//! more charset symbol and each n-gram moves to its rewritten form. Whether a symbol becomes a
//! key depends on the symbol before it, so an n-gram needs one more symbol of context:
//! - mono (from bigrams) and bigrams (from trigrams) are exact;
//! - a trigram's first symbol uses the share of its leading bigram that follows a trigger
//!   (Markov estimate); the other two are exact;
//! - skipgrams: the last symbol of a gap-1 skipgram uses the trigrams `x ? z` (the middle is
//!   assumed a charset symbol); the first symbol, and the last at gaps 2–3, use the share of
//!   that symbol's occurrences that follow a trigger (independence estimate).
//!
//! All rewritten tables are scaled by [`SCALE`] so fractional shares survive rounding (tables
//! are normalized on their own, so this is harmless).
//!
//! [`search_rules`] grows a rule set greedily for a fixed layout (see
//! `context/algorithms/magic-rules.md`).

use serde::{Deserialize, Serialize};

use crate::layout::parse_symbols;
use crate::{Corpus, Error, Geometry, Layout, Profile, Result, Scorer};

/// Default glyph of a magic key (`{magic}`).
pub const MAGIC: char = '\u{2605}'; // ★
/// Default glyph of a repeat key (`{repeat}`).
pub const REPEAT: char = '\u{21BB}'; // ↻
/// Rewritten counts are multiplied by this so split shares survive integer rounding.
pub const SCALE: f64 = 1000.0;

/// One adaptive key: `rules` are `(previous, output)` pairs, at most one per previous symbol.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Magic {
    /// The key's glyph on the layout.
    pub key: char,
    #[serde(default)]
    pub rules: Vec<(char, char)>,
    /// Without a rule for the previous symbol, output it again.
    #[serde(default)]
    pub repeat: bool,
}

/// TOML form: `[[magic]]` with `key = "{magic}"`, `rules = "ao cy …"` (pairs, previous then
/// output; tokens allowed), `repeat = true`.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct MagicFile {
    #[serde(default)]
    pub key: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub rules: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub repeat: bool,
}

impl Magic {
    pub fn from_file(f: &MagicFile) -> Result<Self> {
        let key = match parse_symbols(&f.key)?[..] {
            [] => MAGIC,
            [k] => k,
            _ => return Err(Error::invalid("magic", format!("key '{}' must be one symbol", f.key))),
        };
        let m = Magic { key, rules: parse_rules(&f.rules)?, repeat: f.repeat };
        m.check()?;
        Ok(m)
    }

    pub fn to_file(&self) -> MagicFile {
        MagicFile { key: self.key.to_string(), rules: format_rules(&self.rules), repeat: self.repeat }
    }

    /// At most one rule per previous symbol.
    pub fn check(&self) -> Result<()> {
        for (i, &(w, _)) in self.rules.iter().enumerate() {
            if self.rules[..i].iter().any(|&(v, _)| v == w) {
                return Err(Error::invalid("magic", format!("key '{}' has two rules after '{w}'", self.key)));
            }
        }
        Ok(())
    }

    /// Output after `prev`, if any.
    pub fn output(&self, prev: char) -> Option<char> {
        self.rules.iter().find(|&&(w, _)| w == prev).map(|&(_, x)| x).or(self.repeat.then_some(prev))
    }
}

/// `"ao cy {space}t"` → `[('a','o'), ('c','y'), ('␣','t')]`.
pub fn parse_rules(s: &str) -> Result<Vec<(char, char)>> {
    s.split_whitespace()
        .map(|r| match parse_symbols(r)?[..] {
            [w, x] => Ok((w, x)),
            _ => Err(Error::invalid("magic", format!("rule '{r}' must be two symbols (previous, output)"))),
        })
        .collect()
}

/// Inverse of [`parse_rules`] (tokens spelled `{name}`).
pub fn format_rules(rules: &[(char, char)]) -> String {
    let sym = |c: char| crate::layout::token_name(c).map_or(c.to_string(), |n| format!("{{{n}}}"));
    rules.iter().map(|&(w, x)| format!("{}{}", sym(w), sym(x))).collect::<Vec<_>>().join(" ")
}

/// `corpus` as typed with the adaptive `keys`: their glyphs join the charset (after the
/// existing symbols) and every n-gram moves to its rewritten form (module docs). Rules on
/// symbols outside the charset never fire.
pub fn rewrite(corpus: &Corpus, keys: &[Magic]) -> Result<Corpus> {
    let c = corpus.size();
    let nk = keys.len();
    let mut charset: String = corpus.charset.iter().collect();
    for (i, k) in keys.iter().enumerate() {
        if corpus.index_of(k.key).is_some() || keys[..i].iter().any(|o| o.key == k.key) {
            return Err(Error::invalid("magic", format!("key '{}' is already a symbol", k.key)));
        }
        charset.push(k.key);
    }
    if nk == 0 {
        return Ok(corpus.clone());
    }
    let mut out = Corpus::new(&format!("{}+magic", corpus.name), &charset);
    let cc = c + nk;
    // fire[w * c + x] = new id of x typed after w (c + key index, or x itself).
    let mut fire: Vec<usize> = (0..c * c).map(|wx| wx % c).collect();
    for w in 0..c {
        for (k, m) in keys.iter().enumerate() {
            if let Some(x) = m.output(corpus.charset[w]).and_then(|x| corpus.index_of(x)) {
                if fire[w * c + x] == x {
                    fire[w * c + x] = c + k;
                }
            }
        }
    }
    let f = |w: usize, x: usize| fire[w * c + x];
    let bi = |x: usize, y: usize| corpus.bi[x * c + y] as f64;
    let tri = |x: usize, y: usize, z: usize| corpus.tri[(x * c + y) * c + z] as f64;
    let put = |v: f64| (v * SCALE).round() as u64;

    // Variant shares of a symbol occurrence: index 0 = itself, 1 + k = typed with key k.
    // `lead[x]`: over all occurrences of x (from bigrams). `lead2[x, y]`: over x in bigram x y
    // (from trigrams).
    let mut lead = vec![0f64; c * (nk + 1)];
    for x in 0..c {
        let m = corpus.mono[x] as f64;
        if m > 0.0 {
            for w in 0..c {
                if f(w, x) >= c {
                    lead[x * (nk + 1) + 1 + f(w, x) - c] += bi(w, x) / m;
                }
            }
        }
    }
    let mut lead2 = vec![0f64; c * c * (nk + 1)];
    for x in 0..c {
        for y in 0..c {
            let b = bi(x, y);
            if b <= 0.0 {
                continue;
            }
            let s = &mut lead2[(x * c + y) * (nk + 1)..][..nk + 1];
            for w in 0..c {
                if f(w, x) >= c {
                    s[1 + f(w, x) - c] += tri(w, x, y) / b;
                }
            }
        }
    }
    for shares in [&mut lead, &mut lead2] {
        for s in shares.chunks_mut(nk + 1) {
            let t: f64 = s[1..].iter().sum();
            if t > 1.0 {
                s[1..].iter_mut().for_each(|v| *v /= t);
            }
            s[0] = 1.0 - s[1..].iter().sum::<f64>();
        }
    }
    let variant = |x: usize, k: usize| if k == 0 { x } else { c + k - 1 };

    for x in 0..c {
        for k in 0..=nk {
            out.mono[variant(x, k)] += put(corpus.mono[x] as f64 * lead[x * (nk + 1) + k]);
        }
    }
    for x in 0..c {
        for y in 0..c {
            let (b, y2) = (bi(x, y), f(x, y));
            if b <= 0.0 {
                continue;
            }
            for k in 0..=nk {
                out.bi[variant(x, k) * cc + y2] += put(b * lead2[(x * c + y) * (nk + 1) + k]);
            }
            for z in 0..c {
                let t = tri(x, y, z);
                if t <= 0.0 {
                    continue;
                }
                let z2 = f(y, z);
                for k in 0..=nk {
                    out.tri[(variant(x, k) * cc + y2) * cc + z2] += put(t * lead2[(x * c + y) * (nk + 1) + k]);
                }
            }
        }
    }
    let mut mid = vec![0f64; nk + 1];
    let mut wy = vec![0f64; c];
    for (g, src, dst) in [(1, &corpus.skip, &mut out.skip), (2, &corpus.skip2, &mut out.skip2), (3, &corpus.skip3, &mut out.skip3)] {
        for x in 0..c {
            for z in 0..c {
                let n = src[x * c + z] as f64;
                if n <= 0.0 {
                    continue;
                }
                // z's variant depends on the symbol y right before it. Weight each y: gap 1 by
                // the trigrams x y z; gaps 2–3 by a Markov chain through y,
                // skip(g-1)[x, y] · bi[y, z] / mono[y]. No weight: z's marginal shares.
                for y in 0..c {
                    wy[y] = match g {
                        1 => tri(x, y, z),
                        _ => {
                            let prev = if g == 2 { &corpus.skip } else { &corpus.skip2 };
                            let m = corpus.mono[y] as f64;
                            if m > 0.0 {
                                prev[x * c + y] as f64 * bi(y, z) / m
                            } else {
                                0.0
                            }
                        }
                    };
                }
                let tot: f64 = wy.iter().sum();
                if tot > 0.0 {
                    mid.iter_mut().for_each(|v| *v = 0.0);
                    for y in 0..c {
                        let v = f(y, z);
                        mid[if v >= c { 1 + v - c } else { 0 }] += wy[y] / tot;
                    }
                } else {
                    mid.copy_from_slice(&lead[z * (nk + 1)..][..nk + 1]);
                }
                for kx in 0..=nk {
                    let px = lead[x * (nk + 1) + kx];
                    if px <= 0.0 {
                        continue;
                    }
                    for (kz, &pz) in mid.iter().enumerate() {
                        if pz > 0.0 {
                            dst[variant(x, kx) * cc + variant(z, kz)] += put(n * px * pz);
                        }
                    }
                }
            }
        }
    }
    Ok(out)
}

/// Options for [`search_rules`].
#[derive(Clone, Debug)]
pub struct SearchOpts {
    /// Index into `layout.magic` of the key whose rules are searched.
    pub key: usize,
    /// Stop after this many rules.
    pub max_rules: usize,
    /// Keep the key's existing rules (else start from none).
    pub keep: bool,
    /// Accept a rule only when it lowers the score by more than this.
    pub min_gain: f64,
}

impl Default for SearchOpts {
    fn default() -> Self {
        SearchOpts { key: 0, max_rules: 32, keep: false, min_gain: 0.01 }
    }
}

/// One accepted (or, prefixed `-`, dropped) rule and the score after it.
#[derive(Clone, Debug, Serialize)]
pub struct Step {
    pub rule: String,
    pub score: f64,
}

#[derive(Clone, Debug)]
pub struct SearchResult {
    pub layout: Layout,
    pub start_score: f64,
    pub score: f64,
    pub steps: Vec<Step>,
    pub evals: usize,
}

/// Greedy rule search for one adaptive key on a fixed layout (no symbol moves). Candidates are
/// every `w → x` with both on the layout, `w x` in the corpus and no rule for `w` yet (`x = w`
/// only when the key does not already repeat). Lazy greedy: each candidate's gain is scored
/// exactly once (corpus rewrite + full evaluation), then only the best stale gain is re-scored
/// until the best is fresh; it is accepted while it lowers the score by more than `min_gain`. Finally rules whose
/// removal lowers the score are dropped.
pub fn search_rules(g: &Geometry, corpus: &Corpus, profile: &Profile, layout: &Layout, opts: &SearchOpts) -> Result<SearchResult> {
    if opts.key >= layout.magic.len() {
        return Err(Error::invalid("magic", format!("'{}' has no adaptive key #{}", layout.name, opts.key + 1)));
    }
    let k = opts.key;
    let mut cur = layout.clone();
    if !opts.keep {
        cur.magic[k].rules.clear();
    }
    let mut evals = 0;
    let mut eval = |l: &Layout| -> Result<f64> {
        evals += 1;
        Ok(Scorer::new(g, corpus, profile, l)?.identity().score)
    };
    let with = |l: &Layout, r: (char, char)| {
        let mut l = l.clone();
        l.magic[k].rules.push(r);
        l
    };
    let start_score = eval(layout)?;
    let mut score = eval(&cur)?;
    let on: Vec<char> = layout.chars.iter().chain(layout.layers.iter().flat_map(|l| l.chars.iter())).copied().collect();
    let syms: Vec<usize> = (0..corpus.size()).filter(|&w| on.contains(&corpus.charset[w])).collect();
    let c = corpus.size();
    // (gain, score with the rule added, rule, round both were measured in)
    let mut cands: Vec<(f64, f64, (char, char), usize)> = Vec::new();
    for &w in &syms {
        let wc = corpus.charset[w];
        if cur.magic[k].rules.iter().any(|&(v, _)| v == wc) {
            continue;
        }
        for &x in syms.iter().filter(|&&x| corpus.bi[w * c + x] > 0 && !(x == w && cur.magic[k].repeat)) {
            let r = (wc, corpus.charset[x]);
            let s = eval(&with(&cur, r))?;
            cands.push((score - s, s, r, 0));
        }
    }
    let mut steps = Vec::new();
    let mut round = 0;
    while cur.magic[k].rules.len() < opts.max_rules {
        let Some(i) = (0..cands.len()).max_by(|&a, &b| cands[a].0.total_cmp(&cands[b].0)) else { break };
        let (gain, s, r, at) = cands[i];
        if at != round {
            let s = eval(&with(&cur, r))?;
            cands[i] = (score - s, s, r, round);
            continue;
        }
        if gain <= opts.min_gain.max(1e-9) {
            break;
        }
        cur = with(&cur, r);
        score = s;
        round += 1;
        cands.retain(|&(_, _, (w, _), _)| w != r.0);
        steps.push(Step { rule: format_rules(&[r]), score });
    }
    // Prune: a rule added early may stop paying once later rules exist.
    let mut i = 0;
    while i < cur.magic[k].rules.len() {
        let mut l = cur.clone();
        let r = l.magic[k].rules.remove(i);
        let s = eval(&l)?;
        if s < score - 1e-9 {
            cur = l;
            score = s;
            steps.push(Step { rule: format!("-{}", format_rules(&[r])), score: s });
        } else {
            i += 1;
        }
    }
    Ok(SearchResult { layout: cur, start_score, score, steps, evals })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Text with each occurrence of x after w replaced by the key glyph (output semantics).
    fn typed(text: &str, keys: &[Magic]) -> String {
        let cs: Vec<char> = text.chars().collect();
        let mut out = String::new();
        for (i, &x) in cs.iter().enumerate() {
            let k = i.checked_sub(1).and_then(|j| keys.iter().find(|m| m.output(cs[j]) == Some(x)));
            out.push(k.map_or(x, |m| m.key));
        }
        out
    }

    #[test]
    fn rules_round_trip() {
        let r = parse_rules("ao cy {space}t").unwrap();
        assert_eq!(r, vec![('a', 'o'), ('c', 'y'), (crate::layout::SPACE, 't')]);
        assert_eq!(parse_rules(&format_rules(&r)).unwrap(), r);
        assert!(parse_rules("abc").is_err());
        let m = Magic::from_file(&MagicFile { key: "{magic}".into(), rules: "ab ac".into(), repeat: false });
        assert!(m.is_err());
    }

    #[test]
    fn rewrite_matches_rewritten_text() {
        let text = "abcab cabba acbca bbacc abcabc aabbcc cbacba babcab ".repeat(5);
        let keys = [
            Magic { key: 'm', rules: vec![('a', 'b'), ('c', 'a')], repeat: false },
            Magic { key: 'r', rules: vec![('b', 'c')], repeat: true },
        ];
        let mut orig = Corpus::new("t", "abc\u{2423}");
        orig.add_text(&text);
        let got = rewrite(&orig, &keys).unwrap();
        let mut want = Corpus::new("w", "abc\u{2423}mr");
        want.add_text(&typed(&text, &keys));
        assert_eq!(got.charset, want.charset);
        let scaled = |v: &[u64]| v.iter().map(|&x| x * SCALE as u64).collect::<Vec<_>>();
        // Exact: mono and bigrams.
        assert_eq!(got.mono, scaled(&want.mono));
        assert_eq!(got.bi, scaled(&want.bi));
        // Estimated: totals are kept and the result is closer to the truth than no rewrite.
        let none = rewrite(&orig, &[Magic { key: 'm', ..Default::default() }, Magic { key: 'r', ..Default::default() }]).unwrap();
        let l1 = |a: &[u64], b: &[u64]| {
            let (ta, tb) = (a.iter().sum::<u64>() as f64, b.iter().sum::<u64>() as f64);
            a.iter().zip(b).map(|(&x, &y)| (x as f64 / ta - y as f64 / tb).abs()).sum::<f64>()
        };
        for (g, w, n) in [
            (&got.tri, &want.tri, &none.tri),
            (&got.skip, &want.skip, &none.skip),
            (&got.skip2, &want.skip2, &none.skip2),
            (&got.skip3, &want.skip3, &none.skip3),
        ] {
            let (tg, tw) = (g.iter().sum::<u64>() as f64, w.iter().sum::<u64>() as f64 * SCALE);
            assert!((tg - tw).abs() / tw < 1e-3, "{tg} vs {tw}");
            assert!(l1(g, w) < l1(n, w), "estimate {} vs unrewritten {}", l1(g, w), l1(n, w));
        }
    }

    #[test]
    fn no_keys_is_identity() {
        let mut c = Corpus::new("t", "ab");
        c.add_text("abba ab");
        let r = rewrite(&c, &[]).unwrap();
        assert_eq!((&r.mono, &r.bi, &r.tri), (&c.mono, &c.bi, &c.tri));
        assert!(rewrite(&c, &[Magic { key: 'a', ..Default::default() }]).is_err());
    }
}
