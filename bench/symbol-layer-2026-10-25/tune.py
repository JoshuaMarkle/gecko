"""Small edits to a layered layout, scored with gecko.

python3 -I tune.py path  <layout.toml> <tag> <free|human> <steps> <out.json>
    Greedy: each step makes the single best swap of two Sym-layer keys (symbols or empty) and
    records the score; how many edits of a published layer reach most of the optimum.
python3 -I tune.py table <layout.toml> <tag> <free|human> <out.json>
    Every single swap of two Sym-layer keys and its score change (the menu for hand tuning).
python3 -I tune.py swaps <layout.toml> <tag> <free|human> <out.toml> "a b" "c d" ...
    Applies swaps in order (symbol names; `·` is not allowed, use `{e<k>}` for the empty key k),
    printing the score after each, and writes the result.
"""
import json, re, subprocess, sys, tempfile
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
GECKO = str(ROOT / "target/release/gecko")
E = "·"
TOKENS = re.compile(r"\{[a-z0-9]+\}|.")
CORPUS = {"prog": "programmer-typed", "vim": "vim-programmer-typed"}


def parse(path):
    text = Path(path).read_text()
    head, sym_part, num_part = text.split("[[layer]]")
    rows = lambda part: [r.replace('\\"', '"').replace("\\\\", "\\") for r in re.findall(r'"((?:[^"\\]|\\.)*)"', part.split("rows =")[1])]
    sym = [k for r in rows(sym_part) for k in TOKENS.findall(r)]
    num = [k for r in rows(num_part) for k in TOKENS.findall(r)]
    return head, sym, num


def render(head, sym, num):
    esc = lambda s: s.replace("\\", "\\\\").replace('"', '\\"')
    rws = lambda ks: "[" + ", ".join('"' + esc("".join(ks[a:b])) + '"' for a, b in ((0, 10), (10, 20), (20, 30), (30, 36))) + "]"
    return f'{head.rstrip()}\n\n[[layer]]\nname = "sym"\nrows = {rws(sym)}\n\n[[layer]]\nname = "num"\nrows = {rws(num)}\n'


def score(text, tag, kind):
    with tempfile.NamedTemporaryFile("w", suffix=".toml", delete=False, dir=HERE / "layouts") as f:
        f.write(text)
    try:
        prof = HERE / "profiles" / f"{kind}-{'vim' if tag.endswith('vim') else 'prog'}.toml"
        cmd = [GECKO, "--json", "score", f.name, "-g", "corne-36", "-c", CORPUS["vim" if tag.endswith("vim") else "prog"], "-p", str(prof)]
        for attempt in range(2):
            r = subprocess.run(cmd, capture_output=True, text=True, cwd=ROOT)
            if r.returncode == 0:
                return json.loads(r.stdout)["breakdown"]["score"]
        raise RuntimeError(f"gecko score failed: {r.stderr.strip()}")
    finally:
        Path(f.name).unlink()


def path(layout, tag, kind, steps, out):
    head, sym, num = parse(layout)
    slots = list(range(33))  # Sym layer keys the layer can use (right thumbs belong to the holder)
    cur = score(render(head, sym, num), tag, kind)
    log = [{"step": 0, "score": cur, "swap": None}]
    print(f"0: {cur:.3f}")
    for step in range(1, steps + 1):
        cands = [(a, b) for i, a in enumerate(slots) for b in slots[i + 1:] if not (sym[a] == E and sym[b] == E)]

        def trial(ab):
            a, b = ab
            s = sym[:]
            s[a], s[b] = s[b], s[a]
            return score(render(head, s, num), tag, kind), ab

        with ThreadPoolExecutor(16) as ex:
            best, (a, b) = min(ex.map(trial, cands))
        if best >= cur - 1e-9:
            break
        what = f"{sym[a]}↔{sym[b]}"
        sym[a], sym[b] = sym[b], sym[a]
        cur = best
        log.append({"step": step, "score": cur, "swap": what})
        print(f"{step}: {cur:.3f}  {what}", flush=True)
    json.dump({"layout": str(layout), "tag": tag, "kind": kind, "path": log, "sym": sym}, open(out, "w"), indent=1, ensure_ascii=False)


def table(layout, tag, kind, out):
    head, sym, num = parse(layout)
    cur = score(render(head, sym, num), tag, kind)
    cands = [(a, b) for a in range(33) for b in range(a + 1, 33) if not (sym[a] == E and sym[b] == E)]

    def trial(ab):
        a, b = ab
        s = sym[:]
        s[a], s[b] = s[b], s[a]
        return {"swap": f"{sym[a]} {sym[b]}", "keys": [a, b], "delta": score(render(head, s, num), tag, kind) - cur}

    with ThreadPoolExecutor(16) as ex:
        rows = sorted(ex.map(trial, cands), key=lambda r: r["delta"])
    json.dump({"layout": str(layout), "score": cur, "swaps": rows}, open(out, "w"), indent=1, ensure_ascii=False)
    for r in rows[:25]:
        print(f"{r['swap']:>14}  {r['delta']:+.3f}")


def swaps(layout, tag, kind, out, pairs):
    head, sym, num = parse(layout)
    print(f"start: {score(render(head, sym, num), tag, kind):.3f}")
    for p in pairs:
        a, b = p.split()
        ia = int(a[2:-1]) if a.startswith("{e") else sym.index(a)
        ib = int(b[2:-1]) if b.startswith("{e") else sym.index(b)
        sym[ia], sym[ib] = sym[ib], sym[ia]
        print(f"{p}: {score(render(head, sym, num), tag, kind):.3f}")
    Path(out).parent.mkdir(parents=True, exist_ok=True)
    Path(out).write_text(render(head, sym, num))


if __name__ == "__main__":
    cmd, *a = sys.argv[1:]
    if cmd == "path":
        path(a[0], a[1], a[2], int(a[3]), a[4])
    elif cmd == "table":
        table(a[0], a[1], a[2], a[3])
    elif cmd == "swaps":
        swaps(a[0], a[1], a[2], a[3], a[4:])
