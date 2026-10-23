#!/usr/bin/env bash
# Usage: build_pdf.sh [paper.tex]   (default: journal.tex next to this script)
# Builds <paper>.pdf beside the .tex. gecko-paper.cls is found in this script's directory,
# figures (assets/<name>.pdf) relative to the .tex. Temporary LaTeX files stay out of the tree.
set -euo pipefail
templates=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
tex=${1:-$templates/journal.tex}
[[ -f $tex ]] || { echo "No such file: $tex" >&2; exit 1; }
dir=$(cd -- "$(dirname -- "$tex")" && pwd)
job=$(basename -- "$tex" .tex)
cd -- "$dir"
if ! command -v xelatex >/dev/null 2>&1; then
  echo 'XeLaTeX is required (TeX Live package: xetex).' >&2
  exit 1
fi
# Every \paperfigure{name} or \paperwidefigure{name} needs assets/name.pdf (made by the paper's figure script).
while read -r figure; do
  if [[ ! -f "assets/$figure.pdf" ]]; then
    echo "Missing vector figure assets/$figure.pdf. Generate it first (see the figure script)." >&2
    exit 1
  fi
done < <(grep -v '^[[:space:]]*%' "$job.tex" | grep -oE '\\paper(wide)?figure{[^}]*}' | sed 's/.*{\(.*\)}/\1/')
paper_build_dir=$(mktemp -d "${TMPDIR:-/tmp}/gecko-paper.XXXXXX")
trap 'rm -rf -- "$paper_build_dir"' EXIT
export TEXINPUTS="$templates:${TEXINPUTS:-}"
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
    -output-directory="$paper_build_dir" "$job.tex" >"$paper_build_dir/console.log" 2>&1; then
    cat "$paper_build_dir/console.log" >&2
    cp "$paper_build_dir/$job.log" "$job-build-error.log" 2>/dev/null || true
    exit 1
  fi
done
if grep -E 'Overfull|Missing character' "$paper_build_dir/$job.log"; then
  echo 'Layout/font warnings found; inspect the PDF.' >&2
fi
cp "$paper_build_dir/$job.pdf" "$job.pdf"
printf 'Built %s/%s.pdf\n' "$PWD" "$job"
