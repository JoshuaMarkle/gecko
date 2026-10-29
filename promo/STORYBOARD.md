# GECKO promo: storyboard v5

Supersedes v4 (black stage, LED underglow, stat rings, the loop back to frame 0). This is the prose;
`src/storyboard.ts` holds the timeline and wins where they differ. Every number on screen comes from
real GECKO output (`src/generated/promo-data.json`, the runs listed below).

## Decisions

- **About 84 s at 1920 × 1080, 30 fps, no music.** One composition, `promo`.
- **Light, like the product.** The product's own tokens (`context/design.md`): white stage, near-black
  ink, one blue (`#0066ff`) for the best value and the pressed key. A white studio in 3D (white floor
  and fog, soft shadow). Finger fills and heat are blue ramps.
- **Shown, not told.** No headlines. The only words are data labels, the document, the end card.
- **The board is the hero.** A full-detail Corne 3x6+3: black anodized plate, black PCB with white
  silkscreen, matte white dished caps, blue stems. No LEDs or underglow.
- **Slow camera.** The macro shots move at half the v4 speed; cuts are dips to white.
- **Ends on the wordmark.** No loop back into the opening.

## The runs behind it (all on `corne-42`, corpus `prose`, profile `hand`)

| Run | Shown in |
|---|---|
| `20261029-023718-sa-ed589d56` layout SA, QWERTY 47.45 → 9.872, 102 trace snapshots | Optimize, Tunnel, App |
| `20261029-023950-geom-8e314d16` board run, 10.44 → 0.44, tent 25.5°, row curve 17.8° | Physical |
| same run, `stl/` (plate, case, bottom) | Physical |
| `runs/*-explore-*`, qd grid, `gecko perf` | Tunnel, App |

## Scenes

| # | Scene | What happens |
|---|---|---|
| 1 | `opening` | A slow pivot around the top-left corner key: the camera moves in and swings left around the corner. Dip. A slow drift over the nice!nano; focus pulls from the caps onto its silkscreen (the glass cover is a thin, nearly invisible coat, so the text stays sharp). Dip. A longer low pass along the thumb keys. Cut (dip) to a low, flat shot from the front that rises over the board to the top view (exponential: it leaves the low angle at once and slows into the top), which narrows to orthographic in 0.4 s (the rise takes 1.8 s). |
| 2 | `optimize` | The 2D drawing fades in exactly over the 3D top view (2D legends at the 3D caps' size) and grows. Finger fills wipe in while every legend scrambles into its key's pose, and slide straight back in one motion (about 1.7 s). Back to white; the board rises and a stat strip arrives under it (score with the QWERTY reference, same finger, rolls, alternation, pinky–ring; no leaders, no rings). The SA trace plays: each key's legend changes in place (no travel), the strip counts each snapshot. The score lands at 9.87 and turns blue. The camera dives into the gap between F and G. |
| 3 | `tunnel` | A tunnel of the product's charts (no cards, no walls) facing the camera in mirrored left/right pairs, evenly spaced. It accelerates and fades to white: nothing waits at the end. |
| 4 | `language` | A word processor (title bar, tabs, a ribbon of text tools, ruler) holds a full page: *Pride and Prejudice*, Chapter I. The chrome falls away and the page tilts, right edge toward the camera, its text staying on it. The first four lines unravel character by character into one line, in reading order: the page glides right at a steady speed ahead of the line's end and each glyph flies left off it to its slot, while the camera glides between them, closer. The line runs on through a loop where it glitches through the corpus's symbols, comes out as data (one ink tick per letter, height = the letter's corpus frequency), and the ticks run along the chart's baseline into a letter-frequency chart, fading into it while its bars grow and re-scale. The camera settles on the finished chart. |
| 5 | `physical` | One 3D take. Shot low and level from in front of the left half: the hands type the novel's opening on the final layout; the pressed key turns blue; a stat block reads out that press (key, reach, press angle, reach cost, wrist cost) and the typed text runs along the bottom. As the skin dissolves to the skeleton the camera orbits past the side, a little toward the board's far edge; each finger bone carries its length (mm), and the skeleton keeps typing the sentence. The orbit keeps going the same way round the board, rising, into the board view; a second later the skeleton fades and "Optimize for" switches Layout → Board; the board run plays back beside its score (10.44 → 0.44; only the final score turns blue) and range bars, then "No collisions". The run's real STL case (no bottom plate for now) rises slowly and closes around the caps; the file card for `case.stl` fades up and its grey download button is clicked: it sinks and turns blue with a check. |
| 6 | `app` | The Optimize › Layout tab fills the frame and holds while the run plays (3 s), then tilts back; the explore atlas, MAP-Elites, the Home flowchart (inputs → scoring → optimizers) and a terminal typing `gecko perf` lift off it one by one, half a second apart; the terminal finishes and holds 1.5 s before the end card. |
| 7 | `end` | GECKO on five keycaps; half a second after they land, the C is pressed and turns blue. The long name and the repo URL rise. The video ends here. |

## Components

The library in `README.md` → Layout. Scene files: `src/scenes/{Opening,Optimize,Tunnel,Language,Physical,App,End}.tsx`; the document chrome is `src/ui/Document.tsx`.
