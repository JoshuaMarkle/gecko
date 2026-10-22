#!/usr/bin/env bash
# Build from any working directory; keep temporary LaTeX files out of the source tree.
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
if ! command -v xelatex >/dev/null 2>&1; then
  echo 'XeLaTeX is required (TeX Live package: xetex).' >&2
  exit 1
fi
for figure in pca convergence metrics; do
  if [[ ! -f "assets/$figure.pdf" ]]; then
    echo 'Missing vector figures. Run: MPLCONFIGDIR=/tmp/gecko-mpl python generate_figures.py' >&2
    exit 1
  fi
done
paper_build_dir=$(mktemp -d "${TMPDIR:-/tmp}/gecko-paper.XXXXXX")
trap 'rm -rf -- "$paper_build_dir"' EXIT
paper_format_args=()
# Some installations ship xelatex.ini but do not register/build xelatex.fmt.
# Initialize a private format rather than modifying the user's TeX configuration.
if ! kpsewhich xelatex.fmt >/dev/null 2>&1; then
  echo 'Initializing a temporary XeLaTeX format...'
  if ! xetex -ini -etex -no-shell-escape -interaction=nonstopmode -halt-on-error \
    -jobname=xelatex -output-directory="$paper_build_dir" '\input xelatex.ini' \
    >"$paper_build_dir/format.log" 2>&1; then
    cat "$paper_build_dir/format.log" >&2
    exit 1
  fi
  paper_format_args=("-fmt=$paper_build_dir/xelatex.fmt")
fi
for pass in 1 2; do
  if ! xelatex "${paper_format_args[@]}" -no-shell-escape -interaction=nonstopmode -halt-on-error \
    -output-directory="$paper_build_dir" journal.tex >"$paper_build_dir/console.log" 2>&1; then
    cat "$paper_build_dir/console.log" >&2
    cp "$paper_build_dir/journal.log" journal-build-error.log 2>/dev/null || true
    exit 1
  fi
done
if grep -E 'Overfull|Missing character' "$paper_build_dir/journal.log"; then
  echo 'Layout/font warnings found; inspect the PDF.' >&2
fi
cp "$paper_build_dir/journal.pdf" journal.pdf
printf 'Built %s/journal.pdf\n' "$PWD"
