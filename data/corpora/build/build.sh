#!/usr/bin/env bash
# Rebuild GECKO's component corpora and languages (data/corpora/index.toml).
# Usage: data/corpora/build/build.sh [cmini checkout] [list dir]
# Needs: a release build (./target/release/gecko), a cmini checkout (github.com/Apsu/cmini) for
# the prose word lists, and the local sources select_sources.py looks for (cargo registry,
# Python 3.14 stdlib, node_modules, /usr/bin scripts, Neovim runtime). Sources differ between
# machines, so a rebuild elsewhere gives different hashes; the committed .gkc files are the
# reference.
set -euo pipefail
cd "$(dirname "$0")/../../.."
G=./target/release/gecko
CMINI=${1:-$HOME/projects/gecko-workspace/cmini}
LISTS=${2:-$(mktemp -d)}
/usr/bin/python3 -I data/corpora/build/select_sources.py "$LISTS"
$G corpus import "$CMINI/corpora/shai" --charset code --out prose-web
$G corpus import "$CMINI/corpora/reddit" --charset code --out prose-chat
$G corpus build --charset code --paragraphs --strip-markup --out prose-books data/corpora/src/*.txt
for k in code-rust code-c code-python code-js code-shell code-lua markdown; do
  $G corpus build --charset code --list "$LISTS/$k.list" --out "$k"
done
$G corpus mix --out prose
$G corpus mix --out code-all
$G corpus vim --text code-all --out vim
$G corpus mix --all
