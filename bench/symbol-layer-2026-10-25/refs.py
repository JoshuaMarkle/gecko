"""Reference symbol layers in one frame → layouts/<frame>/<name>.toml (+ completion pins).

usage: python3 -I refs.py

Frame (all designs): corne-36, Colemak-DH letters, thumbs Enter Space Shift | Sym Bspc Num (Vim
frames: Esc instead of Enter, Enter goes on the Sym layer). Sym is held by the right inner thumb,
Num by the right outer thumb. The Num layer is a fixed numpad on the left hand (7 8 9 / 4 5 6 /
1 2 3, 0 on the left middle thumb), so a design is judged on its symbols only.

Frames: `us` keeps the US Shift pairs ' " , < . > / ? on the base (24 symbols on Sym);
`ltgt` keeps only '" and /? (plain , and .), so < and > go on the Sym layer too (26).

The Num layer's non-digit keys are filled with distinct zero-frequency Greek letters and pinned,
so no symbol moves there. Right-thumb keys of both layers belong to the holding thumb; gecko
leaves them out.
A design that lacks a symbol of the frame, or keeps it where the frame puts it on the base, is
completed by search (`complete.sh`): its own symbols are pinned and the missing ones placed best.
"""
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
E = "·"
LETTERS = "qwfpbjluy" "arstgmneio" "zxcdvkh"
FILL = iter("αβγδεζηθικλμνξοπρστυφχψωϐϑϕϖϗϘϙϚϛϜϝϞϟϠϡ")
FRAMES = {
    "us": dict(base_punct="',./", pairs=["'\"", ",<", ".>", "/?"]),
    "ltgt": dict(base_punct="',./", pairs=["'\"", "/?"]),
}
ASCII = "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~"


def grid(left, right, thumbs=E * 3):
    """left/right: 3 strings of 5 keys (pinky → index on the left, index → pinky on the right)."""
    return ["".join(l + r for l, r in zip(left, right)), thumbs]


# Sym layers as published, mapped to 3×5 per hand (sources: research/sources/symbol-layers.md).
DESIGNS = {
    "getreuer": grid(["`<>-|", "!*/=&", "~+[]%"], ["^{}$·", "#();\"", "@:,.'"], "_\\·"),
    "miryoku": grid(["{&*·}", ":$%^+", "~!@#|"], [E * 5] * 3, "()_"),
    "neo3": grid(["·_[]^", "\\/{}*", "#$|~`"], ["!<>=&", "?()-:", "+%\"';"]),
    "ergol": grid(["^<>$%", "{()}=", "~[]_#"], ["@&*'`", "\\+-/\"", "|!;:?"]),
    "seniply": grid(["!@#$%", "····^", "··\\|&"], ["=`:;+", "*({[-", "~)}]_"]),
    "shelzuuz": grid(["`~#&|", "!_:=$", "%?*+\\"], ["^{}[]", "@()·;", "/-<>\""]),
    "beakl15": grid(["-<$>·", "\\(\")#", "·:*+·"], ["·[_];", "%{=}|", "·&^~·"]),
    "elixir": grid(["!@#$%", "+=()\"", "<|->\\"], ["^&*?'", ":][}{", "`_/~;"]),
    "sunaku": grid(["[{}]?", "^=_$*", "<|->/"], [E * 5] * 3, "%:@"),
    "hietala": grid(["~+*!·", "|{}-\\", ",<>%·"], ["·#@^.", "`?[]·", "/&()_"]),
    "moonlander": grid(["!@[]|", "#$()`", "%^{}~"], [E * 5] * 3),
    "us-layer": grid(["!@#$%", "-=[]\\", "_+{}|"], ["^&*()", ";`~:·", "·····"]),
}
# Miryoku keeps the symbols of its Num layer there (around the same numpad).
MIRYOKU_NUM = {0: "[", 4: "]", 10: ";", 14: "=", 20: "`", 24: "\\", 32: "-"}
NUMPAD = {1: "7", 2: "8", 3: "9", 11: "4", 12: "5", 13: "6", 21: "1", 22: "2", 23: "3", 31: "0"}


def arr(xs):
    return "[" + ", ".join(json.dumps(x, ensure_ascii=False) for x in xs) + "]"


def tok(c):
    return {"⏎": "{enter}", "⎋": "{esc}"}.get(c, c)


def layout(frame, vim, name, sym, num):
    f = FRAMES[frame]
    base = list(LETTERS[:9] + "'" + LETTERS[9:19] + LETTERS[19:] + ",./")
    first = "{esc}" if vim else "{enter}"
    thumbs = first + "{space}{shift}{layer1}{bspc}{layer2}"
    rows = ["".join(base[:10]), "".join(base[10:20]), "".join(base[20:30]), thumbs]
    lay = lambda s: ["".join(tok(c) for c in s[a:b]) for a, b in ((0, 10), (10, 20), (20, 30), (30, 36))]
    return f'''name = "{name} ({frame}{', vim' if vim else ''})"
geometry = "corne-36"
source = "bench/symbol-layer-2026-10-25/refs.py"
rows = {arr(rows)}
shifted = {arr(f["pairs"])}

[[layer]]
name = "sym"
rows = {arr(lay("".join(sym)))}

[[layer]]
name = "num"
rows = {arr(lay("".join(num)))}
'''


def build(frame, vim, name, design):
    """→ (toml text, pins, missing symbols). Pins: everything except the missing symbols and empties."""
    global FILL
    FILL = iter("αβγδεζηθικλμνξοπρστυφχψωϐϑϕϖϗϘϙϚϛϜϝϞϟϠϡ")
    f = FRAMES[frame]
    on_base = set(f["base_punct"]) | {p[1] for p in f["pairs"]}
    sym_set = [c for c in ASCII if c not in on_base]
    sym = [c if c not in on_base else E for c in design[0] + design[1]] + [E] * 3
    num = [E] * 36
    for k, d in NUMPAD.items():
        num[k] = d
    if name == "miryoku":
        for k, c in MIRYOKU_NUM.items():
            if c not in on_base:
                num[k] = c
    for k in range(33):  # 33–35: right thumb, the holder's (gecko leaves them out)
        if num[k] == E:
            num[k] = next(FILL)
    placed = set(sym) | set(num)
    missing = [c for c in sym_set if c not in placed]
    if vim:
        missing.append("⏎")
    # Missing symbols start on empty Sym keys (search moves them; `·` is movable).
    free = [k for k in range(33) if sym[k] == E]
    assert len(free) >= len(missing), (frame, name, missing)
    for k, c in zip(free, missing):
        sym[k] = c
    pins = "".join(tok(c) for c in sorted(set(LETTERS) | set("',./") | (placed - {E})))
    pins += "{space}{bspc}" + ("{esc}" if vim else "{enter}")
    dup = [c for c in sym + num if c != E and (sym + num).count(c) > 1]
    assert not dup, (name, dup)
    return layout(frame, vim, name, sym, num), pins, missing


if __name__ == "__main__":
    meta = {}
    for frame in FRAMES:
        for vim in (False, True):
            tag = frame + ("-vim" if vim else "")
            (HERE / "layouts" / tag).mkdir(parents=True, exist_ok=True)
            for name, d in DESIGNS.items():
                text, pins, missing = build(frame, vim, name, d)
                (HERE / "layouts" / tag / f"{name}.toml").write_text(text)
                meta[f"{tag}/{name}"] = {"pins": pins, "missing": "".join(missing)}
    (HERE / "layouts" / "refs.json").write_text(json.dumps(meta, ensure_ascii=False, indent=1))
    for k, v in meta.items():
        print(f"{k:24} missing {v['missing']}")
