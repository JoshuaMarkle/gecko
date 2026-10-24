//! Keysets: which symbols a search places, chosen from the board and the corpus, and a start
//! layout for them (`gecko keyset`, `optimize --keyset`). See `context/keysets.md`.
//!
//! Modes:
//! - `alpha`: the letters and the board's special keys; every other key stays empty (`·`, zero
//!   frequency, so the search leaves the worst keys empty).
//! - `shift`: letters plus symbol keys that each carry a Shift pair (`-_`, `[{`, …). Pairs move
//!   as one key; the shifted symbol costs a Shift press (`Corpus::keyed`). Natural (US) pairs by
//!   default, or pairs by frequency rank, or given pairs. The most frequent pairs fill the free keys.
//! - `layer`: letters on the base, a fixed layer key, and the most frequent symbols on the free
//!   base keys and the layer; the search moves symbols between base and layer freely.
//!
//! Special keys: Space, Backspace, Enter, Tab, Esc move like symbols; Shift, Ctrl and the layer
//! key are fixed during a search, so [`place_fixed`] searches their keys first (each candidate
//! key, then a local re-optimization of the layout).

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::corpus::{Corpus, US_SHIFT};
use crate::geometry::{Geometry, LayerMode};
use crate::layout::{self, Layer, Layout, BACKSPACE, CTRL, EMPTY, ENTER, ESC, SHIFT, SPACE, TAB};
use crate::optimize::{self, Algo, Control, Params};
use crate::{Error, Profile, Result, Scorer};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Alpha,
    Shift,
    Layer,
}

impl Mode {
    pub fn parse(s: &str) -> Option<Mode> {
        match s {
            "alpha" => Some(Mode::Alpha),
            "shift" | "symbols" => Some(Mode::Shift),
            "layer" => Some(Mode::Layer),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Mode::Alpha => "alpha",
            Mode::Shift => "shift",
            Mode::Layer => "layer",
        }
    }
}

/// How symbols pair up on Shift keys (`shift` mode).
#[derive(Clone, Debug, PartialEq)]
pub enum Pairing {
    /// US-QWERTY pairs (`-_ =+ [{ ]} \| ;: '" ,< .> /? `~`, and `1!`… with digits).
    Natural,
    /// The k most frequent symbols on base keys, the next k shifted, paired by rank.
    Frequency,
    /// These (base, shifted) pairs.
    Custom(Vec<(char, char)>),
}

impl Pairing {
    /// `natural`, `frequency`, or pairs like `"-_ =+ [{"`.
    pub fn parse(s: &str) -> Result<Pairing> {
        Ok(match s {
            "natural" | "us" => Pairing::Natural,
            "frequency" | "freq" => Pairing::Frequency,
            _ => Pairing::Custom(layout::parse_shifted(&[s])?.iter().map(|p| layout::pair_chars(p)).collect()),
        })
    }
}

#[derive(Clone, Debug)]
pub struct Spec {
    pub mode: Mode,
    /// Symbol keys to fill (`shift`: pair keys; `layer`: symbols on base and layer); `None` = as
    /// many as fit.
    pub symbols: Option<usize>,
    pub pairing: Pairing,
    /// Digits count as symbols. `None`: yes in `layer` mode, no in `shift` mode.
    pub digits: Option<bool>,
    /// Special keys to place; `None` = by board ([`auto_keys`]).
    pub keys: Option<Vec<char>>,
    pub layer_mode: LayerMode,
    /// Letters and symbols may take thumb keys. Off (the default), they fill only the other
    /// keys, leaving the thumbs to special keys ([`thumb_zone`]).
    pub thumb_letters: bool,
}

impl Spec {
    pub fn new(mode: Mode) -> Self {
        Spec { mode, symbols: None, pairing: Pairing::Natural, digits: None, keys: None, layer_mode: LayerMode::Hold, thumb_letters: false }
    }
}

/// One placed (or left out) key of a keyset.
#[derive(Clone, Debug, Serialize)]
pub struct KeyInfo {
    pub symbol: char,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shifted: Option<char>,
    /// % of the corpus' keystrokes (symbol and its shifted partner).
    pub share: f64,
    /// `letter`, `special`, `fixed`, `symbol`, or `layer` (a symbol placed on the layer).
    pub kind: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct Keyset {
    pub mode: Mode,
    pub layout: Layout,
    pub keys: Vec<KeyInfo>,
    /// Symbols of the corpus that got no key (most frequent first).
    pub left_out: Vec<KeyInfo>,
    /// % of the corpus' keystrokes typeable on the layout.
    pub coverage: f64,
    /// Empty keys left.
    pub empty: usize,
}

/// Special keys a board gets by default: on a board with thumb keys Space, Backspace, Enter,
/// Shift and Ctrl (Tab and Esc as well when keys are left; see [`EXTRA_KEYS`]); none on a
/// board without (its Space bar and modifiers sit off the modelled keys).
pub fn auto_keys(g: &Geometry) -> Vec<char> {
    if g.key.iter().any(|k| k.finger.is_thumb()) {
        vec![SPACE, BACKSPACE, ENTER, SHIFT, CTRL]
    } else {
        Vec::new()
    }
}

/// Placed by default only on keys nothing else needs.
pub const EXTRA_KEYS: [char; 2] = [TAB, ESC];

/// Special-key names for `--keys`: `space,bspc,enter,shift,ctrl,tab,esc` (any token name).
pub fn parse_keys(s: &str) -> Result<Vec<char>> {
    s.split([',', ' '])
        .filter(|w| !w.is_empty())
        .map(|w| layout::token(w).ok_or_else(|| Error::invalid("keys", format!("unknown key '{w}'"))))
        .collect()
}

fn is_letter(c: char) -> bool {
    c.is_ascii_lowercase()
}

/// Build the keyset for `spec` on board `g` from corpus `c`.
pub fn build(g: &Geometry, c: &Corpus, spec: &Spec) -> Result<Keyset> {
    let n = g.len();
    let total = c.total_chars().max(1) as f64;
    let share = |ch: char| c.index_of(ch).map_or(0.0, |i| 100.0 * c.mono[i] as f64 / total);
    let letters: Vec<char> = c.charset.iter().copied().filter(|&ch| is_letter(ch)).collect();
    if letters.len() < 26 {
        return Err(Error::invalid("keyset", format!("corpus '{}' lacks letters ({} of 26)", c.name, letters.len())));
    }
    let digits = spec.digits.unwrap_or(spec.mode == Mode::Layer);
    let is_symbol = |ch: char| ch.is_ascii_punctuation() || (digits && ch.is_ascii_digit());

    // Special keys: explicit, or by board with the extras only if keys are left.
    let explicit = spec.keys.is_some();
    let mut keys: Vec<char> = spec.keys.clone().unwrap_or_else(|| auto_keys(g));
    let layer_key = layout::layer_key(1, spec.layer_mode);
    if spec.mode == Mode::Layer && !keys.iter().any(|&k| layout::is_layer_key(k)) {
        keys.push(layer_key);
    }
    for &k in &keys {
        if !layout::is_fixed_key(k) && keys.iter().filter(|&&x| x == k).count() > 1 {
            return Err(Error::invalid("keyset", format!("key '{k}' listed twice")));
        }
    }
    let free_keys = n.checked_sub(letters.len() + keys.len()).ok_or_else(|| {
        Error::invalid(
            "keyset",
            format!("'{}' has {n} keys: too few for {} letters and {} special keys", g.name, letters.len(), keys.len()),
        )
    })?;

    let fixed: Vec<char> = keys.iter().copied().filter(|&k| layout::is_fixed_key(k)).collect();
    let fixed_slots = fixed_slots(g, fixed.len());
    let holders: Vec<(usize, LayerMode)> =
        fixed.iter().zip(&fixed_slots).filter_map(|(&k, &s)| layout::layer_key_info(k).map(|(_, m)| (s, m))).collect();

    // Symbol keys.
    let mut base_syms: Vec<(char, Option<char>)> = Vec::new();
    let mut layer_syms: Vec<char> = Vec::new();
    let mut left_out: Vec<KeyInfo> = Vec::new();
    let cap = |room: usize| spec.symbols.map_or(room, |s| s.min(room));
    // Without thumb letters, base symbols get only the non-thumb keys that letters and any
    // special keys spilling off the thumbs leave.
    let thumbs = g.key.iter().filter(|k| k.finger.is_thumb()).count();
    let base_room = if spec.thumb_letters || thumbs == 0 {
        free_keys
    } else {
        free_keys.min((n - thumbs).saturating_sub(letters.len() + keys.len().saturating_sub(thumbs)))
    };
    match spec.mode {
        Mode::Alpha => {}
        Mode::Shift => {
            let k = cap(base_room);
            let pairs = shift_pairs(c, &spec.pairing, digits, k, &share);
            let used: Vec<char> = pairs.iter().flat_map(|p| [p.0, p.1]).collect();
            base_syms.extend(pairs.iter().take(k).map(|&(b, s)| (b, Some(s))));
            for &(b, s) in pairs.iter().skip(k) {
                left_out.push(KeyInfo { symbol: b, shifted: Some(s), share: share(b) + share(s), kind: "symbol" });
            }
            // Symbols in no pair at all.
            for &ch in c.charset.iter().filter(|&&ch| is_symbol(ch) && !used.contains(&ch)) {
                left_out.push(KeyInfo { symbol: ch, shifted: None, share: share(ch), kind: "symbol" });
            }
        }
        Mode::Layer => {
            let room_layer = g.layered(&holders)?.len() - n;
            let mut syms: Vec<char> = c.charset.iter().copied().filter(|&ch| is_symbol(ch)).collect();
            syms.sort_by(|&a, &b| share(b).total_cmp(&share(a)).then(a.cmp(&b)));
            let take = cap(base_room + room_layer);
            for (i, &ch) in syms.iter().enumerate() {
                if i >= take {
                    left_out.push(KeyInfo { symbol: ch, shifted: None, share: share(ch), kind: "symbol" });
                } else if i < base_room {
                    base_syms.push((ch, None));
                } else {
                    layer_syms.push(ch);
                }
            }
        }
    }

    // Extras (Tab, Esc) by default on boards with special keys: on free base keys, else on the layer.
    if !explicit && !auto_keys(g).is_empty() {
        let mut free_base = free_keys - base_syms.len();
        for &x in &EXTRA_KEYS {
            if free_base > 0 {
                keys.push(x);
                free_base -= 1;
            } else if spec.mode == Mode::Layer {
                layer_syms.push(x);
            } else {
                left_out.push(KeyInfo { symbol: x, shifted: None, share: share(x), kind: "special" });
            }
        }
    }

    // Start layout: fixed keys first, then everything else by frequency onto the easiest keys.
    let mut chars = vec![EMPTY; n];
    for (&k, &s) in fixed.iter().zip(&fixed_slots) {
        chars[s] = k;
    }
    // Special keys on the thumb keys, letters and symbols off them (the thumb zone, L04);
    // each group by frequency onto the easiest keys, spilling over when its keys run out.
    let freq = |ch: char| share(ch) + layout_shifted(&base_syms, ch).map_or(0.0, &share);
    let by_freq = |v: &mut Vec<char>| v.sort_by(|&a, &b| freq(b).total_cmp(&freq(a)).then(a.cmp(&b)));
    let mut specials: Vec<char> = keys.iter().copied().filter(|&k| !layout::is_fixed_key(k)).collect();
    let mut typed: Vec<char> = letters.clone();
    typed.extend(base_syms.iter().map(|p| p.0));
    by_freq(&mut specials);
    by_freq(&mut typed);
    let mut open: Vec<usize> = (0..n).filter(|&s| chars[s] == EMPTY).collect();
    open.sort_by(|&a, &b| g.key[a].effort.total_cmp(&g.key[b].effort).then(a.cmp(&b)));
    let (mut thumb, mut other): (Vec<usize>, Vec<usize>) = open.into_iter().partition(|&s| g.key[s].finger.is_thumb());
    for ch in specials {
        let s = if thumb.is_empty() { other.pop() } else { Some(thumb.remove(0)) };
        chars[s.expect("room checked")] = ch;
    }
    for ch in typed {
        let s = if other.is_empty() { thumb.remove(0) } else { other.remove(0) };
        chars[s] = ch;
    }
    let mut l = Layout::from_chars(&format!("keyset-{}", spec.mode.name()), &g.name, "keyset", chars)?;
    l.shifted = base_syms.iter().filter_map(|&(b, s)| s.map(|s| [b, s].iter().collect())).collect();
    if spec.mode == Mode::Layer {
        let vg = g.layered(&holders)?;
        let mut usable: Vec<usize> = (n..vg.len()).filter(|&v| vg.key[v].layer == 1).map(|v| vg.phys(v)).collect();
        usable.sort_by(|&a, &b| g.key[a].effort.total_cmp(&g.key[b].effort).then(a.cmp(&b)));
        let mut layer = vec![EMPTY; n];
        if layer_syms.len() > usable.len() {
            for &ch in &layer_syms[usable.len()..] {
                left_out.push(KeyInfo { symbol: ch, shifted: None, share: share(ch), kind: "symbol" });
            }
            layer_syms.truncate(usable.len());
        }
        for (&ch, &k) in layer_syms.iter().zip(&usable) {
            layer[k] = ch;
        }
        l.layers.push(Layer { name: "sym".into(), chars: layer });
    }
    l.check_shifted()?;

    let mut info: Vec<KeyInfo> = Vec::new();
    for &ch in &letters {
        info.push(KeyInfo { symbol: ch, shifted: None, share: share(ch), kind: "letter" });
    }
    for &k in &keys {
        let kind = if layout::is_fixed_key(k) { "fixed" } else { "special" };
        if !info.iter().any(|i| i.symbol == k) {
            info.push(KeyInfo { symbol: k, shifted: None, share: share(k), kind });
        }
    }
    for &(b, s) in &base_syms {
        info.push(KeyInfo { symbol: b, shifted: s, share: share(b) + s.map_or(0.0, &share), kind: "symbol" });
    }
    for &ch in &layer_syms {
        info.push(KeyInfo { symbol: ch, shifted: None, share: share(ch), kind: "layer" });
    }
    left_out.sort_by(|a, b| b.share.total_cmp(&a.share).then(a.symbol.cmp(&b.symbol)));
    left_out.retain(|k| k.share > 0.0 || k.kind == "special");
    let coverage = info.iter().map(|k| k.share).sum::<f64>();
    let empty = l.chars.iter().filter(|&&ch| ch == EMPTY).count();
    Ok(Keyset { mode: spec.mode, layout: l, keys: info, left_out, coverage, empty })
}

fn layout_shifted(pairs: &[(char, Option<char>)], ch: char) -> Option<char> {
    pairs.iter().find(|p| p.0 == ch).and_then(|p| p.1)
}

/// Candidate Shift pairs, most frequent first: natural (US), by frequency rank (`k` pairs from
/// the 2k most frequent symbols), or the given ones. Pairs with no frequency are dropped
/// unless given.
fn shift_pairs(c: &Corpus, pairing: &Pairing, digits: bool, k: usize, share: &dyn Fn(char) -> f64) -> Vec<(char, char)> {
    let in_corpus = |ch: char| c.index_of(ch).is_some();
    let mut pairs: Vec<(char, char)> = match pairing {
        Pairing::Natural => US_SHIFT
            .iter()
            .map(|&(s, b)| (b, s))
            .filter(|&(b, _)| digits || !b.is_ascii_digit())
            .filter(|&(b, s)| in_corpus(b) || in_corpus(s))
            .collect(),
        Pairing::Frequency => {
            let mut syms: Vec<char> =
                c.charset.iter().copied().filter(|&ch| ch.is_ascii_punctuation() || (digits && ch.is_ascii_digit())).collect();
            syms.sort_by(|&a, &b| share(b).total_cmp(&share(a)).then(a.cmp(&b)));
            let k = k.min(syms.len() / 2);
            (0..k).map(|i| (syms[i], syms[k + i])).collect()
        }
        Pairing::Custom(p) => p.clone(),
    };
    pairs.sort_by(|a, b| (share(b.0) + share(b.1)).total_cmp(&(share(a.0) + share(a.1))).then(a.cmp(b)));
    if !matches!(pairing, Pairing::Custom(_)) {
        pairs.retain(|&(b, s)| share(b) + share(s) > 0.0);
    }
    pairs
}

/// Initial keys of `count` fixed keys (Shift, Ctrl, layer key): thumb keys, hardest first,
/// alternating hands (the search moves Space and the other frequent keys onto the easiest
/// thumbs; [`place_fixed`] then moves the fixed keys); without thumbs the hardest keys.
fn fixed_slots(g: &Geometry, count: usize) -> Vec<usize> {
    let thumbs: Vec<usize> = (0..g.len()).filter(|&i| g.key[i].finger.is_thumb()).collect();
    let mut pool = if thumbs.is_empty() { (0..g.len()).collect() } else { thumbs };
    pool.sort_by(|&a, &b| g.key[b].effort.total_cmp(&g.key[a].effort).then(a.cmp(&b)));
    let mut out = Vec::new();
    let mut hand = None;
    while out.len() < count && !pool.is_empty() {
        let i = pool.iter().position(|&s| Some(g.key[s].hand()) != hand).unwrap_or(0);
        let s = pool.remove(i);
        hand = Some(g.key[s].hand());
        out.push(s);
    }
    out
}

/// Spec from text options (CLI flags, API fields): mode, `symbols`, pairing (`natural`,
/// `frequency` or pairs), digits, `keys` list, layer mode (`hold`, `oneshot`, `toggle`) and
/// whether letters may take thumb keys.
pub fn spec_from(
    mode: &str,
    symbols: Option<usize>,
    pairs: &str,
    digits: Option<bool>,
    keys: Option<&str>,
    layer_mode: &str,
    thumb_letters: bool,
) -> Result<Spec> {
    let mode = Mode::parse(mode).ok_or_else(|| Error::invalid("keyset", format!("unknown keyset '{mode}' (alpha, shift, layer)")))?;
    let layer_mode = match layer_mode {
        "hold" | "" => LayerMode::Hold,
        "oneshot" | "os" => LayerMode::Oneshot,
        "toggle" | "tg" => LayerMode::Toggle,
        m => return Err(Error::invalid("keyset", format!("unknown layer mode '{m}' (hold, oneshot, toggle)"))),
    };
    Ok(Spec {
        mode,
        symbols,
        pairing: Pairing::parse(if pairs.is_empty() { "natural" } else { pairs })?,
        digits,
        keys: keys.filter(|k| !k.is_empty()).map(parse_keys).transpose()?,
        layer_mode,
        thumb_letters,
    })
}

/// Adapt profile `p` to keyset layout `l` on `g` and say what changed:
/// - the thumb zone (unless letters may take thumbs) and, for a layered layout, `base_letters`,
///   with L04 at [`ZONE_WEIGHT`] if unweighted;
/// - Y01 at the `code` profile's weight (1, scale 2) if unweighted, so Shift, Ctrl and layer-key
///   presses cost more than their effort.
pub fn apply_profile(g: &Geometry, l: &Layout, p: &mut Profile, thumb_letters: bool) -> Vec<String> {
    let mut notes = Vec::new();
    if !thumb_letters {
        if let Some(zone) = thumb_zone(g, l) {
            notes.push(format!("thumb zone {zone}"));
            p.thumb_zone = Some(zone);
        }
    }
    if !l.layers.is_empty() {
        p.base_letters = true;
        notes.push("letters on the base layer".into());
    }
    if (p.thumb_zone.is_some() || p.base_letters) && p.weight("L04") == 0.0 {
        p.weights.insert("L04".into(), ZONE_WEIGHT);
        notes.push(format!("L04 weight {ZONE_WEIGHT}"));
    }
    let modifiers = l.chars.iter().any(|&c| layout::is_fixed_key(c));
    if modifiers && p.weight("Y01") == 0.0 {
        p.weights.insert("Y01".into(), 1.0);
        p.scale.insert("Y01".into(), 2.0);
        notes.push("Y01 weight 1 (scale 2, as profile code)".into());
    }
    notes
}

/// L04 weight keysets give the thumb zone: far above any trade-off, so it acts as a constraint.
pub const ZONE_WEIGHT: f32 = 100.0;

/// The thumb zone for `l` on `g` (profile `thumb_zone`): Space, Backspace and Enter if the
/// layout has them and `g` has thumb keys.
pub fn thumb_zone(g: &Geometry, l: &Layout) -> Option<String> {
    if !g.key.iter().any(|k| k.finger.is_thumb()) {
        return None;
    }
    let zone: String = [SPACE, BACKSPACE, ENTER].iter().filter(|c| l.chars.contains(c)).collect();
    (!zone.is_empty()).then_some(zone)
}

/// Options for [`place_fixed`].
#[derive(Clone, Debug)]
pub struct PlaceOpts {
    /// Simulated-annealing evaluations for the first optimization and after each accepted move.
    pub evals: u64,
    pub seed: u64,
    pub threads: usize,
    /// Rounds over all fixed keys (stops early when no move helps).
    pub rounds: usize,
    /// Symbols that keep their keys.
    pub pins: Vec<char>,
}

impl Default for PlaceOpts {
    fn default() -> Self {
        PlaceOpts { evals: 2_000_000, seed: 1, threads: 1, rounds: 3, pins: Vec::new() }
    }
}

/// One fixed-key move tried by [`place_fixed`].
#[derive(Clone, Debug, Serialize)]
pub struct Trial {
    pub key: char,
    pub from: usize,
    pub to: usize,
    /// Score after a local re-optimization.
    pub score: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Placement {
    pub layout: Layout,
    pub start_score: f64,
    pub score: f64,
    /// Accepted moves, in order.
    pub moves: Vec<Trial>,
    /// Candidates evaluated.
    pub tried: usize,
}

/// Search the keys of the fixed keys (Shift, Ctrl, layer keys) of `start`: optimize the layout
/// (SA, `opts.evals`), then repeatedly try every fixed key on every candidate key (thumb keys
/// on a board with them, else every base key), letting the displaced symbol take the fixed
/// key's old place, re-optimize locally (steepest descent) and keep the best improving move,
/// re-annealing after it. Layer symbols left on keys a moved hold key makes unusable move to
/// free layer keys (the candidate is skipped when none are free).
pub fn place_fixed(g: &Geometry, c: &Corpus, p: &Profile, start: &Layout, opts: &PlaceOpts) -> Result<Placement> {
    let n = g.len();
    let anneal = |l: &Layout, seed: u64| -> Result<(Layout, f64)> {
        let sc = Scorer::new(g, c, p, l)?;
        let movable: Vec<usize> = (0..sc.n).filter(|&s| !opts.pins.contains(&sc.chars[s])).collect();
        let ctx = optimize::Ctx::new(&sc, movable, sc.identity());
        let params = Params {
            algo: Algo::Sa,
            seed,
            threads: opts.threads.max(1),
            time_ms: None,
            evals: Some(opts.evals),
            top: 1,
            ..Default::default()
        };
        let o = optimize::run(&ctx, &params, Duration::from_secs(3600), &Control::default(), |_| {});
        let best = o.top.into_iter().next().ok_or_else(|| Error::invalid("keyset", "search returned no layout"))?;
        let st = sc.state_from_sym(best.sym);
        Ok((sc.layout_of(&st, &l.name)?.with_extras_of(l)?, st.score))
    };
    let descend = |l: &Layout| -> Result<(Layout, f64)> {
        let sc = Scorer::new(g, c, p, l)?;
        let movable: Vec<usize> = (0..sc.n).filter(|&s| !opts.pins.contains(&sc.chars[s])).collect();
        let ctx = optimize::Ctx::new(&sc, movable, sc.identity());
        let mut st = sc.identity();
        optimize::steepest_descent(&ctx, &mut st);
        Ok((sc.layout_of(&st, &l.name)?.with_extras_of(l)?, st.score))
    };
    let start_score = Scorer::new(g, c, p, start)?.identity().score;
    let (mut cur, mut score) = anneal(start, opts.seed)?;
    let thumbs: Vec<usize> = (0..n).filter(|&i| g.key[i].finger.is_thumb()).collect();
    let candidates: Vec<usize> = if thumbs.is_empty() { (0..n).collect() } else { thumbs };
    let (mut moves, mut tried) = (Vec::new(), 0);
    for round in 0..opts.rounds {
        let mut best: Option<(Trial, Layout)> = None;
        let fixed: Vec<usize> = (0..n).filter(|&s| layout::is_fixed_key(cur.chars[s])).collect();
        for &from in &fixed {
            for &to in &candidates {
                if to == from || layout::is_fixed_key(cur.chars[to]) || opts.pins.contains(&cur.chars[to]) {
                    continue;
                }
                let Some(trial) = move_fixed(g, &cur, from, to)? else { continue };
                tried += 1;
                let (l, s) = descend(&trial)?;
                if s < score - 1e-9 && best.as_ref().is_none_or(|b| s < b.0.score) {
                    best = Some((Trial { key: cur.chars[from], from, to, score: s }, l));
                }
            }
        }
        let Some((t, l)) = best else { break };
        let (l, s) = anneal(&l, opts.seed + 1 + round as u64)?;
        let (l, s) = if s < t.score { (l, s) } else { descend(&l)? };
        moves.push(Trial { score: s, ..t });
        (cur, score) = (l, s);
    }
    Ok(Placement { layout: cur, start_score, score, moves, tried })
}

/// `l` with the fixed key on base key `from` moved to `to` and `to`'s symbols (base and every
/// layer) moved to `from`; layer symbols on keys the new holder cannot use move to free usable
/// layer keys. `None` when they do not fit.
fn move_fixed(g: &Geometry, l: &Layout, from: usize, to: usize) -> Result<Option<Layout>> {
    let mut out = l.clone();
    out.chars.swap(from, to);
    for ly in &mut out.layers {
        ly.chars.swap(from, to);
    }
    if out.layers.is_empty() {
        return Ok(Some(out));
    }
    let vg = g.layered(&out.holders()?)?;
    for li in 1..=out.layers.len() {
        let mut usable = vec![false; g.len()];
        for v in (g.len()..vg.len()).filter(|&v| vg.key[v].layer as usize == li) {
            usable[vg.phys(v)] = true;
        }
        let ly = &mut out.layers[li - 1].chars;
        let stranded: Vec<usize> = (0..g.len()).filter(|&k| !usable[k] && ly[k] != EMPTY).collect();
        let free: Vec<usize> = (0..g.len()).filter(|&k| usable[k] && ly[k] == EMPTY).collect();
        let mut free = free.into_iter();
        for k in stranded {
            let Some(f) = free.next() else { return Ok(None) };
            ly[f] = ly[k];
            ly[k] = EMPTY;
        }
    }
    Ok(Some(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corne() -> Geometry {
        Geometry::from_toml(include_str!("../../../data/geometries/corne-36.toml")).unwrap()
    }

    fn code() -> Corpus {
        let mut c = Corpus::new("t", crate::corpus::CODE_CHARSET);
        c.add_text("Hello, world! fn main() { let x = a[0] + b_c; } // done: \"ok\" -> 'q' z = y - 1; vjkx");
        c
    }

    #[test]
    fn alpha_keeps_letters_and_specials() {
        let k = build(&corne(), &code(), &Spec::new(Mode::Alpha)).unwrap();
        let l = &k.layout;
        assert_eq!(l.chars.iter().filter(|c| c.is_ascii_lowercase()).count(), 26);
        for s in [SPACE, BACKSPACE, ENTER, SHIFT, CTRL, TAB, ESC] {
            assert!(l.chars.contains(&s), "{s}");
        }
        assert_eq!(k.empty, 36 - 26 - 7);
        assert!(l.chars.iter().all(|c| !c.is_ascii_punctuation()));
    }

    #[test]
    fn shift_pairs_fill_free_keys() {
        // 30 non-thumb keys: 26 letters and 4 pair keys; the sixth thumb key gets Tab.
        let k = build(&corne(), &code(), &Spec::new(Mode::Shift)).unwrap();
        let l = &k.layout;
        assert_eq!(l.shifted.len(), 4);
        assert_eq!(k.empty, 0);
        assert!(l.chars.contains(&TAB));
        let free = build(&corne(), &code(), &Spec { thumb_letters: true, ..Spec::new(Mode::Shift) }).unwrap();
        assert_eq!(free.layout.shifted.len(), 36 - 26 - 5);
        for p in &l.shifted {
            let (b, s) = layout::pair_chars(p);
            assert!(US_SHIFT.contains(&(s, b)), "{p} is a US pair");
            assert!(l.chars.contains(&b) && !l.chars.contains(&s));
        }
        let custom = Spec { pairing: Pairing::parse("({ )} -_").unwrap(), symbols: Some(2), ..Spec::new(Mode::Shift) };
        let k = build(&corne(), &code(), &custom).unwrap();
        assert_eq!(k.layout.shifted.len(), 2);
        assert!(k.layout.shifted.iter().all(|p| ["({", ")}", "-_"].contains(&p.as_str())));
    }

    #[test]
    fn layer_mode_has_a_fixed_layer_key() {
        let k = build(&corne(), &code(), &Spec::new(Mode::Layer)).unwrap();
        let l = &k.layout;
        assert_eq!(l.layers.len(), 1);
        assert_eq!(l.holders().unwrap().len(), 1);
        assert!(l.layers[0].chars.contains(&'('));
        let vg = corne().layered(&l.holders().unwrap()).unwrap();
        assert!(l.flat(&corne()).is_ok() && vg.len() > 36);
    }

    #[test]
    fn small_board_without_thumbs() {
        let g = Geometry::from_toml(include_str!("../../../data/geometries/ansi-30.toml")).unwrap();
        let k = build(&g, &code(), &Spec::new(Mode::Alpha)).unwrap();
        assert_eq!(k.empty, 4);
        let k = build(&g, &code(), &Spec::new(Mode::Shift)).unwrap();
        assert_eq!(k.layout.shifted.len(), 4);
        let mut s = Spec::new(Mode::Alpha);
        s.keys = Some(vec![SPACE; 6]);
        assert!(build(&g, &code(), &s).is_err());
    }

    #[test]
    fn moving_a_hold_key_rehomes_stranded_layer_symbols() {
        let g = corne();
        let k = build(&g, &code(), &Spec { symbols: Some(10), ..Spec::new(Mode::Layer) }).unwrap();
        let l = k.layout;
        let from = l.holders().unwrap()[0].0;
        let to = (0..36)
            .find(|&i| g.key[i].finger.is_thumb() && !layout::is_fixed_key(l.chars[i]) && g.key[i].hand() != g.key[from].hand())
            .unwrap();
        let moved = move_fixed(&g, &l, from, to).unwrap().unwrap();
        assert!(moved.flat(&g).is_ok());
        let count = |l: &Layout| l.layers[0].chars.iter().filter(|&&c| c != EMPTY).count();
        assert_eq!(count(&moved), count(&l));
    }
}
