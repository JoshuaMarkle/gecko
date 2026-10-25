#!/bin/sh
# Number layer open to symbols: numpad digits pinned on Num (left hand), every other Num key free,
# so the search decides which symbols live with the digits. 4 frames × free/human, 6 × 10M.
cd "$(dirname "$0")"
for tag in us us-vim ltgt ltgt-vim; do
  case $tag in *vim) c=vim-programmer-typed; p=vim;; *) c=programmer-typed; p=prog;; esac
  for kind in free human; do
    python3 -I study.py search $tag-numopen-$kind layouts/numopen/$tag.toml profiles/$kind-$p.toml $c
  done
done
