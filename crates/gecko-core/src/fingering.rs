//! C11 alternative fingering: which finger presses a key is a search variable. Keys list other
//! fingers that may press them (`alt` in the board file); a descent reassigns one key at a
//! time, re-costing the scorer incrementally ([`Scorer::set_geometry`]) and re-optimizing the
//! layout from the current one for each candidate. See `context/objective.md` (Fingering search).

use serde::{Deserialize, Serialize};

use crate::geomsearch::inner_search;
use crate::rng::Rng;
use crate::scorer::State;
use crate::{Finger, Geometry, Result, Scorer};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct FingeringParams {
    /// Search the board's alternative fingers after the layout search.
    pub search: bool,
    /// SA swaps of the warm-started layout search on each candidate fingering (then a steepest
    /// descent). The given fingering gets the same search, so candidates compete fairly.
    pub evals: u64,
}

impl Default for FingeringParams {
    fn default() -> Self {
        FingeringParams { search: false, evals: 300_000 }
    }
}

impl FingeringParams {
    pub fn is_off(&self) -> bool {
        *self == FingeringParams::default()
    }
}

/// Inner SA acceptance at the start: low, so it refines the warm start (as in `geomsearch`).
const ACCEPT0: f64 = 0.05;

/// A key typed by another finger than the board file gives it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Change {
    pub key: usize,
    pub row: i8,
    pub col: i8,
    pub from: Finger,
    pub to: Finger,
    pub effort: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FingeringReport {
    /// Option chosen per key ([`Geometry::finger_options`]; 0 = the board file's finger).
    pub choice: Vec<u8>,
    pub changes: Vec<Change>,
    /// Score of the best layout on the given fingering (after the same warm-started search)
    /// and on the chosen one.
    pub base_score: f64,
    pub score: f64,
    /// Fingerings tried and reassignments accepted.
    pub candidates: u64,
    pub steps: u64,
}

pub struct Outcome {
    pub report: FingeringReport,
    /// The board with the chosen fingering, its scorer and the best layout on it.
    pub board: Geometry,
    pub scorer: Scorer,
    pub best: State,
}

/// Keys of `board` with alternative fingers.
pub fn has_alternatives(board: &Geometry) -> bool {
    board.key.iter().any(|k| k.layer == 0 && !k.alt.is_empty())
}

/// Best-improvement descent over single-key finger reassignments, starting from the layout
/// `sym` on `scorer` (built on `board`). Every candidate's layout is re-optimized from the
/// current best layout, so a reassignment wins only with a layout that suits it. Stops when no
/// reassignment improves the score. Candidates whose slot structure changes (a hold layer
/// drops its finger's keys) are skipped.
pub fn search(scorer: &Scorer, board: &Geometry, movable: &[usize], sym: &[u8], p: &FingeringParams, rng: &mut Rng) -> Result<Outcome> {
    let mut choice = vec![0u8; board.len()];
    let mut cur_sc = scorer.clone();
    let mut cur = inner_search(&cur_sc, movable, sym, p.evals, ACCEPT0, rng);
    let base_score = cur.score;
    let (mut candidates, mut steps) = (0, 0);
    loop {
        let mut best: Option<(Vec<u8>, Scorer, State)> = None;
        for i in (0..board.len()).filter(|&i| board.key[i].layer == 0 && !board.key[i].alt.is_empty()) {
            for o in 0..=board.key[i].alt.len() as u8 {
                if o == choice[i] {
                    continue;
                }
                let mut c = choice.clone();
                c[i] = o;
                let Ok(sc) = cur_sc.with_geometry(&board.with_fingering(&c)?) else { continue };
                let st = inner_search(&sc, movable, &cur.sym, p.evals, ACCEPT0, rng);
                candidates += 1;
                if st.score < best.as_ref().map_or(cur.score - 1e-9, |b| b.2.score) {
                    best = Some((c, sc, st));
                }
            }
        }
        let Some((c, sc, st)) = best else { break };
        (choice, cur_sc, cur) = (c, sc, st);
        steps += 1;
    }
    let chosen = board.with_fingering(&choice)?;
    let changes = (0..board.len())
        .filter(|&i| choice[i] != 0)
        .map(|i| {
            let k = &chosen.key[i];
            Change { key: i, row: k.row, col: k.col, from: board.key[i].finger, to: k.finger, effort: k.effort }
        })
        .collect();
    let report = FingeringReport { choice, changes, base_score, score: cur.score, candidates, steps };
    Ok(Outcome { report, board: chosen, scorer: cur_sc, best: cur })
}
