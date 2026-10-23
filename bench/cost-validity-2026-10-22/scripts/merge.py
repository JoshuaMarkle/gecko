import json
R={}
for p in ('data/results_part1.json','data/results_part2.json','data/results_part3.json'): R.update(json.load(open(p)))
R['_note']='Aggregates for docs/papers/cost-validity. GECKO-derived values (ansi-30, english, default profile unless stated); timing values are summary statistics of the Aalto 136M keystroke data (Dhakal et al. 2018, Kaggle mirror noelsmathew/keystrokes).'
json.dump(R,open('/home/josh/projects/gecko/bench/cost-validity-2026-10-22/results.json','w'),indent=1,sort_keys=True)
