"""Swap-delta throughput against the number of enabled metrics (software paper, Fig. 3).
Usage: python3 -I bench/software-paper-2026-10-21/metrics_scaling.py   (run on an idle machine)
Metrics are enabled in a fixed order: mono/bigram, then skipgram, then load/balance terms, then trigram.
Each point is the median of 5 `gecko perf` runs (1 thread, ansi-30, english, qwerty start)."""
import json, statistics, subprocess, sys, tempfile
from pathlib import Path
HERE = Path(__file__).resolve().parent
G = HERE.parents[1] / 'target/release/gecko'
ORDER = ['B01', 'S01', 'B03',            # bigram-class tables
         'S02',                          # first skipgram metric
         'B04', 'B05', 'B06', 'B07', 'B09', 'B10', 'B14', 'B11', 'M01',   # more bigram / mono
         'M02', 'M03', 'M07',            # non-linear load terms
         'M04', 'M09', 'M05', 'M06', 'B08', 'S03', 'S05', 'S04',
         'T01',                          # first trigram metric
         'T02', 'T03', 'T04', 'T06', 'T07', 'T08', 'T09', 'T10', 'E04', 'E05']
KS = (1, 3, 4, 8, 12, 13, 16, 24, 25, 30, len(ORDER))
rows = []
with tempfile.TemporaryDirectory() as tmp:
    for k in KS:
        p = Path(tmp) / f'k{k}.toml'
        p.write_text(f'name = "k{k}"\n[weights]\n' + ''.join(f'{m} = 1.0\n' for m in ORDER[:k]))
        runs = [json.loads(subprocess.run([G, 'perf', '--json', '-p', p], capture_output=True, text=True, check=True).stdout)
                for _ in range(5)]
        rows.append({'metrics': k, 'last_added': ORDER[k - 1],
                     **{key: statistics.median(r[key] for r in runs) for key in ('delta_per_sec', 'full_per_sec', 'build_per_sec')}})
        print(rows[-1], flush=True)
(HERE / 'metrics_scaling.json').write_text(json.dumps(rows, indent=1) + '\n')
