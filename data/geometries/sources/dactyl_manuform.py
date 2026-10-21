"""3D poses for dactyl-manuform-5x6.toml from the Dactyl-Manuform generator's own math.

Port of the key placement in tshort/dactyl-keyboard `src/dactyl_keyboard/dactyl.clj`
(Dactyl-Manuform, 5 rows x 6 columns, default parameters): the concave keywell
(row curvature alpha, column curvature beta, per-column offsets), 15 deg tenting, and the
six-key thumb fan. Only poses are written: fingers, rows, columns, efforts and fills of
the existing slots are kept.

Usage (from the repo root): python3 data/geometries/sources/dactyl_manuform.py
Conventions: `context/geometry.md` (3D) and `crates/gecko-core/src/pose.rs`.
"""

import math
import pathlib
import re

U = 19.05  # mm per key unit
CAP = 0.42  # keycap height in u (pose::CAP_HEIGHT)

# --- Dactyl-Manuform parameters (right half, mm, Dactyl axes: x right, y away, z up) ---
NROWS, NCOLS = 5, 6
ALPHA = math.pi / 12  # column curvature: angle between rows
BETA = math.pi / 36  # row curvature: angle between columns
CENTERROW = NROWS - 3  # home row stays level
CENTERCOL = 4  # pinky column stays level (higher = more tenting of the inner columns)
TENTING = math.pi / 12
Z_OFFSET = 9.0
KEYSWITCH = 14.4
PLATE = 4.0
MOUNT = KEYSWITCH + 3.0
EXTRA_W, EXTRA_H = 2.5, 1.0
SA_HEIGHT = 12.7
CAP_TOP = PLATE + SA_HEIGHT  # finger contact height above the switch origin
ROW_R = (MOUNT + EXTRA_H) / 2 / math.sin(ALPHA / 2) + CAP_TOP
COL_R = (MOUNT + EXTRA_W) / 2 / math.sin(BETA / 2) + CAP_TOP
THUMB_OFFSETS = (6.0, -3.0, 7.0)


def column_offset(col):
    if col == 2:
        return (0.0, 2.82, -4.5)
    if col >= 4:
        return (0.0, -12.0, 5.64)
    return (0.0, 0.0, 0.0)


def rx(a):
    c, s = math.cos(a), math.sin(a)
    return [[1, 0, 0], [0, c, -s], [0, s, c]]


def ry(a):
    c, s = math.cos(a), math.sin(a)
    return [[c, 0, s], [0, 1, 0], [-s, 0, c]]


def rz(a):
    c, s = math.cos(a), math.sin(a)
    return [[c, -s, 0], [s, c, 0], [0, 0, 1]]


def mul(a, b):
    return [[sum(a[i][k] * b[k][j] for k in range(3)) for j in range(3)] for i in range(3)]


def app(m, v):
    return [sum(m[i][k] * v[k] for k in range(3)) for i in range(3)]


def add(a, b):
    return [x + y for x, y in zip(a, b)]


I3 = [[1, 0, 0], [0, 1, 0], [0, 0, 1]]


class Pose:
    """A rigid transform: point -> m @ point + t (Dactyl's chained translate/rotate)."""

    def __init__(self, m=I3, t=(0, 0, 0)):
        self.m, self.t = m, list(t)

    def then_translate(self, v):
        return Pose(self.m, add(self.t, v))

    def then_rotate(self, r):
        return Pose(mul(r, self.m), app(r, self.t))

    def point(self, p):
        return add(app(self.m, p), self.t)


def key_place(col, row):
    p = Pose().then_translate((0, 0, -ROW_R)).then_rotate(rx(ALPHA * (CENTERROW - row))).then_translate((0, 0, ROW_R))
    p = p.then_translate((0, 0, -COL_R)).then_rotate(ry(BETA * (CENTERCOL - col))).then_translate((0, 0, COL_R))
    p = p.then_translate(column_offset(col))
    return p.then_rotate(ry(TENTING)).then_translate((0, 0, Z_OFFSET))


THUMB_ORIGIN = add(key_place(1, NROWS - 2).point((MOUNT / 2, -MOUNT / 2, 0)), THUMB_OFFSETS)
D = math.radians
THUMBS = {  # name: (x, y, z rotations in degrees, applied in that order; translation)
    "tr": ((10, -23, 10), (-12, -16, 3)),
    "tl": ((10, -23, 10), (-32, -15, -2)),
    "mr": ((-6, -34, 48), (-29, -40, -13)),
    "ml": ((6, -34, 40), (-51, -25, -12)),
    "br": ((-16, -33, 54), (-37.8, -55.3, -25.3)),
    "bl": ((-4, -35, 52), (-56.3, -43.3, -23.5)),
}


def thumb_place(name):
    (ax, ay, az), off = THUMBS[name]
    p = Pose().then_rotate(rx(D(ax))).then_rotate(ry(D(ay))).then_rotate(rz(D(az)))
    return p.then_translate(THUMB_ORIGIN).then_translate(off)


# --- Dactyl (mm, y away) -> GECKO board (u, y toward the user) ---
F = [[1, 0, 0], [0, -1, 0], [0, 0, 1]]


def to_board(pose):
    """Keycap top centre (u) and rotation matrix in board axes."""
    top = pose.point((0, 0, CAP_TOP))
    return [top[0] / U, -top[1] / U, top[2] / U], mul(mul(F, pose.m), F)


def euler(m):
    """Key::rot of a board rotation matrix: M = Rx(rx) Rz(rz) Ry(ry) (see pose.rs)."""
    rzv = math.asin(max(-1.0, min(1.0, -m[0][1])))
    return [math.degrees(math.atan2(-m[2][1], m[1][1])), math.degrees(math.atan2(-m[0][2], m[0][0])), math.degrees(rzv)]


def board_rot(deg):
    """Board-axis rotation matrix of Key::rot (pose::rotation)."""
    a, b, c = (math.radians(v) for v in deg)
    bx = [[1, 0, 0], [0, math.cos(a), math.sin(a)], [0, -math.sin(a), math.cos(a)]]
    bz = [[math.cos(c), -math.sin(c), 0], [math.sin(c), math.cos(c), 0], [0, 0, 1]]
    by = [[math.cos(b), 0, -math.sin(b)], [0, 1, 0], [math.sin(b), 0, math.cos(b)]]
    return mul(mul(bx, bz), by)


def pos_from_top(top, m):
    t = app(m, (0, 0, CAP / 2))
    return [top[0] - t[0] - 0.5, top[1] - t[1] - 0.5, top[2] - t[2] - CAP / 2]


# Right-half slots (row, col) of the existing file -> Dactyl placement.
RIGHT = {}
for r in range(4):  # Dactyl rows 0..3 = GECKO rows -1..2; Dactyl col c = GECKO col 6 + c
    for c in range(NCOLS):
        RIGHT[(r - 1, 6 + c)] = key_place(c, r)
RIGHT[(3, 14)] = key_place(2, 4)  # bottom row: middle
RIGHT[(3, 15)] = key_place(3, 4)  # bottom row: ring
for slot, name in {(3, 13): "tr", (3, 12): "tl", (3, 10): "mr", (3, 8): "ml", (3, 11): "br", (3, 9): "bl"}.items():
    RIGHT[slot] = thumb_place(name)
LEFT_OF = {**{(r, 6 + c): (r, 5 - c) for r in range(-1, 3) for c in range(NCOLS)},
           (3, 14): (3, 1), (3, 15): (3, 0),
           (3, 13): (3, 2), (3, 12): (3, 3), (3, 10): (3, 4), (3, 11): (3, 5), (3, 8): (3, 6), (3, 9): (3, 7)}

AXIS = 8.5  # mirror plane x, as in the QMK projection
HALF_GAP = 3.0  # inner (stretch) home keycaps sit AXIS ± HALF_GAP, 6u apart like QMK

poses = {}
for slot, place in RIGHT.items():
    top, m = to_board(place)
    poses[slot] = (top, euler(m))
ref = poses[(1, 6)][0]
dx = AXIS + HALF_GAP - ref[0]
for slot, (top, rot) in list(poses.items()):
    top[0] += dx
    l_top = [2 * AXIS - top[0], top[1], top[2]]
    poses[slot] = (top, rot)
    poses[LEFT_OF[slot]] = (l_top, [rot[0], -rot[1], -rot[2]])
keys = {s: pos_from_top(t, board_rot(r)) + [r] for s, (t, r) in poses.items()}
# Board origin: the top-most key edge at y = 0 and the lowest keycap bottom at z = 0.
y0 = min(k[1] for k in keys.values())
z0 = min(k[2] for k in keys.values())


def num(v):
    v = round(v, 4)
    return f"{v:.4f}".rstrip("0").rstrip(".") if v != 0 else "0.0"


path = pathlib.Path(__file__).resolve().parents[1] / "dactyl-manuform-5x6.toml"
text = path.read_text()
head, *blocks = re.split(r"\n(?=\[\[key\]\])", text)
out = []
for block in blocks:
    row = int(re.search(r"^row = (-?\d+)", block, re.M).group(1))
    col = int(re.search(r"^col = (-?\d+)", block, re.M).group(1))
    x, y, z, rot = keys[(row, col)]
    lines = [ln for ln in block.strip("\n").split("\n") if not ln.startswith(("pos =", "rot ="))]
    pos = f"pos = [{num(x)}, {num(y - y0)}, {num(z - z0)}]"
    rot = f"rot = [{num(rot[0])}, {num(rot[1])}, {num(rot[2])}]"
    out.append("\n".join([lines[0], pos, rot] + lines[1:]))
path.write_text(head.rstrip("\n") + "\n\n" + "\n\n".join(out) + "\n")
print(f"wrote {len(out)} keys to {path}")
