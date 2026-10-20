# Data sources

## Text corpora (for n-gram statistics)

| Source | Use | Notes |
|---|---|---|
| Wikipedia dumps | General English prose | Large; strip markup |
| Project Gutenberg | Literary prose | Older language; good for robustness checks |
| OpenSubtitles / chat logs | Conversational text | Closer to everyday typing |
| Google Books n-grams | Pre-counted n-grams | Huge (German 1-grams ≈ 3 GB); Kühlmak uses it |
| GitHub code (per language) | Symbol layers, code profile | Needed for Y02 metrics |
| Monkeytype / typing-test word lists | Short common-word sets | Small, popular in the community |
| User's own text | Personal profile | Emails, notes, code the user actually writes |

Keep raw text out of git if large; commit only the compiled n-gram tables or a fetch script.

## Typing timing data (for empirical metrics)

| Source | Use | Notes |
|---|---|---|
| Aalto "136M keystrokes" (Dhakal et al., CHI 2018) | Fit IKI model per bigram/finger pair | 168k typists, mostly QWERTY; public for research |
| User keystroke logs | Personal timing model (E02) | Requires a logger; privacy: keep local |

Caveat: timing data comes from QWERTY typists, so it reflects QWERTY habits. Use it to fit
**geometric** costs (finger pair, distance, direction), not per-letter costs.

## Reference layouts

QWERTY, Dvorak, Colemak, Colemak-DH, Workman, Halmak, Canary, Graphite, Sturdy, Semimak,
Gallium, Hands Down variants. Store as `data/layouts/*.toml` with source links.
