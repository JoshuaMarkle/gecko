#!/bin/sh
# Vim grid on corne-36 (see README): h j k l policies on vim-programmer-typed, then the Vim share.
cd "$(dirname "$0")"
G=../../target/release/gecko
S=layouts/us-layer-esc.toml
P=profiles
SEEDS=../programming-layout-2026-10-25/seeds.py
for v in free pairs row fingers; do python3 -I $SEEDS $G vim-$v $S vim-programmer-typed $P/vim-$v.toml > seeds-vim-$v.json; done
python3 -I $SEEDS $G vim-qwerty layouts/us-layer-esc-qhjkl.toml vim-programmer-typed $P/vim-free.toml 6 40000000 hjkl > seeds-vim-qwerty.json
for s in 11 33; do python3 -I $SEEDS $G share-$s $S vimshare-$s $P/vim-pairs.toml > seeds-share-$s.json; done
python3 -I $SEEDS $G share-00 $S programmer-typed $P/vim-pairs.toml > seeds-share-00.json
