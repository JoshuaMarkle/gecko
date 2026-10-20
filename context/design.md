# Visual design

The look of the GECKO web UI. Adapted from a monochrome, gallery-style design system and
scaled down for a dense tool (closer to CAD or Blender than a marketing page).
Tokens live in `web/src/styles.css` `:root`; use them, never literal values.

## Principles

- **Monochrome.** Near-black ink on white. Structure comes from a ladder of faint neutral
  tints and 1px hairlines, not shadows or color.
- **Content supplies the color.** The keyboard (finger colors, heatmaps) is the only rich
  color on screen. Chrome frames it.
- **One accent.** Electric blue marks selection and the one thing that needs attention
  (the best layout, the selected key). One or two blue elements per screen at most.
- **Semantic colors report state only** (better / near / worse, run ok / failed), as small
  marks: a dot, a bar, a badge, a 3px left edge. Never decoration.
- **Quiet precise geometry.** Small radii, flat surfaces, no gradients on chrome.

## Colors

| Token | Value | Use |
|---|---|---|
| `--ink` | #141414 | Text, primary buttons, focus ring, active states |
| `--ink-soft` | #262626 | Row labels, secondary lockups, primary button hover |
| `--muted` | #707070 | Secondary copy, hints, inactive segments, table headers |
| `--faint` | #adadad | Placeholders, disabled, tertiary text |
| `--canvas` | #ffffff | Page and panel background |
| `--soft` | #f3f3f3 | Workhorse tint: 3D viewport, segmented tracks, notices, soft buttons |
| `--field` | #f0f0f0 | Form input fill (inputs have no border at rest) |
| `--hair-soft` | #f0f0f0 | Faintest dividers (table rows, section edges) |
| `--hair` | #e0e0e0 | Control borders, panel edges |
| `--accent` | #0066ff | Selection and the single highlighted item (best layout, selected key) |
| `--success` | #24a148 | Better than reference, run complete |
| `--warning` | #f1c21b | Near a limit; fill or edge only, never text on white |
| `--error` | #da1e28 | Worse than reference, failed run, errors, danger button |

Finger colors in the 3D view and heatmaps are data, not chrome: a muted 8-step
qualitative palette (pinky → index, mirrored per hand), plus a white → ink sequential ramp
for frequency heat. Keep them desaturated so the accent still stands out.

## Typography

Font: **Inter** (variable), fallback `-apple-system, "Helvetica Neue", Arial, sans-serif`.
The voice comes from weight contrast: **650** headings, **450** text, **300** light leads.
Mono (`ui-monospace, Menlo, Consolas`) for layout strings and n-grams.

| Role | Size | Weight | Line height |
|---|---|---|---|
| Wordmark | 15px | 650 | 1.0 |
| Pane heading | 13px | 650 | 1.25 |
| Section title | 12px | 600 | 1.3 |
| Body / controls | 12px | 450 | 1.4 |
| Hint / table | 11px | 450 | 1.33 |
| Big number (score) | 28px | 650 | 1.0 |

- Zero letter-spacing, sentence case, no all-caps.
- `font-variant-numeric: tabular-nums` for every number.

## Spacing and shape

- Base 8px with 4px half-steps: `--s1` 4 · `--s2` 8 · `--s3` 12 · `--s4` 16.
- Control height `--ctl-h` 26px; primary button 30px.
- Radii: `--r-xs` 4px (badges, segments, swatches) · `--r-sm` 6px (buttons, inputs,
  selects, segmented tracks) · `--r-md` 8px (cards, notices) · `--r-full` only for
  switches and progress tracks. Never stadium-pill buttons.
- Keycaps in the 2D view: `--r-sm`; in 3D: a sharp base and tapered walls with rounded
  vertical corners (plan radius larger at the base than the top), and a flat top with no bevel.

## Elevation

Shadow-free. Hierarchy = fill difference (white vs `--soft`/`--field`) + hairlines.
The only lift is the active segment of a segmented control (white segment, 1px hairline ring).

## Components

- **Button primary**: `--ink` fill, white label. For a pane's single main action.
- **Icon tool button**: segmented-track option holding a 16px icon; see the toolbar above.
- **Button outline**: white, 1px `--hair` border. Secondary actions.
- **Button soft**: `--soft` fill, no border. Tertiary/utility (exports, reset).
- **Button danger**: `--error` fill. Only for destructive actions (delete run).
- **Input / select**: `--field` fill, no border, `--r-sm`; focus = 2px `--ink` ring; invalid =
  1px inset `--error` ring.
- **Switch**: 28×16 track, `--hair` off, `--ink` on.
- **Segmented control**: `--soft` track, `--r-xs` options; active option is a white segment;
  inactive labels `--muted`.
- **Section** (`details`): title row with a small chevron in the left gutter, body in a
  3-column grid (label | control | unit) so everything lines up through the sidebar.
- **Status badge**: `--r-xs` chip; solid success/error with white text, warning fill with ink text.
- **Notice**: `--soft` box, `--r-md`, 3px left edge in the semantic color.
- **Table**: 11px, tabular numbers, header `--muted` with 1px `--hair` rule, rows divided by
  `--hair-soft`; first column left-aligned, numbers right-aligned.
- **Best badge**: `--accent` chip marking the winning layout. The only blue chip on screen.
- **Progress**: 4px `--soft` track, `--ink` fill.

## App shell (`web/`)

CSS grid: a toolbar over one of three views. The toolbar follows the Unity editor: view tabs
left, transport controls dead centre, run progress right.

- **Toolbar**: three equal-weight columns so the transport stays centred.
  - Left: icon tabs in one segmented track: Home, Results, Layout analysis. Active tab is a
    white segment. No wordmark; the Home icon is the way back.
  - Centre: one joined transport cluster (rounded only at its outer ends, square seams):
    a black `--ink` "Start optimization" button with a play icon (reads "Resume" while
    paused, "Optimizing…" while running), then Pause (toggle) and End (stop now, results
    kept) icons. Pause while paused is an `--accent` fill, the toolbar's single blue mark.
    Disabled Start is `--muted`; disabled icons are `--faint`.
  - Right: run status line over the 4px progress bar. Paused time does not advance it.
- Icons: 16px, 1.4px stroke or solid glyphs, `currentColor`, in 30×22 hit targets (toolbar is 36px tall; tracks and the transport cluster are 26px)
  (`components/Icons.tsx`). Every icon button has a `title` and `aria-label`.
- **Home**: inputs (left) | 3D view over charts (centre) | result (right).
  - Left, Inputs: a segmented control picks one group at a time (Layout, Board, Corpus,
    Profile, Search). Search starts with the Algorithm picker and shows only that
    algorithm's settings. The chosen group and collapsed sections are remembered per browser.
  - Centre: the 3D keyboard (on `--soft`) with view controls top-right (3D / top) and an
    overlay picker (fingers, heat, SFB arcs). Below it, chart tabs as one segmented track:
    Convergence, Metrics, Finger load, Heatmap, Worst n-grams.
  - Right, Result: run errors (notice), the layout with its `Best` badge and score, the
    layout string (mono, copyable), per-metric table vs reference, run results, runs list,
    and exports (JSON, layout TOML).
  - Side panels and the chart pane resize by dragging their hairline edge (double-click
    resets). Below 1360px wide the side panels start narrower.
- **Results**: runs table (left) | selected run (right): convergence, best layouts with
  delta vs reference (click loads one into the editor), and the run's reproducibility facts.
- **Layout analysis**: the current layout's score and delta, then cards (1px `--hair`,
  `--r-md`) in a two-column grid: heatmap, metrics table, score contributions, finger load,
  and worst n-grams full width. One column below 1100px.

## Do

- Let the keyboard supply color; keep chrome monochrome.
- Use `--accent` only for selection and the best layout.
- Report state with small semantic marks.
- Keep corners small and consistent (`--r-sm` controls, `--r-md` containers).

## Don't

- No drop shadows, gradients on chrome, or colored section bands.
- No accent on primary buttons; primary is always ink.
- No borders on inputs at rest; no letter-spacing or all-caps.
- No semantic colors as decoration; no warning-yellow text on white.
