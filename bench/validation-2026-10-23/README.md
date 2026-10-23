# Phase 1 validation (#140), 2026-10-23

Write-up: `context/objective.md` §Validation. GECKO commit `fed6641`, cmini `0db088e`.

```
python3 -I cmini_compare.py ~/projects/gecko-workspace/cmini ../../target/release/gecko > numbers.json
for p in default sfb engram hand familiar; do python3 -I ranking.py ../../target/release/gecko $p > ranking-$p.json; done
gecko compare --corpora english,english-cased,shai,mt-450k,reddit [--json] > compare-corpora.{txt,json}
gecko layouts likes -c shai --top 15 > likes-shai.txt
python3 -I oxey_compare.py ~/projects/gecko-workspace/oxeylyzer numbers.json > oxeylyzer.json
```

`numbers.json` columns per stat: `cmini` (its analyzer), `aligned` (GECKO's denominator),
`matched` (+ GECKO's class definitions), `words` (n-grams from words.json, SFS incl. x␣z),
`gecko` (default profile), `gecko_full` (`trigram_coverage = 1.0`; equals `words`).

`oxeylyzer.json` columns: `oxey` (as shipped), `oxey_full` (trigram_precision 1e6), `aligned`
(÷ share of n-grams on the layout), `gecko_full`. Needs a built Oxeylyzer (`d015a16`).
