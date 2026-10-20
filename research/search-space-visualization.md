# Search-space exploration and visualization

How GECKO can show the layout search space, show what its optimizers are doing, find what
they miss, and switch from "optimize one layout" to "explore and explain the space".
Written 2026-10-20. Builds on [SYNTHESIS.md](SYNTHESIS.md) §5 (search) and the sources in
[`sources/`](sources/README.md), mainly [qap-metaheuristics](sources/qap-metaheuristics.md)
and [search-methods-general](sources/search-methods-general.md).

## 0. Summary

- **Why this matters now.** SA reaches the same best layout (8.396) from every seed in 10–30 s
  (`context/algorithms/simulated-annealing.md`). Finding *the* optimum of one profile is
  solved. What is not solved: how many other layouts are almost as good, how different they
  are, which choices are forced and which are arbitrary, and how much the winner depends on
  weights the project chose by hand. Those are data-analysis questions, and visualization
  answers them.
- **The key design idea: archive once, re-weight forever.** Every local optimum the search
  finds is stored with its full per-metric raw vector, not only its score. Because the score is
  a weighted sum of those raw values (plus a few nonlinear terms that are also stored), the UI
  can re-rank the whole archive under new weights in milliseconds, with no new search. Most
  views below are views of this archive.
- **What to build first** (§7): (1) a trace + optima archive written by the existing runner,
  (2) an `explore` algorithm that samples a local optima network with Breakout-style kicks,
  (3) a web **Explore** view with five linked panels: embedding map, fitness–distance plot,
  consensus keyboard, trade-off scatter, and the optimum's breakdown.
- **What we are probably missing** (§5): near-optimal alternatives far from the best,
  the effect of pins and symmetry, whether the default profile is being exploited
  (a "too good" score of 8.4 vs Semimak 11.85 is a warning sign), weight sensitivity, and any
  certificate of how close to optimal we are.

## 1. The space, in numbers

| Quantity | Value (ansi-30, 30 movable symbols) | Consequence |
|---|---|---|
| Layouts | 30! ≈ 2.65 × 10³² | Cannot enumerate; sample and model |
| Swap neighbours per layout | C(30,2) = 435 | A full neighbourhood scan is cheap (≈ 1 ms with trigrams) |
| Max swap distance between layouts | 29 (Cayley distance) | Distances are small integers: easy to histogram |
| Hamming distance (slots that differ) | 0, 2..30 | Natural "how different" for users: "7 keys moved" |
| Delta evals/s | ≈ 400k/thread with trigrams, 3M without | 20 threads ≈ 8M/s: millions of local optima per hour |
| Mirror symmetry (left/right hand) | 2× (if geometry and profile are symmetric) | Canonicalize before comparing optima |

Distances to use (all cheap, O(S)):

- **Hamming** `d_H` = number of slots holding different symbols. Default for every plot.
- **Swap (Cayley) distance** = S − cycles(π₁⁻¹∘π₂). Minimum swaps to go from one to the other;
  matches the search's move. Use for LON edges and "path length".
- **Finger distance** = number of symbols typed by a different finger. Two layouts with the
  same letter→finger assignment feel alike even if rows differ; this groups them.
- **Hand distance** = symbols on a different hand. Coarsest, explains alternation profiles.
- **Feature distance** = Euclidean distance between standardized metric vectors. "Do they
  *type* alike?" regardless of where letters are.

Showing the same archive under each of these distances answers different questions; let the
user switch.

## 2. What to record (telemetry)

Visualizations need data the runner does not keep today (`Progress` has only elapsed, evals,
best score, best chars). Record four streams. None of them touches the inner swap loop.

| Stream | Content | Where it comes from | Rate / size |
|---|---|---|---|
| **trace** | per worker: elapsed, evals, current score, best score, temperature, accepted / improving / worsening counts since last record, current layout (32 B) | SA chunk boundary (every 256 steps), GA generation, HC descent step | every ~10–50 ms per worker; ~1 MB/min |
| **optima** | each local optimum (no improving swap): canonical sym, score, raw metric vector, nonlinear accumulators, start-layout hash, worker, time, evals-to-reach, number of times hit | after `steepest_descent` (SA restart end, HC, explore kicks) | ~100–200 B each; dedupe by hash and count hits |
| **edges** | optimum A → optimum B, kick type and strength, Δscore | explore/ILS/BLS kicks | 16–24 B each |
| **samples** | random layouts and random-walk sequences with score + metric vector | landscape probes (§3.2) | thousands, one-off per (geometry, corpus, profile) |

The archive ("atlas") is keyed by the scorer's input hashes (geometry, corpus, profile *without
weights*, pins). Weights are deliberately not part of the key because the raw metric vectors
are weight-independent; an archive can be re-scored under any profile with the same metric set.

## 3. Visualization catalog

Grouped by the question each answers. **Bold** = recommended for the first version. Each entry
lists the data it needs (stream from §2). All of them fit the design system: monochrome chrome,
data carries the color, one accent for the selected/best item (`context/design.md`).

### 3.1 How is this run going? (process views)

1. **Multi-worker convergence** (trace). Today's `Convergence` chart, but one thin line per
   worker for the *current* score plus the bold best-so-far line. Shows exploration (current
   far above best) vs exploitation, and whether workers agree.
2. **Restart outcome strip** (optima). One dot per restart/descent at (time, final score);
   y-jitter by worker. A horizontal band of dots at the same score = the search keeps falling
   into one basin. Mark the global best in accent.
3. **Time-to-target ECDF / run-length distribution** (optima across seeds). Fraction of
   runs that reached score ≤ target by time t, for several targets (best, best + 0.5 %, + 1 %).
   The standard way to compare stochastic optimizers (Hoos & Stützle, RTDs); replaces
   "mean ± stdev" in `gecko bench`.
4. **Temperature and acceptance panel** (trace). T(t), acceptance rate of worsening moves,
   and fraction of improving moves on one shared time axis. Diagnoses a bad schedule at a
   glance: acceptance should go from ~accept0 to ~0; a long flat tail at 0 = wasted budget.
5. **Delta histogram over time** (trace, sampled deltas). Ridge plot of the distribution of
   proposed Δ per time window. Shows how rugged the neighbourhood is around current states.
6. **Metric trajectories** (trace + breakdown at checkpoints). Stacked area of weighted
   metric contributions of the best-so-far layout over time. Shows *what the optimizer is
   buying*: e.g. SFB falls first, then trigram terms trade against each other.
7. **Layout raster / "symbol migration"** (trace). Matrix with time on x, slot on y, cell
   = symbol (colored by its final finger). Long stable bands = settled keys; flicker =
   keys still being decided. Reads like a piano roll; it is the single best "watch the
   search think" view.
8. **Settling order** (trace). For each symbol, the last time it moved in the best-so-far
   layout. Sorted bar chart: which letters are decided early (e, t, space-adjacent vowels),
   which last (q, z, punctuation). Late-settling keys are where the objective is flat.
9. **Swap-frequency matrix** (trace counters). S×S heatmap of accepted swaps by symbol pair.
   Hot cells reveal symbols the search keeps exchanging = near-ties.
10. **Distance-to-final river plot** (trace). Hamming distance from each worker's current
    layout to the final best, vs time. Shows when each worker "found the basin".

### 3.2 What does the landscape look like? (structure views)

11. **Fitness–distance plot (FDC)** (optima). Scatter of every local optimum: x = distance to
    the best-known layout, y = score. Report Pearson FDC (Jones & Forrest 1995). Strong
    positive correlation = "big valley": good optima cluster near the best and restarts from
    good solutions pay off. Weak/negative = deceptive; diversity methods pay off. This is the
    classic QAP diagnostic (Merz & Freisleben 2000). Overlay reference layouts (Semimak,
    Graphite, Colemak) as labelled marks.
12. **Random-walk autocorrelation (correlogram)** (samples). ρ(k) of score along random swap
    walks; correlation length ℓ = −1/ln ρ(1) (Weinberger 1990). Gives a number for
    ruggedness and a principled kick strength for ILS (kick ≈ ℓ swaps). Compare profiles: a
    profile with trigrams on vs off, or carpalx vs default, has a different ℓ.
13. **Score distribution of random layouts vs local optima** (samples, optima). Two histograms
    on one axis. Where QWERTY and each reference layout sit as percentiles. Answers "how good
    is good?".
14. **Density of states** (Wang–Landau or replica exchange samples). Estimated log count of
    layouts at each score, down into the tail. Gives "1 in 10ⁿ layouts is better than
    Semimak under this profile", which random sampling cannot reach. Wang & Landau 2001;
    works on permutations unchanged.
15. **Local optima network (LON)** (optima + edges). Nodes = optima (size = times hit ≈
    basin size, color = score), edges = escape transitions from kicks. Layout: force-directed,
    or y = score ("landscape" layout). Funnels show up as trees draining into one sink; multiple
    sinks = multiple funnels = where single-start search fails. Daolio, Verel, Ochoa,
    Tomassini 2010 built LONs for QAP and found real-like instances form modular,
    clustered networks; keyboard instances are real-like. Compute and show: number of sinks,
    funnel depth, share of escapes that improve, community structure (Louvain).
16. **Compressed LON / funnel diagram.** Merge optima on plateaus and contract to funnels;
    one bar per funnel: best score, basin share, distance between funnel bottoms. The
    1-chart answer to "is there more than one good design?".
17. **Disconnectivity graph** (optima + barrier estimates). Tree from statistical physics
    (Becker & Karplus 1997): leaves = optima at their score; branches join at the lowest
    barrier score at which two optima become connected. Barrier = worst score on the best
    path found between them (estimate with path relinking, §4). Shows *how high* you must
    climb to move between designs.
18. **Search trajectory network (STN)** (trace, coarse-grained). Ochoa, Malan, Blum 2021:
    nodes = locations (layouts, or clusters of layouts) visited by a run, edges = consecutive
    visits; one color per algorithm. Overlaying SA, GA, tabu, explore on one graph shows
    which algorithms explore where and where they converge. Built from data the runs
    produce anyway, no extra sampling.
19. **Neighbourhood profile of one layout** (one 435-swap scan). Sorted Δ of all swaps:
    how many neutral or near-neutral swaps exist (plateau width), how steep the walls are.
    A "near-neutral swap count" ≥ a few dozen means the user has free choices.
20. **Swap-delta matrix** (one scan). S×S heatmap, cell = Δscore for swapping symbols i,j
    on the current layout. Blue = improving, ink = neutral, red = costly. The most useful
    *editor* view: it tells the user which hand edits are free before they make them.
    Also drawn on the keyboard: click a key, every other key is tinted by the cost of
    swapping with it.
21. **Per-slot sensitivity / "importance" keyboard** (one scan). For each slot, min and
    median Δ of swapping its symbol away. Overlay on the 3D board: keys that matter glow,
    keys that do not are flat. Explains a layout better than any score.
22. **Plateau / neutral network size** (neutral walks). Random walks restricted to
    |Δ| < ε from an optimum; count distinct layouts reached. Reports how many near-copies
    of a layout exist with "the same" score.
23. **Basin-of-attraction estimate** (optima hit counts). Hits per optimum from random starts
    estimate basin size; also apply capture–recapture (Schnabel / Chao1) to the hit counts
    to estimate *how many* optima exist in total below a score threshold. Tells you whether
    the archive is nearly complete or a tiny sample.

### 3.3 Where are the good layouts? (solution-space maps)

24. **Embedding map** (optima). 2D projection of the archive. Encode each layout as a
    one-hot S×N matrix (symbol × slot, 900 dims for 30 keys); Euclidean distance on that
    encoding is √(2·d_H), so PCA/MDS preserve the natural metric. PCA for stability and
    axes you can explain; UMAP (McInnes et al. 2018) or t-SNE for cluster structure. Color by
    score, size by hit count, shape by funnel. Brushing a region selects layouts in all other
    views. Variants: same map under finger distance (encode symbol × finger, 30×8) and under
    feature distance (standardized metric vector).
25. **Consensus keyboard** (top-N optima). Draw the board; each key shows the distribution
    of symbols at that slot over the top N layouts (stacked mini-bars, or the modal symbol
    with opacity = its share). Overlay per-slot entropy: low entropy = a forced choice
    ("e is always on the left middle home key"), high = a free choice. Recompute live as
    the user drags a "top N" or "within x % of best" slider. Called the *backbone* in
    combinatorial optimization; QAP and TSP backbones are a standard analysis.
26. **Symbol × slot probability matrix** (top-N optima). The same data as 25 as an S×N
    heatmap, rows sorted by frequency. Good for spotting letter pairs that move together.
27. **Finger co-assignment matrix** (top-N optima). S×S: share of good layouts where
    symbols i and j are on the same finger. Block structure = the "finger groups" good
    layouts agree on (e.g. vowels on one hand). Cluster rows (hierarchical, dendrogram on the
    side).
28. **Hand split diagram** (top-N optima). For each symbol, share of good layouts that put
    it on the left hand. One horizontal diverging bar per symbol. Shows the vowel/consonant
    hand split and which letters are contested.
29. **Dendrogram of optima** (optima). Agglomerative clustering on d_H; leaves labelled by
    score; cut at a distance to get "families". Each family gets a representative (its best)
    and a consensus keyboard.
30. **Small multiples gallery** (families). One mini keyboard per family, sorted by best
    score, with "n layouts, best x, spread y". The browseable output of exploration.
31. **Path between two layouts** (path relinking). Pick A and B; show the sequence of
    swaps that turns A into B, choosing at each step the swap with the smallest score
    increase, as a score profile along the path plus the keyboards at each step. Shows the
    barrier between two designs and the halfway layouts nobody tried.

### 3.4 What does the objective trade? (trade-off views)

32. **Scatterplot matrix of metrics** (optima + samples). Pairs of raw metrics (SFB vs
    rolls, alternation vs redirects, index load vs hand balance), optima colored by score.
    Negative correlation inside the optima cloud = a real trade-off the user must choose on.
33. **Metric correlation matrix** (samples and optima, separately). Correlations among
    metrics across random layouts vs across good layouts. Metrics that are redundant can be
    merged; metrics that trade only among good layouts are the real preference axes.
34. **Pareto front explorer** (optima, any 2–3 metrics). Non-dominated optima highlighted;
    hover shows the keyboard. NSGA-II (`context/algorithms/nsga2.md`) fills the front
    directly, but an archive of single-objective optima under many random weights already
    gives a good approximation.
35. **Parallel coordinates** (optima). One axis per metric; one line per optimum; brush
    ranges on axes to filter ("SFB < 1 %, rolls > 40 %, index load < 30 %"). The best
    general-purpose filter for a 15-metric archive.
36. **MAP-Elites grid** (QD archive, §4.3). 2D grid over two behaviour descriptors
    (e.g. roll/alternation ratio × index load, or left-hand share × home-row share);
    each cell = best layout with those behaviours, color = score; click to open. Directly
    the "map of good but different layouts" from SYNTHESIS §5.2.
37. **Weight-sensitivity / stability region** (archive re-scoring). For the current best,
    sweep one weight (or two) and show where it stops being the best and which archive
    layout takes over. A 2D "phase diagram" over two weights, each region colored by its
    winning layout family, makes the dependence on hand-tuned weights visible.
38. **Rank-flip chart** (archive re-scoring). Bump chart of the top 20 archive layouts'
    ranks across profiles (default, carpalx, low-sfb, Engram-comfort later). Layouts that stay
    near the top under every profile are robust recommendations.
39. **Corpus robustness strip** (re-score optima on other corpora). For each top layout,
    score on each corpus as dots on one row; spread = sensitivity to text (metric L05).
40. **Contribution waterfall** (one layout vs a reference). Per-metric weighted difference
    from Semimak or QWERTY as a waterfall that sums to the score gap. Explains "why it wins".
41. **Exploitation detector** (optima). Radar or bar chart of raw metrics for the best
    optimum next to the range of community layouts (cmini import). A metric far outside the
    human range (e.g. extremely low SFB bought with absurd redirects or pinky load) flags that
    the optimizer is exploiting a gap in the objective. The 8.40 vs 11.85 gap to Semimak
    needs this check.

### 3.5 Comparing algorithms (bench views)

42. **Performance profiles** (Dolan & Moré 2002) across (seed, geometry, profile)
    instances: fraction of instances on which each algorithm is within factor τ of the best.
43. **ECDF of time-to-target** per algorithm (= 3, side by side).
44. **Merged STN** (= 18) with one color per algorithm.
45. **Diversity-over-time** (trace). Mean pairwise d_H among a GA's population or among
    workers' current layouts. Premature convergence shows as a collapse to 0.

### 3.6 Explaining one layout (editor views beyond what exists)

Existing: 3D finger colors, frequency heat, SFB arcs, metric bars, finger load, worst
n-grams. Add:

46. **Swap-cost tint** (= 20 on the board) and **importance glow** (= 21).
47. **Trigram flow** on the board: arcs for top rolls (one style) and redirects (another),
    width ∝ frequency. Same mechanism as the SFB arcs.
48. **Worst words** list per metric (SYNTHESIS §6), with each word animated as a finger path
    on the board on hover.
49. **"Nearest better layouts"**: the archive layouts within d_H ≤ 4 that score better,
    with the swaps needed. Turns the archive into editing suggestions.
50. **"How unusual is this layout"**: percentile in the random-layout distribution (13) and
    the density of states (14), plus distance to the nearest known community layout.

## 4. Exploring the space and escaping local minima

The goal of exploration differs from optimization: find **all** good basins, describe them,
and measure how well we have covered the space. Methods, in order of value for GECKO.

### 4.1 Better single-trajectory escapes (cheap, reuse `delta`)

| Method | Idea | Why for GECKO |
|---|---|---|
| **Iterated local search / Breakout Local Search** | Descend; kick with k random (or tabu-directed) swaps; descend; accept if better or by rule. BLS adapts k: grow when it returns to the same optimum | Every kick produces an (optimum → optimum) edge: ILS *is* a LON sampler. Benlic & Hao matched best-known results on almost all QAPLIB instances ([qap-metaheuristics](sources/qap-metaheuristics.md)) |
| **Structured kicks** | Swap two whole columns on the same finger, swap the hands (mirror), rotate a finger's keys, 3-cycles, swap two letters' fingers keeping rows | Large semantic moves cross barriers that 1-swap SA climbs slowly; and they map to ideas users understand |
| **Robust tabu search** | Best non-tabu swap; random tenure; aspiration; long-term frequency memory forces unused moves | Deterministic-ish coverage; its frequency memory is itself a visualization (3.1 #9) |
| **Parallel tempering (replica exchange)** | R replicas at a temperature ladder; neighbours swap states with Metropolis probability | Uses 20 cores well; robust to the T schedule; samples the Boltzmann distribution, which feeds the density of states and consensus views without bias |
| **Population annealing** | Many replicas cooled together with resampling | Same benefits, and the population at each T is an ensemble to visualize |
| **Late acceptance / threshold accepting** | Accept if better than the score L steps ago / within a threshold | One-parameter SA alternatives; cheap bench competitors |

### 4.2 Systematic diversification (find *different* good layouts)

| Method | Idea | Notes |
|---|---|---|
| **Exclusion search ("next best distinct")** | Optimize with a penalty λ·max(0, d_min − d_H(x, archive)) or a hard constraint d_H ≥ d_min to every known funnel bottom | The direct answer to "what is the best layout that is *not* this one?" Repeat to list funnels in score order. The penalty is cheap to update per swap (d_H changes by ≤ 2 per archive member; keep only funnel bottoms) |
| **Novelty search** | Reward distance from an archive of previously seen behaviours instead of score (Lehman & Stanley 2011) | Use as a scout with a score floor (novelty only among layouts within x % of best) |
| **MAP-Elites / CVT-MAP-Elites** | Keep the best layout per cell of a behaviour grid; mutate (swap/kick + short descent) random elites | Mouret & Clune 2015; CVT variant (Vassiliades et al. 2018) for 3+ descriptors. Produces view 36 directly |
| **Niching / crowding in GA** | Replace the most similar individual; fitness sharing by d_H | Fix for the GA's high variance in the leaderboard |
| **Path relinking** | Walk from optimum A toward B along the best swaps; descend from intermediate points | Finds optima *between* funnels, and gives barrier heights for the disconnectivity graph |
| **Random-weight restarts** | Each restart uses a random weight vector near the profile; record raw metrics | Fills the Pareto/trade-off views and tests weight sensitivity for free (Kühlmak-style rank-sum) |

### 4.3 Exactness and coverage (know what is missing)

| Method | Idea | Notes |
|---|---|---|
| **Large neighbourhood search with exact repair** | Free k slots (k = 8–10, 8! = 40k to 10! = 3.6M), fix the rest, enumerate the free slots exhaustively | Proves local optimality in a much larger neighbourhood than 1-swap. Choose free slots by finger, by row, or by high entropy in the consensus keyboard |
| **Lower bounds** | Gilmore–Lawler / LP relaxation on the bigram part, MIP on sub-problems ([integer-programming](sources/integer-programming-bounds.md)) | The only way to show "within x % of optimal" on a chart (a dashed floor on the convergence chart) |
| **Capture–recapture of optima** | Estimate total optima below a threshold from hit counts | "We have seen 412 of an estimated ~460 optima within 1 % of best" is a coverage statement |
| **Canonicalization** | Mirror (and any geometry symmetry) mapped to a canonical form before dedupe | Otherwise every funnel appears twice and the maps lie |
| **Pin sweeps** | Re-run exploration with each user pin removed / added | Quantifies the cost of each constraint ("keeping `,./` costs 0.21") |
| **Objective continuation** | Explore on a smoothed objective (bigrams only, or truncated trigram coverage) first, then refine on the full one | Fewer, wider basins on the smooth landscape; fast scouting. Already partly supported by `trigram_coverage` |

### 4.4 Recommended explore engine

One algorithm, `explore`, combining the pieces that produce the most visual data per eval:

1. **Probe** (seconds): 10k random layouts + 200 random walks of length 1000 → score
   distribution, correlogram, correlation length ℓ, metric correlations.
2. **Seed** (seconds): many short descents from random starts (multi-start HC) → first
   optima, hit counts, initial FDC.
3. **Funnel mapping** (bulk of budget): BLS on every worker, kick strength around ℓ and
   adaptive; every descent writes an optimum, every kick an edge. Workers start from
   different funnels (pick least-visited funnel bottoms) to spread coverage.
4. **Diversity pass**: exclusion search for the next-best distinct funnel until the new
   funnel bottom is worse than best + x %; MAP-Elites over two user-chosen descriptors
   in parallel on some workers.
5. **Certify** (optional): exact k-slot repair on each funnel bottom; lower bound if
   available; capture–recapture estimate.

Output: an atlas directory (§6.2). Every view in §3 reads from it.

## 5. What we might be missing

1. **Near-optimal alternatives.** One profile, one optimum is what every run reports. If
   FDC is weak or the LON has several sinks, there are very different layouts within 1 %
   of the best that the user would prefer. Nothing currently looks for them.
2. **Objective exploitation.** Our best scores 8.40; Semimak 11.85; QWERTY 53.3. A large
   gap to human-designed layouts is either a real improvement or the optimizer finding a
   hole in the objective. The exploitation detector (#41), corpus robustness (#39), and
   independent judges (Engram comfort profile, SYNTHESIS §4) separate the two.
3. **Weight dependence.** Weights are hand-tuned. If the winner changes under small weight
   changes (#37), the "best layout" is a statement about our weights, not about typing.
   Show the stability region with every result.
4. **Metric redundancy and blind spots.** Correlation among metrics (#33) shows which
   metrics double count and which properties no metric measures (e.g. if two very different
   funnels score identically, the objective cannot tell them apart; ask what does).
5. **Constraint cost.** Pins and fixed punctuation shape the space. Pin sweeps make the
   price of each constraint explicit.
6. **Symmetry and duplicates.** Mirror images and swaps of keys with identical cost
   (columns that are geometrically equivalent on ortho boards) inflate the archive and fake
   diversity. Canonicalize; report true distinct counts.
7. **Neighbourhood blindness.** 1-swap local optima are not k-swap optima. The steepest
   descent polish stops at 1-swap optimality; exact k-slot repair (§4.3) tests whether
   cheap improvements are being left behind.
8. **Coverage.** We do not know how many good basins exist. Capture–recapture and the
   density of states turn "we ran a lot" into a number.
9. **Search bias.** All restarts but the first start random; the first starts from the
   given layout. STNs and the restart strip reveal if one start dominates the result.
10. **Ergonomic plausibility beyond the score.** Hand-model costs arrive later (phase 7);
    until then, the consensus keyboard and importance glow let the user judge plausibility
    visually ("why is `h` on the pinky in every good layout?").

## 6. Explore mode: adapting the program

The product change: next to **Optimize** ("give me the best layout"), add **Explore**
("show me the space of good layouts and why"). Optimize answers with one layout and a
convergence line. Explore answers with an atlas and a set of linked views.

### 6.1 User flow

1. Choose geometry, corpus, profile, pins (same Inputs panel).
2. Press **Explore** with a budget (e.g. 2 min). The atlas fills live: points appear in the
   embedding map, funnels form in the LON, the consensus keyboard sharpens.
3. Brush a region of the map or a range in parallel coordinates → the consensus keyboard,
   metric distributions, and gallery update to the selection.
4. Drag profile weights → the whole archive re-ranks instantly (archive re-scoring, no
   search). Regions of the map light up or fade. "Re-explore here" seeds a new explore run
   from the selection with the new weights.
5. Open any layout in the editor with its swap-cost tint and nearest-better suggestions.
6. Save a family or a layout to `data/layouts/` with its provenance (atlas id, funnel).

### 6.2 Atlas format (new run type)

```
runs/<timestamp>-explore-<hash>/
  config.json        # as today, + explore params; input hashes; metric list with ids
  atlas.json         # summary: counts, FDC, correlation length, funnels, coverage estimate,
                     #   probe stats, metric correlation matrices, top families
  optima.bin         # fixed-size records: sym[N] u8, score f64, raw[M] f32, nonlin[F] f32,
                     #   hits u32, first_seen_ms u32, funnel u32  (canonicalized, deduped)
  edges.bin          # (from u32, to u32, kick u8, strength u8, dscore f32)
  samples.bin        # random layouts / walks: score f32 + raw[M] f32 (+ sym for walks)
  trace.bin          # per-worker trace records (§2)
  embedding.json     # precomputed PCA (and optional UMAP) coordinates for the top K optima
  progress.jsonl     # as today
  best.json          # as today, so existing Results view keeps working
```

Binary records are little-endian with a header (`GKATLAS1`, record sizes, M and the metric
ids) so the web can read them with a `DataView` without a parser. Sizes: 1M optima ≈ 150 MB;
cap with a score threshold (keep optima within x % of best + a uniform sample of the rest).

### 6.3 Implementation notes by crate

**gecko-core**

- `optimize/mod.rs`: extend `StepInfo` (or add an `Observer` passed to `step`) with
  optional trace and optimum events. Keep the inner loop untouched: SA already works in
  256-step chunks; emit one trace record per chunk with counters it increments (accepted,
  improving) — two integer adds per step. `end_restart` already polishes the best and
  offers it to `TopK`; also emit it as an optimum event with its start hash.
- New `explore/` module, depends on `scorer` and `optimize`:
  - `archive.rs`: hash-indexed store of canonical optima (open-addressing on the layout
    hash; outside the hot loop so a `HashMap` is acceptable here), hit counts, funnel ids,
    raw metric vectors via `report::breakdown` (costly, so compute once per *new* optimum).
  - `probe.rs`: random sampling, random walks, autocorrelation, correlation length.
  - `bls.rs`: Breakout Local Search as an `Optimizer` (adds `Algo::Bls`; also a bench entry,
    SYNTHESIS backlog #7).
  - `explore.rs`: the §4.4 schedule as an `Optimizer` (`Algo::Explore`), using the archive.
  - `exclusion.rs`: distance penalty to funnel bottoms; delta of the penalty for a swap is
    O(#bottoms), applied *after* the scorer's delta (no change to `Scorer`).
  - `qd.rs`: MAP-Elites grid with descriptor functions over the raw metric vector.
  - `stats.rs`: FDC, Pearson/Spearman, capture–recapture (Chao1), PCA on one-hot (power
    iteration on a 900-dim Gram matrix is enough), hierarchical clustering on d_H for the
    top K, funnel extraction (follow each optimum's best improving edge to a sink).
  - `distance.rs`: Hamming, Cayley, finger, hand distance; mirror canonicalization using
    the geometry's mirror map (add `mirror: Option<Vec<usize>>` to `Geometry`, computed from
    positions).
- `report.rs`: add `swap_matrix(sc, st) -> Vec<f32>` (435 deltas, symmetric S×S) and
  `slot_importance`. Both are one neighbourhood scan.
- Re-scoring: `rescore(raw: &[f32], nonlin: &[f32], profile) -> f64` must match
  `Scorer::full` exactly; add it to the property tests (rescored archive score == full
  score for random layouts and random weights). This makes the archive trustworthy.

**gecko-cli**

- `gecko explore --time 2m [--descriptors roll_ratio,index_load] [--within 2%]` → atlas run.
- `gecko landscape` → probe only: prints correlation length, FDC (if an atlas exists),
  random-layout percentiles of the reference layouts. `--json` with stable schema.
- `gecko atlas <run> [--families] [--rescore profile] [--pareto SFB,T01]` → tables from an
  existing atlas.
- `gecko bench` gains `--ecdf` output (time-to-target per seed) for views 3/42/43.

**gecko serve API**

| Route | Purpose |
|---|---|
| `POST /api/runs` with `algo: "explore"` | start an explore run (same lifecycle, pause/resume/cancel) |
| `GET /api/runs/{id}/atlas` | `atlas.json` |
| `GET /api/runs/{id}/atlas/{optima,edges,samples,trace}` | binary streams (range requests for big files) |
| SSE `atlas` events | batches of new optima/edges while running (append to client arrays) |
| `POST /api/swap-matrix` | swap-delta matrix for a layout (fallback when WASM is unavailable) |

**gecko-wasm**

- `swap_matrix(chars, profile) -> Float32Array` and `slot_importance` for live editor views.
- `rescore_archive(optima_bytes, profile) -> Float32Array` so weight edits re-rank the atlas
  in the browser. 100k optima × ~25 metrics = 2.5M multiply-adds: well under 10 ms.

**web**

- New view tab **Explore** beside Home / Results / Layout analysis.
- Panels (resizable grid, linked selection state in `App.tsx`): Embedding map (canvas or
  WebGL points; SVG does not scale past ~5k points), FDC scatter, Consensus keyboard (reuse
  `Keyboard3D` with a per-key texture of symbol shares, plus a 2D variant like `Heatmap`),
  Parallel coordinates / scatter matrix, LON (force layout in a worker; d3-force is the
  only new dependency worth adding), Gallery of families, selected layout's breakdown.
- Editor additions on Home: swap-cost tint overlay, importance overlay, trigram arcs.
- Results view: restart strip and multi-worker convergence from `trace.bin`.
- Keep hand-rolled SVG for small charts (consistent with `Charts.tsx`); use canvas for
  dense point clouds and rasters (#7, #9, #24).
- Colors: score as the white → ink sequential ramp already in `colors.ts`; families as the
  existing muted qualitative palette; accent only for the selected layout and the best.

### 6.4 Performance budget

- Telemetry cost: ≤ 1 % of evals/s (one record per 256 steps). Verify with `gecko perf`
  before/after.
- New optimum handling (breakdown + archive insert) ≈ tens of µs; descents take ~1–10 ms,
  so < 1 %.
- Explore at 20 threads for 2 minutes: roughly 10⁵–10⁶ descents → after dedupe and a
  threshold, 10⁴–10⁵ archived optima. Enough for every view; the web loads the top 20k plus
  a uniform sample.
- Determinism: atlas contents are reproducible with `--evals` budgets (as runs are today);
  time budgets give the same statistics but not identical records.

## 7. Roadmap

| # | Step | Effort | Delivers views |
|---|---|---|---|
| 1 | Trace + optimum events in the runner; `trace.bin`, optima list in `best.json` | S | 1, 2, 4, 7, 8, 10 |
| 2 | Swap-delta matrix + importance (core, WASM, editor overlays) | S | 19, 20, 21, 46 |
| 3 | `gecko landscape` probe | S | 12, 13, 33 |
| 4 | Archive + raw-vector re-scoring with property test | M | 11, 25–28, 32, 35, 37–39 |
| 5 | BLS + `explore` engine + edges | M | 15, 16, 23, 31 |
| 6 | Web Explore view (embedding, FDC, consensus, parallel coords, gallery) | M | 24, 29, 30, 34 |
| 7 | Exclusion search + MAP-Elites | M | 36, "next best distinct" |
| 8 | Parallel tempering / Wang–Landau density of states | M | 14, 50 |
| 9 | Exact k-slot repair, lower bound line, capture–recapture | M | coverage and certificates |
| 10 | STNs and ECDF / performance profiles in bench | S | 3, 18, 42–45 |

Steps 1–4 are worth doing before more objective work: they make every later change to the
objective visible (does a new metric move the optimum, split a funnel, or just shift scores?).

## 8. Open questions

- Which two behaviour descriptors users care about most for the MAP-Elites grid. Candidate
  default: roll/alternation ratio × index-finger load; let the UI choose any two metrics.
- Whether to store full trace layouts or only hashes beyond the first minute (size).
- Threshold for "near-optimal" in every view: fixed % of best, or in reference-spread units
  once spread-normalized weights exist (SYNTHESIS §3.6). Spread units read better.
- Whether UMAP is worth a dependency (umap-js in the web) over PCA + clustering; start with
  PCA and add UMAP if PCA maps look like one blob.

## References

Landscape analysis
- Jones & Forrest, "Fitness distance correlation as a measure of problem difficulty", ICGA 1995.
- Weinberger, "Correlated and uncorrelated fitness landscapes and how to tell the difference",
  *Biol. Cybern.* 63, 1990 (autocorrelation, correlation length).
- Merz & Freisleben, "Fitness landscape analysis and memetic algorithms for the quadratic
  assignment problem", *IEEE TEC* 4(4), 2000 (FDC + autocorrelation for QAP).
- Daolio, Verel, Ochoa, Tomassini, "Local Optima Networks of the Quadratic Assignment Problem",
  CEC 2010, https://arxiv.org/abs/1107.4161 ; communities in QAP LONs:
  https://ar5iv.labs.arxiv.org/html/1210.4021 ; LONs + autocorrelation vs performance (PPSN 2012):
  https://www.cs.stir.ac.uk/~goc/papers/qaplonppsn12.pdf
- Ochoa, Malan, Blum, "Search trajectory networks: A tool for analysing and visualising the
  behaviour of metaheuristics", *Applied Soft Computing* 109, 2021,
  https://dspace.stir.ac.uk/bitstream/1893/32613/1/stns_asoc_2021.pdf
- Becker & Karplus, "The topology of multidimensional potential energy surfaces", *J. Chem.
  Phys.* 106, 1997 (disconnectivity graphs).
- Wang & Landau, "Efficient, multiple-range random walk algorithm to calculate the density of
  states", *PRL* 86, 2001.

Search
- Benlic & Hao, Breakout Local Search for QAP, 2013 (see [qap-metaheuristics](sources/qap-metaheuristics.md)).
- Lehman & Stanley, "Abandoning Objectives: Evolution through the Search for Novelty Alone",
  *Evolutionary Computation* 19(2), 2011,
  https://www.cs.swarthmore.edu/~meeden/cs81/f17/papers/lehman_ecj11.pdf
- Mouret & Clune, MAP-Elites, arXiv:1504.04909, 2015; Vassiliades, Chatzilygeroudis, Mouret,
  CVT-MAP-Elites, *IEEE TEC* 22(4), 2018 **(verify citation details)**.
- Earl & Deem, parallel tempering review, 2005 (see [search-methods-general](sources/search-methods-general.md)).

Evaluation and visualization
- Hoos & Stützle, *Stochastic Local Search: Foundations and Applications*, 2004 (run-time
  distributions, ECDFs).
- Dolan & Moré, "Benchmarking optimization software with performance profiles", *Math.
  Program.* 91, 2002.
- McInnes, Healy, Melville, UMAP, arXiv:1802.03426, 2018.
- Chao, "Nonparametric estimation of the number of classes in a population", *Scand. J.
  Stat.* 11, 1984 (capture–recapture, Chao1).

Confidence: the LON/QAP, STN and novelty-search references were checked online on 2026-10-20.
Other references are standard and cited from memory; check volume/page details before
quoting them elsewhere. Performance numbers in §6.4 are estimates from `gecko perf` figures in
`context/objective.md`, not measurements.
