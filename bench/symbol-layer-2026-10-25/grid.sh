#!/bin/sh
# Main grid: each frame (us, us-vim, ltgt, ltgt-vim) × profile (free, human), 6 seeds × 10M BLS,
# from the frame's US-style layer; letters, base punctuation, digits and fillers pinned.
cd "$(dirname "$0")"
for tag in us us-vim ltgt ltgt-vim; do
  case $tag in *vim) c=vim-programmer-typed; p=vim;; *) c=programmer-typed; p=prog;; esac
  for kind in free human; do
    python3 -I study.py search $tag-$kind layouts/$tag/us-layer.c.toml profiles/$kind-$p.toml $c
  done
done
