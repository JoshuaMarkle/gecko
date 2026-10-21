//! Atlas files in an explore run directory (`context/representation.md`):
//! `optima.bin` (`GKATLAS1`), `edges.bin` (`GKEDGES1`), `samples.bin` (`GKSAMPL1`), and the
//! `atlas.json` summary. Little-endian, fixed-size records after a header.

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// Record flag: the end of an exclusion descent (constrained, not necessarily a local optimum).
pub const CONSTRAINED: u16 = 1;

#[derive(Clone, Debug, PartialEq)]
pub struct Optimum {
    /// `sym[slot]` in the run's symbol ids (canonical if the atlas is mirror-symmetric).
    pub sym: Vec<u8>,
    /// Score under the run's profile.
    pub score: f64,
    /// Raw value of every metric in the atlas's metric list (weight-independent).
    pub raw: Vec<f32>,
    /// Descents that ended here.
    pub hits: u32,
    /// Active run time when first found.
    pub first_ms: u32,
    pub flags: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Edge {
    /// Row indices into the optima file.
    pub from: u32,
    pub to: u32,
    pub count: u32,
    /// `optimize::Kick` as u8.
    pub kick: u8,
    pub strength: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sample {
    pub score: f64,
    pub raw: Vec<f32>,
}

/// Parsed atlas binaries.
#[derive(Clone, Debug, Default)]
pub struct Atlas {
    pub slots: usize,
    pub metrics: Vec<String>,
    pub optima: Vec<Optimum>,
    pub edges: Vec<Edge>,
    pub samples: Vec<Sample>,
}

const OPT_MAGIC: &[u8; 8] = b"GKATLAS1";
const EDGE_MAGIC: &[u8; 8] = b"GKEDGES1";
const SAMPLE_MAGIC: &[u8; 8] = b"GKSAMPL1";

fn put_ids(out: &mut Vec<u8>, ids: &[String]) {
    for id in ids {
        let mut b = [0u8; 4];
        for (d, s) in b.iter_mut().zip(id.bytes()) {
            *d = s;
        }
        out.extend_from_slice(&b);
    }
}

pub fn write_optima(slots: usize, metrics: &[String], optima: &[Optimum]) -> Vec<u8> {
    let m = metrics.len();
    let mut out = Vec::with_capacity(28 + 4 * m + optima.len() * (18 + slots + 4 * m));
    out.extend_from_slice(OPT_MAGIC);
    for v in [1u32, slots as u32, m as u32, optima.len() as u32] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    put_ids(&mut out, metrics);
    for o in optima {
        out.extend_from_slice(&o.score.to_le_bytes());
        out.extend_from_slice(&o.hits.to_le_bytes());
        out.extend_from_slice(&o.first_ms.to_le_bytes());
        out.extend_from_slice(&o.flags.to_le_bytes());
        out.extend_from_slice(&o.sym);
        for &r in &o.raw {
            out.extend_from_slice(&r.to_le_bytes());
        }
    }
    out
}

pub fn write_edges(edges: &[Edge]) -> Vec<u8> {
    let mut out = Vec::with_capacity(12 + edges.len() * 14);
    out.extend_from_slice(EDGE_MAGIC);
    out.extend_from_slice(&(edges.len() as u32).to_le_bytes());
    for e in edges {
        for v in [e.from, e.to, e.count] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.extend_from_slice(&[e.kick, e.strength]);
    }
    out
}

pub fn write_samples(metrics: usize, samples: &[Sample]) -> Vec<u8> {
    let mut out = Vec::with_capacity(16 + samples.len() * (8 + 4 * metrics));
    out.extend_from_slice(SAMPLE_MAGIC);
    out.extend_from_slice(&(metrics as u32).to_le_bytes());
    out.extend_from_slice(&(samples.len() as u32).to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.score.to_le_bytes());
        for &r in &s.raw {
            out.extend_from_slice(&r.to_le_bytes());
        }
    }
    out
}

struct Reader<'a> {
    b: &'a [u8],
    at: usize,
    what: &'static str,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let s = self.b.get(self.at..self.at + n).ok_or_else(|| Error::invalid(self.what, "truncated file"))?;
        self.at += n;
        Ok(s)
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn f64(&mut self) -> Result<f64> {
        Ok(f64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn f32s(&mut self, n: usize) -> Result<Vec<f32>> {
        Ok(self.take(4 * n)?.as_chunks::<4>().0.iter().map(|c| f32::from_le_bytes(*c)).collect())
    }
    fn magic(&mut self, m: &[u8; 8]) -> Result<()> {
        if self.take(8)? != m {
            return Err(Error::invalid(self.what, format!("not a {} file", String::from_utf8_lossy(m))));
        }
        Ok(())
    }
}

/// Parse `optima.bin`; `limit` keeps only the first records (the file is sorted by score).
pub fn read_optima(b: &[u8], limit: Option<usize>) -> Result<(usize, Vec<String>, Vec<Optimum>)> {
    let mut r = Reader { b, at: 0, what: "optima.bin" };
    r.magic(OPT_MAGIC)?;
    let _version = r.u32()?;
    let (slots, m, count) = (r.u32()? as usize, r.u32()? as usize, r.u32()? as usize);
    let metrics: Vec<String> =
        (0..m).map(|_| r.take(4).map(|b| String::from_utf8_lossy(b).trim_end_matches('\0').to_string())).collect::<Result<_>>()?;
    let count = limit.map_or(count, |l| l.min(count));
    let mut optima = Vec::with_capacity(count);
    for _ in 0..count {
        let score = r.f64()?;
        let (hits, first_ms, flags) = (r.u32()?, r.u32()?, r.u16()?);
        let sym = r.take(slots)?.to_vec();
        let raw = r.f32s(m)?;
        optima.push(Optimum { sym, score, raw, hits, first_ms, flags });
    }
    Ok((slots, metrics, optima))
}

/// Parse `edges.bin`, keeping edges whose ends are both below `rows`.
pub fn read_edges(b: &[u8], rows: usize) -> Result<Vec<Edge>> {
    let mut r = Reader { b, at: 0, what: "edges.bin" };
    r.magic(EDGE_MAGIC)?;
    let n = r.u32()? as usize;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let (from, to, count) = (r.u32()?, r.u32()?, r.u32()?);
        let k = r.take(2)?;
        if (from as usize) < rows && (to as usize) < rows {
            out.push(Edge { from, to, count, kick: k[0], strength: k[1] });
        }
    }
    Ok(out)
}

pub fn read_samples(b: &[u8]) -> Result<Vec<Sample>> {
    let mut r = Reader { b, at: 0, what: "samples.bin" };
    r.magic(SAMPLE_MAGIC)?;
    let (m, n) = (r.u32()? as usize, r.u32()? as usize);
    (0..n).map(|_| Ok(Sample { score: r.f64()?, raw: r.f32s(m)? })).collect()
}

/// Write `bytes` to `path` atomically (temp file + rename) so readers never see half a file.
pub fn write_atomic(path: &std::path::Path, bytes: &[u8]) -> Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// `atlas.json`: counts and landscape statistics under the run's profile.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Summary {
    pub schema: u32,
    pub id: String,
    /// False while the run is still writing snapshots.
    pub done: bool,
    pub elapsed_ms: u64,
    pub evals: u64,
    /// Symbol id → char (the alphabet of every `sym` array).
    pub chars: String,
    pub slots: usize,
    pub metrics: Vec<String>,
    /// Optima were deduplicated up to the left/right mirror image.
    pub symmetric: bool,
    /// Profile parameters that shape the metric tables; re-scoring is exact only when they match.
    pub table_hash: String,
    pub counts: Counts,
    pub best: Option<Best>,
    pub probe: ProbeSummary,
    /// Fitness–distance correlation (Hamming to the best) over all optima.
    pub fdc: f64,
    /// Number of funnel bottoms (sinks of the best-improving-edge forest).
    pub funnels: usize,
    pub coverage: Coverage,
    /// Best layouts at least `distinct_dist` slots apart, best first.
    pub distinct_dist: usize,
    pub distinct: Vec<Best>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Counts {
    pub optima: usize,
    pub constrained: usize,
    /// Descents that ended in an archived optimum (incl. repeats).
    pub hits: u64,
    pub edges: usize,
    /// Optima dropped to respect `max_optima` (the worst are pruned).
    pub pruned: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Best {
    pub score: f64,
    pub chars: String,
    /// Hamming distance to the overall best.
    pub d_best: usize,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProbeSummary {
    pub samples: usize,
    pub random_mean: f64,
    pub random_sd: f64,
    pub random_min: f64,
    pub walks: usize,
    pub walk_len: usize,
    /// Autocorrelation of score along random swap walks, lag 0..
    pub rho: Vec<f64>,
    /// Correlation length ℓ = −1/ln ρ(1), in swaps.
    pub corr_length: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Coverage {
    /// Optima within this fraction of the best score are counted.
    pub within: f64,
    pub observed: usize,
    /// Chao1 estimate of how many such optima exist.
    pub estimate: f64,
}

/// Load an atlas from a run directory (`limit`: keep only the best optima).
pub fn load(dir: &std::path::Path, limit: Option<usize>) -> Result<(Summary, Atlas)> {
    let rd = |f: &str| {
        std::fs::read(dir.join(f)).map_err(|e| Error::invalid("atlas", format!("{}: {e} (not an explore run?)", dir.join(f).display())))
    };
    let summary: Summary = serde_json::from_slice(&rd("atlas.json")?).map_err(|e| Error::parse("atlas.json", e))?;
    let (slots, metrics, optima) = read_optima(&rd("optima.bin")?, limit)?;
    let edges = read_edges(&rd("edges.bin")?, optima.len())?;
    let samples = rd("samples.bin").ok().map(|b| read_samples(&b)).transpose()?.unwrap_or_default();
    Ok((summary, Atlas { slots, metrics, optima, edges, samples }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_roundtrip() {
        let metrics = vec!["B01".to_string(), "M02".to_string()];
        let optima = vec![
            Optimum { sym: vec![2, 0, 1], score: 1.5, raw: vec![0.25, 3.0], hits: 4, first_ms: 10, flags: 0 },
            Optimum { sym: vec![0, 2, 1], score: 2.5, raw: vec![1.25, -3.0], hits: 1, first_ms: 99, flags: CONSTRAINED },
        ];
        let (s, m, o) = read_optima(&write_optima(3, &metrics, &optima), None).unwrap();
        assert_eq!((s, m, o), (3, metrics.clone(), optima.clone()));
        assert_eq!(read_optima(&write_optima(3, &metrics, &optima), Some(1)).unwrap().2.len(), 1);
        let edges = vec![Edge { from: 0, to: 1, count: 3, kick: 1, strength: 3 }, Edge { from: 1, to: 5, count: 1, kick: 0, strength: 2 }];
        assert_eq!(read_edges(&write_edges(&edges), 2).unwrap(), vec![edges[0]]);
        let samples = vec![Sample { score: 9.0, raw: vec![1.0, 2.0] }];
        assert_eq!(read_samples(&write_samples(2, &samples)).unwrap(), samples);
        assert!(read_optima(b"nope", None).is_err());
    }
}
