#!/bin/sh
# Digits on the Sym layer (digits.py): row or numpad shape, pinned; the Num layer is open for
# overflow. us and us-vim × free/human, 6 seeds × 10M BLS.
cd "$(dirname "$0")"
for tag in us us-vim; do
  case $tag in *vim) c=vim-programmer-typed; p=vim;; *) c=programmer-typed; p=prog;; esac
  for shape in row pad; do
    for kind in free human; do
      python3 -I study.py search $tag-digits-$shape-$kind layouts/digits/$tag-$shape.toml profiles/$kind-$p.toml $c
    done
  done
done
