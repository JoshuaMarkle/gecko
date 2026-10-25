#!/bin/sh
# Digits on the Sym layer in the ltgt frames (< > on the layer): row/pad × free/human, 6 × 20M.
cd "$(dirname "$0")"
for tag in ltgt ltgt-vim; do
  case $tag in *vim) c=vim-programmer-typed; p=vim;; *) c=programmer-typed; p=prog;; esac
  for shape in pad row; do
    for kind in human free; do
      python3 -I study.py search $tag-digits-$shape-$kind layouts/digits/$tag-$shape.toml profiles/$kind-$p.toml $c 6 20000000
    done
  done
done
