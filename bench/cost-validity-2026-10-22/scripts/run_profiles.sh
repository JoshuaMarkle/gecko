cd ~/projects/gecko-workspace/costlab
until ! pgrep -f "costlab opt --evals 20000000 --seeds 2$" >/dev/null && [ $(wc -l < data/sweep.jsonl) -ge 598 ]; do sleep 10; done
for pr in default sfb hand engram learned data/timefit.toml; do
  echo "{\"id\":\"profile:$(basename $pr .toml)\"}" | ./target/release/costlab opt -p $pr --evals 20000000 --seeds 2
done > data/profile_opt.jsonl 2> data/profile_opt.err
echo profiles done
