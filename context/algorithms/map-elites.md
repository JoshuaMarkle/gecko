# MAP-Elites (quality-diversity map)

Spec and background: `research/studies/search-space-visualization.md` §4.2 (view 36), Mouret & Clune,
arXiv:1504.04909 (2015). Code: `gecko-core/src/explore/qd.rs`; CLI `gecko qd`.

**Idea:** instead of one winner, a grid over 1–3 *behaviour descriptors*; each cell keeps the
best layout with that behaviour. The user picks by preference ("more rolls", "less index
load") and sees what each choice costs.

**Descriptors** (`--axes`, each `name` or `name:lo:hi`):

| Name | Value |
|---|---|
| `roll_ratio` | 100 · (T03 + T04) / (T01 + T03 + T04): rolls vs alternation, % |
| `index_load` | % of keystrokes on the two index fingers |
| `pinky_load` | % of keystrokes on the two pinkies |
| `hand_balance` | % of keystrokes on the left hand (M04) |
| any metric id | its raw value (e.g. `B01`, `M05`, `T06`) |

Default `roll_ratio,index_load`, 16 bins each. Values outside the range go to the edge bins.

**Loop:**

1. **Seed** (`seeds` 64): the start layout plus random layouts, each descended to a local
   optimum. Axes without an explicit range get the seeds' span padded by half on each side
   (percent descriptors clamped to 0–100).
2. **Mutate** on every thread until the budget ends: pick a filled cell uniformly, apply a
   uniform `kick_min..=kick_max` (2–6) random swaps, run steepest descent (`descent` = 0: to
   a local optimum; N: at most N improving moves), measure (raw metric vector + finger loads,
   from scratch), and offer it to its cell: it wins an empty cell or a strictly better score.
3. **Snapshots** every `snapshot_ms` (atomic writes): `qd.json` + `elites.bin` (format in
   `representation.md`).

The grid is a mutex-guarded `Observer` (one lock per offspring, never in the swap loop). The
`MapElites` worker asks it for a parent via `Observer::parent`; without a grid (e.g. `bench`)
it falls back to random restarts with full descent.

**Parameters (`Params.qd`):** `axes`, `bins` 16, `seeds` 64, `kick_min` 2, `kick_max` 6,
`descent` 0, `snapshot_ms` 2000.

**Pitfalls:** automatic ranges depend on the seeds, so filled-cell counts and QD-scores are only
comparable between runs with the same ranges (pass `name:lo:hi` to compare). QD-score is
Σ (random_mean − score) over filled cells; random_mean comes from the seeds' random starts.
Elites carry raw metric vectors, so `stats::Weights` re-ranks them under another profile, but a
re-weighted elite is no longer guaranteed to be the best in its cell.

## Results

ansi-30, default profile, `english`, 20 threads (2026-10-21):

- 10 s, auto ranges (roll_ratio 36.9–77.3, index_load 14.0–42.8), seed 1: `descent` 0 →
  134–149 / 256 cells, best 15.7037 (the explore/SA/BLS best), mean elite 19.3; `descent` 3 →
  204 cells, mean elite 20.4, best only 16.01. Seeds 2–3 repeat the pattern (0: 95–116 cells,
  mean 18.0–18.6, always 15.7037; 3: 165–192 cells, mean 20.4–21.1, best 15.86–16.36; 8:
  89–146 cells, ≈ the full-descent numbers). Capped descent fills more cells with layouts that
  are not local optima; full descent keeps every elite a local optimum and is the default.
- 30 s, fixed `roll_ratio:35:80,index_load:14:44` (`runs/20261021-112749-qd-a0715323`): 138 /
  256 cells, 45 k offspring, QD-score 10 336, best 15.7037. The good region is wide: every
  roll ratio from 48 % to 66 % has an elite within 6.6 % of the best (index load 23–28 %), so
  roll-vs-alternation preference is nearly free in that band. Above 69 % rolls the elites
  switch to a different family (`ybdgfqouml intsw;aehr …`, 20–21). Index load below ~18 %
  costs ≥ 4 deciles; 30–40 % costs 1–2. Same budget with `descent` 3: 191 cells, best 15.80.
