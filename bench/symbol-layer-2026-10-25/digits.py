"""Digits on the Sym layer: a start layout with the digits in a fixed shape there.

usage: python3 -I digits.py <start.toml> <row|pad> <out.toml>

row: 1 2 3 4 5 | 6 7 8 9 0 on the Sym home row.
pad: a numpad on the left hand (7 8 9 / 4 5 6 / 1 2 3 on ring, middle, index; 0 on the left middle
     thumb), the Num layer's positions moved onto Sym.
Symbols displaced by a digit move to free keys; the Num layer is emptied (no fillers), so the
search may send symbols that no longer fit there (24 symbols + 10 digits > 33 Sym keys).
"""
import re, sys
from pathlib import Path

E = "·"
SHAPES = {
    "row": {10 + i: d for i, d in enumerate("1234567890")},
    "pad": {1: "7", 2: "8", 3: "9", 11: "4", 12: "5", 13: "6", 21: "1", 22: "2", 23: "3", 31: "0"},
}
TOKENS = re.compile(r"\{[a-z0-9]+\}|.")


def keys(row):
    return TOKENS.findall(row)


start, shape, out = sys.argv[1:]
text = Path(start).read_text()
head, sym_part, num_part = text.split("[[layer]]")
sym_rows = re.findall(r'"((?:[^"\\]|\\.)*)"', sym_part.split("rows =")[1])
unesc = lambda s: s.replace('\\"', '"').replace("\\\\", "\\")
sym = [k for r in sym_rows for k in keys(unesc(r))]
assert len(sym) == 36, len(sym)
num_rows = re.findall(r'"((?:[^"\\]|\\.)*)"', num_part.split("rows =")[1])
num = [k for r in num_rows for k in keys(unesc(r))]
# Anything on the Num layer that is not a digit or a filler comes along to Sym (Miryoku's extras).
extra = [c for c in num if c != E and not c.isdigit() and not ("α" <= c <= "Ͽ")]
displaced = [sym[k] for k in SHAPES[shape] if sym[k] != E] + extra
for k, d in SHAPES[shape].items():
    sym[k] = d
num = [E] * 36
free_sym = [k for k in range(33) if sym[k] == E]
free_num = [k for k in range(33)]
for c in displaced:
    if free_sym:
        sym[free_sym.pop(0)] = c
    else:
        num[free_num.pop(0)] = c
esc = lambda s: s.replace("\\", "\\\\").replace('"', '\\"')
row = lambda ks: '"' + esc("".join(ks)) + '"'
rows = lambda ks: "[" + ", ".join(row(ks[a:b]) for a, b in ((0, 10), (10, 20), (20, 30), (30, 36))) + "]"
head = re.sub(r'name = "([^"]*)"', lambda m: f'name = "{m.group(1)} + digits {shape}"', head, count=1)
Path(out).parent.mkdir(parents=True, exist_ok=True)
Path(out).write_text(f'''{head.rstrip()}

[[layer]]
name = "sym"
rows = {rows(sym)}

[[layer]]
name = "num"
rows = {rows(num)}
''')
print(out, "displaced:", "".join(displaced))
