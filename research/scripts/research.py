"""Keep research/ consistent.

usage: python3 -I research/scripts/research.py lint     # exit 1 on any problem
       python3 -I research/scripts/research.py index    # rewrite the table in sources/README.md

Source summaries (research/sources/*.md) start with frontmatter; the index is generated from it.
Stdlib only.
"""
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
RES = ROOT / "research"
SRC = RES / "sources"

GROUPS = {
    "analyzers": "Analyzers and optimizers",
    "community": "Community knowledge and data",
    "human": "Human data and biomechanics",
    "optimization": "Optimization and ML",
    "geometry": "Geometry generators",
}
KINDS = {"tool", "paper", "data", "community"}
STATUS = {"read", "secondary", "verified"}
REQUIRED = ["group", "kind", "read", "status", "feeds", "take"]
START, END = "<!-- index:start -->", "<!-- index:end -->"


def frontmatter(path):
    text = path.read_text()
    m = re.match(r"---\n(.*?)\n---\n", text, re.S)
    if not m:
        return None
    return dict(line.split(": ", 1) for line in m.group(1).splitlines() if ": " in line)


def sources():
    return sorted(p for p in SRC.glob("*.md") if p.name != "README.md" and not p.name.startswith("_"))


def index_table():
    rows = {g: [] for g in GROUPS}
    for p in sources():
        fm = frontmatter(p) or {}
        if fm.get("group") in rows:
            rows[fm["group"]].append((p.name, fm))
    out = []
    for g, title in GROUPS.items():
        if not rows[g]:
            continue
        out += [f"### {title}", "", "| File | Take | Status |", "|---|---|---|"]
        out += [f"| [{n}]({n}) | {fm.get('take', '')} | {fm.get('status', '')} |" for n, fm in rows[g]]
        out.append("")
    return "\n".join(out).rstrip() + "\n"


def write_index():
    p = SRC / "README.md"
    text = p.read_text()
    head, rest = text.split(START, 1)
    tail = rest.split(END, 1)[1]
    p.write_text(f"{head}{START}\n\n{index_table()}\n{END}{tail}")


def tracked_text_files():
    out = subprocess.run(["git", "ls-files", "-z"], cwd=ROOT, capture_output=True, check=True).stdout
    for name in out.decode().split("\0"):
        f = ROOT / name
        if name and f.is_file() and f.suffix in {".md", ".rs", ".ts", ".tsx", ".toml", ".py", ".sh", ".html", ".tex"}:
            yield f


def lint():
    errors = []

    # 1. frontmatter on every source summary
    for p in sources():
        fm = frontmatter(p)
        if fm is None:
            errors.append(f"{p.relative_to(ROOT)}: missing frontmatter")
            continue
        for k in REQUIRED:
            if not fm.get(k):
                errors.append(f"{p.relative_to(ROOT)}: frontmatter lacks `{k}`")
        if fm.get("group") not in GROUPS:
            errors.append(f"{p.relative_to(ROOT)}: group `{fm.get('group')}` not in {sorted(GROUPS)}")
        if fm.get("kind") not in KINDS:
            errors.append(f"{p.relative_to(ROOT)}: kind `{fm.get('kind')}` not in {sorted(KINDS)}")
        if fm.get("status") not in STATUS:
            errors.append(f"{p.relative_to(ROOT)}: status `{fm.get('status')}` not in {sorted(STATUS)}")
        if not re.fullmatch(r"\d{4}-\d\d-\d\d", fm.get("read", "")):
            errors.append(f"{p.relative_to(ROOT)}: `read` must be YYYY-MM-DD")

    # 2. relative markdown links resolve; every research page is linked from some other research page
    linked = set()
    pages = [p for p in RES.rglob("*.md")]
    for p in pages:
        for target in re.findall(r"\]\(([^)\s]+)\)", p.read_text()):
            if re.match(r"[a-z]+:", target) or target.startswith("#"):
                continue
            dest = (p.parent / target.split("#")[0]).resolve()
            if not dest.exists():
                errors.append(f"{p.relative_to(ROOT)}: broken link {target}")
            elif dest != p.resolve():
                linked.add(dest)
    for p in pages:
        if p.name.startswith("_") or p == RES / "README.md":
            continue
        if p.resolve() not in linked:
            errors.append(f"{p.relative_to(ROOT)}: not linked from any research page")

    # 3. generated index is current
    idx = (SRC / "README.md").read_text()
    if START not in idx or END not in idx:
        errors.append("sources/README.md: missing index markers")
    elif idx.split(START, 1)[1].split(END, 1)[0].strip() != index_table().strip():
        errors.append("sources/README.md: index out of date (run `index`)")

    # 4. research/ paths cited anywhere in the repo exist
    for f in tracked_text_files():
        for ref in set(re.findall(r"research/[A-Za-z0-9_./-]*[A-Za-z0-9_/]", f.read_text())):
            if not (ROOT / ref).exists():
                errors.append(f"{f.relative_to(ROOT)}: cites missing path {ref}")

    # report
    verify = 0
    for p in pages:
        if p.name == "OPEN-QUESTIONS.md":
            continue
        verify += len(re.findall(r"\(verify|verify\)|\*\*verify", p.read_text()))
    print(f"{len(sources())} sources, {len(pages)} pages, {verify} (verify) markers outside OPEN-QUESTIONS.md")
    for e in errors:
        print("ERROR", e)
    return 1 if errors else 0


if __name__ == "__main__":
    cmd = sys.argv[1] if len(sys.argv) > 1 else "lint"
    if cmd == "index":
        write_index()
    elif cmd == "lint":
        sys.exit(lint())
    else:
        sys.exit(__doc__)
