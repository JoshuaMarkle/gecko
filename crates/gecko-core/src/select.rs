//! B9 rank-sum selection over many runs (Kühlmak): gather the layouts of optimize runs (ranked
//! top-K and Pareto fronts), explore atlases (best optima) and QD grids (elites), score each
//! on one board / corpus / profile, rank every layout per metric and sort by the sum of its
//! ranks. A layout that is good on everything beats one that wins a single metric, whatever
//! the profile's weights. See `context/cli.md` (`gecko select`).

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;

use crate::explore::{atlas, qd};
use crate::magic::Magic;
use crate::metrics::{self, Dir};
use crate::report::breakdown;
use crate::{hash, run, Corpus, Error, Geometry, Layout, Profile, Result, Scorer};

/// A layout found by one or more runs.
#[derive(Clone, Debug)]
pub struct Candidate {
    /// Slot-ordered symbols on the run's board (layered layouts flat).
    pub chars: String,
    /// Where it came from: `<run id>#<rank>` (optimize), `#o<row>` (atlas), `#e<row>` (qd).
    pub sources: Vec<String>,
    /// Board the chars are laid out on.
    pub geometry: String,
    pub magic: Vec<Magic>,
}

/// What one run contributed.
#[derive(Clone, Debug, Serialize)]
pub struct RunSource {
    pub id: String,
    /// `optimize`, `pareto`, `atlas` or `qd`.
    pub kind: String,
    pub geometry: String,
    pub corpus: String,
    pub profile: String,
    pub layouts: usize,
}

/// The layouts of `ids` (directories under `runs`), at most `pool` per atlas / qd run (best
/// first). Layouts found by several runs are merged (same board and chars).
pub fn gather(runs: &Path, ids: &[String], pool: usize) -> Result<(Vec<Candidate>, Vec<RunSource>)> {
    let mut out: Vec<Candidate> = Vec::new();
    let mut index: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut sources = Vec::new();
    for id in ids {
        let id = id.strip_prefix("run:").unwrap_or(id);
        if id.is_empty() || id.contains(['/', '\\']) || id.contains("..") {
            return Err(Error::invalid("select", format!("bad run id '{id}'")));
        }
        let dir = runs.join(id);
        if crate::geomsearch::is_run(&dir) {
            return Err(Error::invalid("select", format!("{id} is a geometry run (its layouts are on different boards)")));
        }
        let cfg = run::load_config(&dir)?;
        let mut found: Vec<(String, String)> = Vec::new();
        let kind = if dir.join("atlas.json").is_file() {
            let (s, at) = atlas::load(&dir, None)?;
            let alpha: Vec<char> = s.chars.chars().collect();
            let rows = at.optima.iter().enumerate().filter(|(_, o)| o.flags & atlas::CONSTRAINED == 0).take(pool);
            found.extend(rows.map(|(r, o)| (format!("{id}#o{r}"), o.sym.iter().map(|&x| alpha[x as usize]).collect())));
            "atlas"
        } else if dir.join("qd.json").is_file() {
            let q = qd::load(&dir)?;
            found.extend(q.elites.iter().take(pool).map(|e| (format!("{id}#e{}", e.row), e.chars.clone())));
            "qd"
        } else {
            let res = run::load_result(&dir)?;
            found.extend(res.best.iter().map(|l| (format!("{id}#{}", l.rank), l.chars.clone())));
            found.extend(res.front.iter().map(|p| (format!("{id}#p{}", p.layout.rank), p.layout.chars.clone())));
            if res.front.is_empty() {
                "optimize"
            } else {
                "pareto"
            }
        };
        sources.push(RunSource {
            id: id.to_string(),
            kind: kind.into(),
            geometry: cfg.geometry.name.clone(),
            corpus: cfg.corpus.name.clone(),
            profile: cfg.profile.name.clone(),
            layouts: found.len(),
        });
        for (src, chars) in found {
            let key = (cfg.geometry.name.clone(), chars.clone());
            match index.get(&key) {
                Some(&i) => out[i].sources.push(src),
                None => {
                    index.insert(key, out.len());
                    out.push(Candidate { chars, sources: vec![src], geometry: cfg.geometry.name.clone(), magic: cfg.magic.clone() });
                }
            }
        }
    }
    Ok((out, sources))
}

/// Average 1-based ranks of `v` (lower value = better rank; ties share the mean rank).
pub fn ranks(v: &[f64]) -> Vec<f64> {
    let mut order: Vec<usize> = (0..v.len()).collect();
    order.sort_by(|&a, &b| v[a].total_cmp(&v[b]));
    let mut out = vec![0.0; v.len()];
    let mut i = 0;
    while i < order.len() {
        let mut j = i;
        while j + 1 < order.len() && v[order[j + 1]] == v[order[i]] {
            j += 1;
        }
        let r = (i + j) as f64 / 2.0 + 1.0;
        for &k in &order[i..=j] {
            out[k] = r;
        }
        i = j + 1;
    }
    out
}

#[derive(Clone, Debug, Serialize)]
pub struct MetricCol {
    pub id: &'static str,
    pub name: &'static str,
    pub unit: &'static str,
    /// `min` or `max`: which way is better.
    pub dir: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct Selected {
    pub rank: usize,
    pub rank_sum: f64,
    /// Rank per metric (1 = best), aligned with `metrics`.
    pub ranks: Vec<f64>,
    /// Metric value per metric in natural units, aligned with `metrics`.
    pub values: Vec<f64>,
    /// Weighted score under the profile (tie-break).
    pub score: f64,
    pub chars: String,
    pub rows: Vec<String>,
    pub hash: String,
    pub sources: Vec<String>,
}

/// Metrics to rank on: `ids`, or every metric the profile weights. Informational metrics fail.
pub fn metric_cols(ids: &[String], profile: &Profile) -> Result<Vec<&'static metrics::MetricDef>> {
    let defs: Vec<&metrics::MetricDef> = if ids.is_empty() {
        metrics::METRICS.iter().filter(|d| profile.weight(d.id) > 0.0 && d.dir != Dir::Info).collect()
    } else {
        ids.iter()
            .map(|i| metrics::def(i).ok_or_else(|| Error::invalid("select", format!("unknown metric id '{i}'"))))
            .collect::<Result<_>>()?
    };
    if let Some(d) = defs.iter().find(|d| d.dir == Dir::Info) {
        return Err(Error::invalid("select", format!("{} is informational (no better direction)", d.id)));
    }
    if defs.is_empty() {
        return Err(Error::invalid("select", "no metrics to rank on (the profile weights none; pass --metrics)"));
    }
    Ok(defs)
}

/// Score every candidate on `g` (laid out on its own board `boards[name]`, then fitted to
/// `g`), rank per metric and sort by rank sum, then score, then chars.
pub fn select(
    cands: &[Candidate],
    boards: &BTreeMap<String, Geometry>,
    g: &Geometry,
    corpus: &Corpus,
    profile: &Profile,
    defs: &[&'static metrics::MetricDef],
) -> Result<(Vec<MetricCol>, Vec<Selected>)> {
    // One scorer per symbol set (and adaptive-key rules); other layouts with the same symbols
    // are states of it.
    let mut scorers: BTreeMap<(String, String), Scorer> = BTreeMap::new();
    let idx: Vec<usize> = defs.iter().map(|d| metrics::index_of(d.id).expect("known metric")).collect();
    let mut rows = Vec::with_capacity(cands.len());
    for c in cands {
        let board = boards.get(&c.geometry).ok_or_else(|| Error::invalid("select", format!("board '{}' not loaded", c.geometry)))?;
        let layout = Layout::from_flat(&c.sources[0], board, c.chars.chars().collect())?.with_magic(c.magic.clone())?.fit(g)?;
        let mut set = layout.chars.clone();
        set.extend(layout.layers.iter().flat_map(|l| l.chars.iter().copied()));
        set.sort_unstable();
        let key = (
            set.into_iter().collect::<String>() + &layout.layers.len().to_string(),
            serde_json::to_string(&layout.magic).unwrap_or_default(),
        );
        let st = match scorers.get(&key) {
            Some(sc) if layout.layers.is_empty() && sc.n == layout.chars.len() => {
                let mut free: BTreeMap<char, Vec<u8>> = BTreeMap::new();
                for (i, &ch) in sc.chars.iter().enumerate().rev() {
                    free.entry(ch).or_default().push(i as u8);
                }
                let sym: Vec<u8> = layout.chars.iter().map(|ch| free.get_mut(ch).and_then(|v| v.pop()).expect("same symbols")).collect();
                sc.state_from_sym(sym)
            }
            _ => {
                let sc = Scorer::new(g, corpus, profile, &layout)?;
                let st = sc.identity();
                scorers.insert(key.clone(), sc);
                st
            }
        };
        let sc = &scorers[&key];
        let b = breakdown(sc, &st);
        let values: Vec<f64> = idx.iter().map(|&m| b.metrics[m].value).collect();
        let flat = st.chars(sc);
        let shown = Layout::from_flat("", g, flat.clone())?.with_magic(layout.magic.clone())?;
        rows.push(Selected {
            rank: 0,
            rank_sum: 0.0,
            ranks: Vec::new(),
            values,
            score: b.score,
            hash: hash::short(shown.hash()),
            rows: shown.all_rows(g),
            chars: flat.iter().collect(),
            sources: c.sources.clone(),
        });
    }
    for (m, d) in defs.iter().enumerate() {
        let sign = if d.dir == Dir::Max { -1.0 } else { 1.0 };
        let r = ranks(&rows.iter().map(|s| sign * s.values[m]).collect::<Vec<_>>());
        for (s, r) in rows.iter_mut().zip(r) {
            s.ranks.push(r);
            s.rank_sum += r;
        }
    }
    rows.sort_by(|a, b| a.rank_sum.total_cmp(&b.rank_sum).then(a.score.total_cmp(&b.score)).then_with(|| a.chars.cmp(&b.chars)));
    for (i, s) in rows.iter_mut().enumerate() {
        s.rank = i + 1;
    }
    let cols = defs
        .iter()
        .map(|d| MetricCol { id: d.id, name: d.name, unit: d.unit, dir: if d.dir == Dir::Max { "max" } else { "min" } })
        .collect();
    Ok((cols, rows))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ties_share_the_mean_rank() {
        assert_eq!(ranks(&[3.0, 1.0, 2.0]), [3.0, 1.0, 2.0]);
        assert_eq!(ranks(&[1.0, 1.0, 5.0, 0.0]), [2.5, 2.5, 4.0, 1.0]);
        assert!(ranks(&[]).is_empty());
    }
}
