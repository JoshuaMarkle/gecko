"""Reference symbol layers on corne-36 → layouts/<name>.toml (checked: every ASCII symbol and digit once).
Usage: python3 -I bench/programming-layout-2026-10-25/refs.py
Slots per layer: 30 alpha keys (3 rows × 10, left to right) then 6 thumbs (L outer → R outer).
Common thumbs (all designs): Enter, Space, Shift | Sym, Backspace, Num. Sym and Num are held by
the right thumb, as Miryoku holds its left-hand layers; `·` = empty (the holder's own thumb keys).
Base letters: Colemak-DH with ' , . / and their US Shift pairs (Miryoku's base).
GECKO cannot place a symbol twice, so designs that duplicate a key (Miryoku's `(` and `.`,
Getreuer's `, . '`) keep one copy; the other key is empty.
"""
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
ASCII = "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~" + "0123456789"
BASE = "qwfpbjluy'" "arstgmneio" "zxcdvkh,./"
THUMBS = "{enter}{space}{shift}{layer1}{bspc}{layer2}"
PAIRS = ["'\"", ",<", ".>", "/?"]
E = "·"
HOLD = E * 3 + E * 3  # layer thumbs: left free unless given, right held


def rows(left, right, thumbs_left=E * 3):
    """left/right: 3 strings of 5 keys each; returns the 36-slot layer string."""
    return "".join(l + r for l, r in zip(left, right)) + thumbs_left + E * 3


DESIGNS = {
    # Miryoku: Sym and Num on the left hand, numpad, Sym = shifted Num positions.
    "miryoku": dict(
        sym=rows(["{&*·}", ":$%^+", "~!@#|"], [E * 5] * 3, "()_"),
        num=rows(["[789]", ";456=", "`123\\"], [E * 5] * 3, "·0-"),
    ),
    # Getreuer v2 (3×5 part): index fingers on = and (; numpad on the left of Num.
    "getreuer": dict(
        sym=rows(["`<>-|", "!*·=&", "~+[]%"], ["^{}$·", "#();\"", "@:···"], "_\\·"),
        num=rows(["·987·", "·321·", "·654·"], [E * 5] * 3, "·0·"),
        pairs=["/?"],  # his " < > are on the layer; / stays on the base key
    ),
    # US-style: the shifted number row on the Sym top row, the rest of the US symbol keys below.
    "us-layer": dict(
        sym=rows(["!@#$%", "-=[]\\", "_+{}|"], ["^&*()", ";`~:·", "·····"], E * 3),
        num=rows(["·····", "12345", "·····"], ["·····", "67890", "·····"], E * 3),
    ),
}


def arr(xs):
    return "[" + ", ".join(json.dumps(x, ensure_ascii=False) for x in xs) + "]"


def toml(name, sym, num, pairs, mode=None):
    m = f'\nmode = "{mode}"' if mode else ""
    return f'''name = "{name} (Colemak-DH base)"
geometry = "corne-36"
source = "bench/programming-layout-2026-10-25/refs.py"
rows = {arr([BASE[:10], BASE[10:20], BASE[20:], THUMBS])}
shifted = {arr(pairs)}

[[layer]]
name = "sym"{m}
rows = {arr([sym[:10], sym[10:20], sym[20:30], sym[30:]])}

[[layer]]
name = "num"{m}
rows = {arr([num[:10], num[10:20], num[20:30], num[30:]])}
'''


(HERE / "layouts").mkdir(exist_ok=True)
for name, d in DESIGNS.items():
    pairs = d.get("pairs", PAIRS)
    placed = BASE + d["sym"] + d["num"] + "".join(p[1] for p in pairs)
    sym = [c for c in placed if c != E and not c.isalpha()]
    dup = sorted({c for c in sym if sym.count(c) > 1})
    missing = [c for c in ASCII if c not in sym]
    assert len(d["sym"]) == 36 and len(d["num"]) == 36, name
    assert not dup and not missing, (name, dup, missing)
    (HERE / "layouts" / f"{name}.toml").write_text(toml(name, d["sym"], d["num"], pairs))
    print(name, "ok")
