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

Finger colors in the 3D view and heatmaps are data, not chrome: a vivid 8-step
qualitative palette (pinky → index, mirrored per hand; mid-lightness so ink legends stay
readable), plus a white → ink sequential ramp for frequency heat. Keep finger hues lighter
than the accent blue so selection still stands out.

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
  vertical corners (plan radius larger at the base than the top), and a flat top with no bevel. The case is a
  flat slab with the same treatment: rounded vertical corners, sharp top and bottom edges.

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

A navbar over one view. The navbar follows the Unity editor: tabs left, transport dead
centre, working model right.

- **Navbar** (40px): three columns (`1fr auto 1fr`) so the transport stays centred.
  - Left: icon tabs (no text labels, so more tabs fit) in segmented tracks with small muted
    group labels and hairline dividers: Home alone, Inputs (Board, Hand, Language, Profile,
    Summary), Optimize (Layout, Board, Both, Explore), Saved (Runs, Export). Active tab is a
    white segment.
  - Centre (Optimize tabs, or any tab while a run is going): one joined cluster of three icon buttons: Play (`--ink` fill; resumes while
    paused), Pause (toggle; `--accent` while paused, the navbar's single blue mark), Stop.
    The run percent sits just right of the cluster, outside the centred group.
  - Right: view actions (Summarize on Language, outline button), then the working model:
    name in 650 + score in `--muted`; "No model" in `--muted`; during a run a one-line mono
    status (`SA · 1.2M evals · best 3.41`).
- Errors show in a bar under the navbar (error notice + Dismiss).
- Icons: 16px, 1.4px stroke or solid glyphs, `currentColor`, in 30×22 hit targets
  (`components/Icons.tsx`). Every icon button has a `title` and `aria-label`.
- **Three-panel frame** (`Form.tsx` `Frame`; every input tab and Home): sidebar (`--side`: 380px, 340px at viewport widths ≤1360px;
  full height, title row with an optional right-aligned control, scrolling body, optional
  footer) | main visual (`--soft`, top right) over the bottom panel (`--bottom` = 38%,
  hairline above). Overlays on the main visual: label top-left (white chip), view controls
  top-right (segmented on white), hints bottom-left, legends bottom-right.
- Sidebar sections are collapsible (`Sec`, remembered per browser) or, on Profile, flat
  sections whose focused one gets a 2px `--accent` left edge. Disabled groups (Hand off)
  fade to 45%.
- Cards (board cards, Home flowchart boxes): 1px `--hair`, `--r-md`; selected card =
  `--accent` border + inset ring. Status labels (Language) and flowchart edges (Home) are `--soft`
  chips with a 3px left edge in the semantic color (success = summarized / on, warning =
  outdated, hair = off).
- **Runs**: runs table (left) | selected run (right): convergence, best layouts as 2D
  thumbnails with delta vs reference, reproducibility facts.
- **Home**: the flowchart alone on `--canvas`; boxes are cards (1px `--hair`, `--r-md`) with
  a 3px left edge in the semantic color (input on / off, amber when an optimize mode cannot run),
  derived boxes (physical cost, frequencies, scoring) on `--soft`; hairline elbow connectors with
  small arrowheads; the last-used optimize tab gets the `--accent` border. Column captions in
  `--muted` below.
- **Explore**: atlas controls (left, shared `--side` width) | linked cards in a two-column grid (centre) | picked
  layout (right). Point clouds are canvas scatters, grey by score (darker = better), best in
  ink and picked in `--accent` with a 2px surface ring; drag to brush, double-click clears,
  click picks. Off-chart reference layouts are listed in the plot corner, not clamped to the
  edge. Card hints wrap; controls in a card header never shrink.

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

## Research paper template

`docs/papers/templates/journal.html` is the retained local HTML paper template;
`index.html` forwards to it. It has a centered title without corner labels or a divider
above the abstract, generous abstract spacing, and a two-column body (one on mobile).
It reuses the site palette and font stack with larger reading typography and A4 print
styles. Blue highlights selected data. Figures and sample results are synthetic;
the folder includes their generator.

On screen, the paper is a white sheet with thin edges and generous margins on the
neutral soft background. It flows with content, shrinks to mobile gutters, and omits
the surrounding background when printed. No shadows are used.

The journal includes `journal.tex` and a compiled `journal.pdf`; `build_pdf.sh` runs
XeLaTeX twice with a private-format fallback for incomplete local TeX setups. PDF
figures are vector exports sized for a paper column. The PDF uses installed Noto Sans
and retains the centered title, roomy full-width abstract, and two-column body. Small
PDF/source download links sit outside the HTML paper and are hidden when printing.
HTML and LaTeX prose are maintained together; downloads serve the last compiled PDF.

PDF typography is compact: 9pt body, 8pt captions/table text, 9pt section headings,
and a 20pt title. Section, subsection, and abstract headings use lightly strengthened Noto Sans Medium at 9pt (matching body text);
the title and caption labels use Noto Sans Medium (500);
figure labels use 6.5–7pt at their designed column size.

The four-page specimen exercises numbered sections/subsections, in-text citations,
declarations, an appendix, and a five-entry bibliography. References are explicitly
fictional placeholders; LaTeX resolves citations in the existing two-pass build.

In-text citations are smaller blue links (6.5pt in PDF, 0.72em in HTML) to their
bibliography entries. Headings keep their prior space above with tighter space below
(3pt sections / 2pt subsections in PDF). HTML section headings share 14px semibold styling.

Heading weight is currently 900 in HTML and Noto Sans Medium with `FakeBold=1.5`
in the PDF, retaining body-sized headings and the compact spacing below.
