#!/bin/sh
# Main grid: prose x 3 symbol sets x 20 seeds; then the apos-set optimum of each English corpus (10 seeds).
G=../../target/release/gecko
for s in apos qwerty semi; do python3 -I seeds.py $G prose $s 20 25000000 shai > seeds-prose-$s.json; done
for c in prose-web prose-chat prose-books shai reddit english; do python3 -I seeds.py $G $c apos 10 25000000 prose > seeds-$c-apos.json; done
