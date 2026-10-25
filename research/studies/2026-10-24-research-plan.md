# Research plan: from the best 30-key layout to the best standard keyboard

Date: 2026-10-24. Status: plan; nothing below is run yet. Builds on the
[synthesis](../SYNTHESIS.md), the [open questions](../OPEN-QUESTIONS.md), the
[cost-function optimization](../experiments/2026-10-24-cost-function-optimization.md),
[Phase 2](../../context/evaluation.md) and the [English corpus audit](2026-10-24-english-corpus-audit.md).

## Where we stand

- **Search is solved for 30 keys.** BLS reaches the same optimum from 24–30 of 30 seeds;
  18-letter sub-problems are certified optimal. Phase 2 margins over the best known layouts are 6–17 %.
- **The cost function is evidence-fitted but thin in places.** Pair weights come from 49k QWERTY
  typists (split-half r 0.98). Comfort (E05, now the second-largest weight) rests on one study:
  61 people, 12 keys. Key effort and trigram context are the two largest unexplained parts
  (+0.17 CV R² from per-key effects; trigram classes explain 0.22 of a 0.92 ceiling).
- **Speed barely separates good layouts** (≤ 0.3 % per keystroke), so comfort, and how much it
  is worth, decides the winner.
- **The corpora support letter layouts, not full keyboards.** The audit found dropped curly
  quotes, a biased Backspace/navigation sampler, and novels that are one source counted four times.
- **The ultimate target is not modelled yet.** Every result is on 30 keys (`ansi-30`) or split
  boards. No geometry has the standard board's number row and right-hand punctuation keys, and no
  study has placed digits, Shift pairs and all ASCII symbols together on a row-stagger board.

The plan therefore has one main line (W0 → W1 → W2 → W3 → W5) and side lines (W4, W6, W7) that
feed it or answer the alternate objectives.

## W0 · Define the target (first, small)

**Question.** What exactly is "the best layout for a standard keyboard with standard symbols"?

- Build `ansi-47`: the ANSI typing area (number row 13, top 13, home 11, bottom 10), Space bar
  and modifiers off the modelled keys as on `ansi-30`. Efforts and finger map from the
  `ansi-30` conventions; number-row and far-right keys need new effort values (see W2.2).
- Write the constraints as an explicit, versioned **target spec** (`context/keysets.md`):
  1. which symbols must be on the base/Shift layers (all printable ASCII: 26 letters, 10 digits,
     32 punctuation);
  2. whether Shift pairs are free (`--pairs frequency`) or US-fixed (`1!`, `[{`, …);
  3. whether digits stay in order on the number row;
  4. whether Ctrl shortcuts (`z x c v`) and Vim `hjkl` are pinned or merely costed.
- Run each variant once (BLS, 25 M+) to see how much each constraint costs. That table is the
  first result and tells the user which constraints are worth keeping.

**Decisions for the user** (the only ones in this plan that change the goal): the constraint set
above. Default proposal: free letters and punctuation, US Shift pairs, digits in order, shortcuts
costed but not pinned.

## W1 · Corpus foundation (blocks every full-board claim)

From the audit; the order matters because later studies rescore against these.

1. **Repairs.** Normalize curly quotes and other Unicode punctuation; fix the edit and navigation
   samplers (RNG inside a binary-search predicate); keep case through synthetic streams. Version
   the corpus format; keep old artifacts for replay.
2. **Source-separated English suite.** MASC written slices, OANC (check overlap), contemporary
   informational text, email/chat. Keep document IDs, case, punctuation and paragraph breaks.
   Split train/dev/test by document and source; hold one source out as a final test.
3. **Task corpora** (each with provenance and a held-out split):
   - *Programming:* per language (Python, JS/TS, Rust, C, Go, shell, Markdown/LaTeX) from a
     permissively licensed sample (e.g. The Stack v2 subset). Text is not typing: strip
     auto-indent, decide what an editor auto-closes (brackets, quotes) and model it as a variant.
   - *Vim / editor:* the hard one, because commands are not text. Candidates: keystroke logs
     (the user's own, via a logging plugin, as an n-of-1 corpus), VimGolf solutions (biased to
     golf), and a synthetic command model (motion/operator frequencies from published usage
     stats). Treat as a mixture component, and report how the optimum moves with its share.
   - *Shortcuts and shell:* Ctrl-shortcut frequencies (currently an assumption, 0.01 per key),
     shell histories.
4. **Mixture and stopping rule.** Fix plausible mixtures (prose / code / editor) before looking
   at results. Stop adding data when a new independent source no longer changes the
   recommended layout and every domain loss stays inside a predeclared tolerance.

**Done when:** every corpus has provenance, splits and a dedup report; finalists from Phase 2 and
the cost study are rescored on the new suite before any new search.

## W2 · Cost function, second round

Ordered by expected effect on the optimum per unit of effort.

1. **Comfort on all keys (largest gap).** A pairwise comfort study over all 30 keys (then the
   number row), including center columns, SFS and cross-hand pairs, in the web UI (#50 design).
   Active pair selection (only pairs that move current optima); Bradley–Terry fit; pilot with the
   user, then recruit (cmini/Discord). Answers the open SFS question and whether E05 deserves its weight.
2. **Key effort from timing, separated from letter.** Fit per-key effects with designs that break
   the key/letter confound: QWERTZ/AZERTY `y/z`, `a/q`, `m` swaps from the Aalto data, and a
   letter-frequency control. Extend to the number row, Shift (capitals are in the Aalto
   sentences) and the right-hand punctuation keys, which W0 needs.
3. **Trigram and longer context.** The unmodelled 0.70 of trigram variance: try richer features
   (finger triples, row patterns, same-key repeats like `ere`) and a small learned model, used
   as a judge, never as the objective (practice confound).
4. **Non-QWERTY evidence.** Find timing from Colemak/Dvorak/modern-layout typists (Monkeytype or
   keybr exports, emg2qwerty, community typing logs). Even a few hundred typists would test
   whether the fitted coefficients transfer beyond QWERTY practice.
5. **Costs not modelled yet:** Shift chord cost per hand and finger, number-row reach, pinky
   load on the far-right keys, learning cost (L02) as an explicit user weight, finger
   fatigue over long sessions (literature says typing is ~10 % MVC; check for symbol-heavy work).
6. **Keep the method:** every new weight is judged by what its *optimum* is like on held-out
   judges (bilevel, winner's-curse re-runs), as in the 2026-10-24 study. Retire S01/S02/T01/T06/M05
   only after (1).

## W3 · Search on the full board

1. **Scaling.** 47 keys with Shift pairs and digits: does BLS still converge across seeds? Measure
   seeds-at-best vs budget as in Phase 2. Add adaptive fidelity (#55) if the trigram table makes
   evaluation slow.
2. **Bounds.** Certify sub-problems (letters fixed, punctuation free; number row alone) with
   `gecko certify`; the whole-board bound is useless while reward terms are negative.
3. **Landscape (paper 01).** Weight sweeps and the optima archive on the full board: consensus
   keyboard, contested keys, regret matrix. Expect letters to be stable and punctuation to be
   the contested part.
4. **Near-optimal set, not one layout.** When the differences between top layouts are below the
   uncertainty from W1 (corpus) and W2 (weights), report the set and what separates them.

## W4 · Task tuning (programming, Vim, English)

**Question.** How much does one general layout lose against task-specific optima, and which keys move?

- Optimize per task (english, each language, editor mix) and build the **regret matrix**: the
  optimum for task *i* scored on task *j*. Small off-diagonal regret means one layout suffices.
- Find the **stable core**: keys that agree across tasks, and the few that move (brackets,
  operators, `;`, `:`). A task variant can then be the general layout plus a small diff.
- Vim specifically: compare costing `hjkl`/operators as a key stream vs pinning them (mnemonics
  and muscle memory matter more than motor cost for commands); report the price of each.
- Programming: auto-closing editors halve closing-bracket frequency; report both.

## W5 · Validate with people

No result is "best" until typists confirm part of it. In order of cost:

1. Rescore with every independent judge available (time halves, Engram, new comfort data).
2. **n-of-1 learning log.** The user (and volunteers) learn the W3 winner; log keystrokes
   (keybr/Monkeytype + local logger) for speed, error rate and comfort ratings over weeks.
   Preregister what counts as success.
3. **Comfort confirmation.** Short pairwise or word-level comfort trials on the winner's most
   different n-grams vs the best known layout.

## W6 · Modern layout analysis (feeds W0 and W2)

- Extend the [layout ranking](../../docs/papers/layout-ranking/layout-ranking.md) to full-board
  layouts that define digits and symbols (Programmer Dvorak, Neo, Bépo, Workman-P, Engram full,
  Hands Down variants) and to adaptive/magic layouts on equal terms.
- **Community agreement (T11):** preregistered test of which metrics predict cmini likes and
  survival, with confidence intervals.
- Explain differences: per-metric fingerprints show *what* each design traded, which checks
  whether our weights miss something the community values (e.g. inward rolls, which the
  current profile slightly penalizes).
- Non-English: score modern layouts on French/German corpora (`french.gkc` exists) to test
  whether the cost function, not just the corpus, carries over.

## W7 · Physical boards (alternate objective)

Lower priority until W0–W3 give the standard-keyboard answer; most tooling exists.

- **Hand-size robustness (T7):** ANSUR II (download approved) → hand presets; score layouts and
  boards across the 5th–95th percentile; find designs that lose little everywhere.
- **Geometry co-design (T6):** which board features (stagger, splay, thumb arc, tent) buy the
  most; bilevel with the W2 cost function.
- Thumb-key and switch-force data are thin in the literature (open questions); only pursue if a
  design decision depends on them.

## Order and milestones

| Step | Work | Output | Gate |
|---|---|---|---|
| 1 | W0 geometry + target spec; W1.1 repairs | `ansi-47`, constraint-cost table, repaired corpora | user picks constraints |
| 2 | W1.2–1.3 corpora; W2.2 key effort incl. number row and Shift | source-split suite, task corpora, timing effort table | finalists rescored |
| 3 | W3.1–3.2 full-board search | first full-board optimum, seed agreement, sub-problem certificates | converges across seeds |
| 4 | W2.1 comfort study (pilot → recruited) | all-key comfort table, SFS answer | ≥ pilot data |
| 5 | W4 task regret matrix; W6 full-board ranking | general layout + task diffs; modern layouts placed | |
| 6 | W2 re-fit, W3.3–3.4 landscape and near-optimal set | the recommended layout (or set) with uncertainty | stopping rule met |
| 7 | W5 human validation; papers 01–03 | learning log, comfort confirmation | preregistered |
| — | W7 in parallel when CPU is idle | hand-size and geometry results | |

## Risks

- **Goodhart.** More metrics means more to game. Every optimum is checked on judges it never saw.
- **QWERTY practice bias** in all timing data; W2.4 is the only direct test.
- **Comfort recruitment.** If the study gets few participants, keep E05's weight where it is and
  report the result conditional on it.
- **Vim data** may stay synthetic; then report the Vim variant as a sensitivity, not a claim.
