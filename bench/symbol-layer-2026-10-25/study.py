"""Symbol-layer study driver (README). Run from anywhere; gecko is ../../target/release/gecko.

python3 -I study.py complete               # fill each reference's missing symbols → layouts/<tag>/<name>.c.toml
python3 -I study.py search <cell> <start> <profile> <corpus> [seeds=6] [evals=10000000] [extra pins]
                                           # seeds × BLS, letters/digits/fillers pinned → seeds-<cell>.json,
                                           # best layout → layouts/best/<cell>.toml
python3 -I study.py score <out.json>       # every reference (completed) and best layout, every profile
"""
import json, string, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
GECKO = str(ROOT / "target/release/gecko")
TOK = {"⏎": "{enter}", "␣": "{space}", "⌫": "{bspc}", "⇧": "{shift}", "①": "{layer1}", "②": "{layer2}", "⎋": "{esc}"}
TAGS = {"us": "programmer-typed", "us-vim": "vim-programmer-typed", "ltgt": "programmer-typed", "ltgt-vim": "vim-programmer-typed"}


def gecko(*args):
    out = subprocess.run([GECKO, "--json", *args], check=True, capture_output=True, text=True, cwd=ROOT).stdout
    return json.loads(out)


def profile(tag, kind):
    return str(HERE / "profiles" / f"{kind}-{'vim' if tag.endswith('vim') else 'prog'}.toml")


def pins_of(layout_path):
    """Letters, base punctuation, digits, fillers and the base thumbs: everything that is not a Sym symbol."""
    text = Path(layout_path).read_text()
    vim = "{esc}" in text.split("[[layer]]")[0]
    num = text.split("[[layer]]")[2]
    keep = set(string.ascii_lowercase + "',./0123456789")
    keep |= {c for c in num if "α" <= c <= "Ͽ"}
    return "".join(sorted(keep)) + "{space}{bspc}" + ("{esc}" if vim else "{enter}")


def write_layout(run_id, out, name):
    d = ROOT / "runs" / run_id
    rows = json.load(open(d / "best.json"))["best"][0]["rows"]
    cfg = json.load(open(d / "config.json"))
    shifted = cfg.get("shifted", [])
    row = lambda cs: json.dumps("".join(TOK.get(c, c) for c in cs), ensure_ascii=False)
    rws = lambda i: "[" + ", ".join(row(r) for r in rows[4 * i:4 * i + 4]) + "]"
    Path(out).parent.mkdir(parents=True, exist_ok=True)
    Path(out).write_text(f'''name = {json.dumps(name, ensure_ascii=False)}
geometry = "corne-36"
source = "bench/symbol-layer-2026-10-25/study.py (run {run_id})"
rows = {rws(0)}
shifted = {json.dumps(shifted, ensure_ascii=False)}

[[layer]]
name = "sym"
rows = {rws(1)}

[[layer]]
name = "num"
rows = {rws(2)}
''')


def optimize(start, prof, corpus, pins, seed, evals):
    run = gecko("optimize", "--algo", "bls", "--evals", str(evals), "--seed", str(seed), "--threads", "4",
                "-p", prof, "-g", "corne-36", "-c", corpus, "--from", str(start), "--pin", pins, "--top", "1")["run"]
    return {"seed": seed, "run": run["id"], "score": run["best"][0]["score"], "rows": run["best"][0]["rows"]}


def complete():
    meta = json.load(open(HERE / "layouts/refs.json"))
    jobs = []
    for key, m in meta.items():
        tag, name = key.split("/")
        src = HERE / "layouts" / tag / f"{name}.toml"
        jobs.append((tag, name, src, m))

    def one(job):
        tag, name, src, m = job
        out = HERE / "layouts" / tag / f"{name}.c.toml"
        if not m["missing"]:
            out.write_text(src.read_text())
            return key_line(tag, name, "complete")
        r = optimize(src, profile(tag, "free"), TAGS[tag], m["pins"], 1, 2_000_000)
        write_layout(r["run"], out, f"{name} ({tag}, completed: {m['missing']})")
        return key_line(tag, name, f"placed {m['missing']} (run {r['run']})")

    with ThreadPoolExecutor(5) as ex:
        for line in ex.map(one, jobs):
            print(line)


def key_line(tag, name, what):
    return f"{tag}/{name}: {what}"


def search(cell, start, prof, corpus, seeds=6, evals=10_000_000, extra=""):
    pins = pins_of(start) + extra
    with ThreadPoolExecutor(5) as ex:
        rows = list(ex.map(lambda s: optimize(start, prof, corpus, pins, s, evals), range(1, seeds + 1)))
    best = min(rows, key=lambda r: r["score"])
    at = sum(abs(r["score"] - best["score"]) < 1e-6 for r in rows)
    json.dump({"cell": cell, "start": str(start), "profile": prof, "corpus": corpus, "pins": pins, "evals": evals,
               "best": best["score"], "at_best": at, "rows": rows}, open(HERE / f"seeds-{cell}.json", "w"), indent=1, ensure_ascii=False)
    write_layout(best["run"], HERE / "layouts/best" / f"{cell}.toml", f"{cell} (best of {seeds} seeds)")
    print(f"{cell}: best {best['score']:.4f}, {at}/{seeds} seeds at best, worst {max(r['score'] for r in rows):.4f}")


def score_all(out):
    res = {}
    layouts = sorted((HERE / "layouts").glob("*/*.c.toml")) + sorted((HERE / "layouts/best").glob("*.toml"))
    layouts += sorted((HERE / "layouts/tuned").glob("*.toml")) if (HERE / "layouts/tuned").exists() else []

    def one(p):
        text = p.read_text()
        tag = p.parent.name
        if tag in ("best", "tuned"):
            tag = p.stem.split(".")[0].split("-")[0] + ("-vim" if "vim" in p.stem else "")
        r = {}
        for kind in ("free", "human"):
            b = gecko("score", str(p), "-g", "corne-36", "-c", TAGS[tag], "-p", profile(tag, kind))["breakdown"]
            r[kind] = {"score": b["score"], "metrics": {m["id"]: [m["value"], m.get("weighted")] for m in b["metrics"]}}
        return str(p.relative_to(HERE)), {"tag": tag, **r}

    with ThreadPoolExecutor(8) as ex:
        res = dict(ex.map(one, layouts))
    json.dump(res, open(HERE / out, "w"), indent=1, ensure_ascii=False)
    print(f"{len(res)} layouts → {out}")


if __name__ == "__main__":
    cmd, *a = sys.argv[1:]
    if cmd == "complete":
        complete()
    elif cmd == "search":
        cell, start, prof, corpus = a[:4]
        search(cell, Path(start).resolve(), str(Path(prof).resolve()), corpus,
               int(a[4]) if len(a) > 4 else 6, int(a[5]) if len(a) > 5 else 10_000_000, a[6] if len(a) > 6 else "")
    elif cmd == "score":
        score_all(a[0])
