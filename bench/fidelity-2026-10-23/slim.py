"""Drop per-layout breakdowns from saved `optimize --json` outputs (keeps ids, timing, scores)."""
import json, sys
for f in sys.argv[1:]:
    d = json.load(open(f))
    r = d['run']
    r['best'] = [{k: b[k] for k in ('rank', 'score', 'chars', 'hash')} for b in r['best'][:3]]
    for k in ('bound', 'front'):
        r.pop(k, None)
    d.pop('near_ties', None)
    json.dump(d, open(f, 'w'), indent=1)
