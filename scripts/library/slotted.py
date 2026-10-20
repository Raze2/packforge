# Generates FEFCO 02xx slotted-container component files from one template.
import sys, textwrap
out = sys.argv[1]

COMMON_PARAMS = '''
[[param]]
id = "L"
label = "Inside length"
type = "length"
default = 300
min = 40
max = 2500
group = "Dimensions"

[[param]]
id = "W"
label = "Inside width"
type = "length"
default = 200
min = 40
max = 2500
group = "Dimensions"

[[param]]
id = "H"
label = "Inside height"
type = "length"
default = 150
min = 20
max = 2500
group = "Dimensions"
'''

CONSTRUCTION = '''
[[param]]
id = "glue_width"
label = "Glue flap width"
type = "length"
default = 35
min = 15
max = 60
group = "Construction"

[[param]]
id = "glue_side"
label = "Glue flap side"
type = "enum"
values = ["left", "right"]
default = "left"
group = "Construction"
'''

BASE_DERIVED = '''t = "board.caliper"
pL = "L + t"                    # length panel width (crease to crease)
pW = "W + t"                    # width panel width
pH = "H + 2 * t"                # body height (score to score)
s = "max(1.5 * t, 5mm)"         # slot width between flaps
hs = "s / 2"
g = "glue_width"
taper = "min(g, 15mm, pH / 4)"  # chamfer on the glue flap ends
X0 = "g"
X1 = "X0 + pL"
X2 = "X1 + pW"
X3 = "X2 + pL"
X4 = "X3 + pW"
fmax = "max(fS, fE)"
yb = "fmax"                     # bottom score line
yt = "yb + pH"                  # top score line
bS = "yb - fS"                  # bottom edge of side-panel flaps
bE = "yb - fE"                  # bottom edge of end-panel flaps
tS = "yt + fS"                  # top edge of side-panel flaps
tE = "yt + fE"                  # top edge of end-panel flaps
'''

def q(s): return f'"{s}"'
def pt(x, y): return f'[{q(x)}, {q(y)}]'

def outline(top):
    pts = [pt("0", "yb + taper"), pt("0", "yt - taper"), pt("X0", "yt")]
    if top:
        for (a, b, f) in [("X0", "X1", "tS"), ("X1", "X2", "tE"), ("X2", "X3", "tS"), ("X3", "X4", "tE")]:
            pts += [pt(f"{a} + hs", "yt"), pt(f"{a} + hs", f), pt(f"{b} - hs", f), pt(f"{b} - hs", "yt")]
        pts.append(pt("X4", "yt"))
    else:
        pts.append(pt("X4", "yt"))
    pts.append(pt("X4", "yb"))
    for (a, b, f) in [("X4", "X3", "bE"), ("X3", "X2", "bS"), ("X2", "X1", "bE"), ("X1", "X0", "bS")]:
        pts += [pt(f"{a} - hs", "yb"), pt(f"{a} - hs", f), pt(f"{b} + hs", f), pt(f"{b} + hs", "yb")]
    pts.append(pt("X0", "yb"))
    # Drop the duplicated slot corners at panel joints (consecutive equal points are skipped by the
    # engine anyway, but keep the file tidy).
    body = ",\n  ".join(pts)
    return f'''[[path]]
type = "cut"
closed = true
points = [
  {body},
]
'''

def line(a, b):
    return f'''[[line]]
type = "crease"
from = {a}
to = {b}
'''

def creases(top):
    out = []
    for x in ["X0", "X1", "X2", "X3"]:
        out.append(line(pt(x, "yb"), pt(x, "yt")))
    rows = ["yb"] + (["yt"] if top else [])
    for y in rows:
        for (a, b) in [("X0", "X1"), ("X1", "X2"), ("X2", "X3"), ("X3", "X4")]:
            out.append(line(pt(f"{a} + hs", y), pt(f"{b} - hs", y)))
    return "\n".join(out)

def panel(i, x, y, w, h):
    return f'''[[panel]]
id = "{i}"
x = {q(x)}
y = {q(y)}
w = {q(w)}
h = {q(h)}
'''

def hinge(a, b, ax):
    return f'''[[hinge]]
a = "{a}"
b = "{b}"
axis = [{", ".join(q(v) for v in ax)}]
closed_angle = 90
'''

def panels_and_hinges(top):
    out = [panel("glue", "0", "yb", "g", "pH"),
           panel("side1", "X0", "yb", "pL", "pH"), panel("end1", "X1", "yb", "pW", "pH"),
           panel("side2", "X2", "yb", "pL", "pH"), panel("end2", "X3", "yb", "pW", "pH")]
    spec = [("1", "X0", "X1", "pL", "fS", "side1"), ("2", "X1", "X2", "pW", "fE", "end1"),
            ("3", "X2", "X3", "pL", "fS", "side2"), ("4", "X3", "X4", "pW", "fE", "end2")]
    for (n, a, b, w, f, body) in spec:
        out.append(panel(f"b{n}", f"{a} + hs", f"yb - {f}", f"{w} - s", f))
        if top:
            out.append(panel(f"t{n}", f"{a} + hs", "yt", f"{w} - s", f))
    hs_ = [hinge("side1", "glue", ["X0", "yb", "X0", "yt"]), hinge("side1", "end1", ["X1", "yb", "X1", "yt"]),
           hinge("end1", "side2", ["X2", "yb", "X2", "yt"]), hinge("side2", "end2", ["X3", "yb", "X3", "yt"])]
    for (n, a, b, w, f, body) in spec:
        hs_.append(hinge(body, f"b{n}", [f"{a} + hs", "yb", f"{b} - hs", "yb"]))
        if top:
            hs_.append(hinge(body, f"t{n}", [f"{a} + hs", "yt", f"{b} - hs", "yt"]))
    return "\n".join(out) + "\n" + "\n".join(hs_)

STYLES = {
  "0200": dict(name="Half slotted container", tier="free", top=False,
     desc="Like FEFCO 0201 without top flaps: an open-top box closed by a separate lid or cover.",
     flaps='fS = "pW / 2"\nfE = "pW / 2"\n', extra_params="", rules=""),
  "0201": None,  # hand-written file kept as is
  "0202": dict(name="Overlapping slotted container", tier="premium", top=True,
     desc="Slotted box whose outer flaps overlap in the middle by an adjustable amount.",
     flaps='fS = "pW / 2 + overlap / 2"   # outer flaps overlap by `overlap`\nfE = "fS"\n',
     extra_params='''
[[param]]
id = "overlap"
label = "Flap overlap"
type = "length"
default = 30
min = 5
max = 500
group = "Construction"
''',
     rules='''
[[rule]]
when = "overlap > pW"
level = "error"
message = "The flap overlap cannot exceed the box width."
'''),
  "0203": dict(name="Full overlap slotted container", tier="premium", top=True,
     desc="Slotted box whose outer flaps each span the full width for extra stacking strength.",
     flaps='fS = "pW"                     # flaps span the full width\nfE = "pW"\n', extra_params="", rules='''
[[rule]]
when = "pW > pL"
level = "warning"
message = "Full-width flaps are longer than the length panel; check the end flaps clear each other."
'''),
  "0204": dict(name="Centre special slotted container", tier="premium", top=True,
     desc="Slotted box where both the outer and the inner flaps meet in the centre (full flap coverage top and bottom).",
     flaps='fS = "pW / 2"                 # side-panel flaps meet in the centre\nfE = "pL / 2"                 # end-panel flaps meet in the centre too\n',
     extra_params="", rules=""),
}

for code, st in STYLES.items():
    if st is None:
        continue
    top = st["top"]
    header = f'''# FEFCO {code} — {st["name"]}
#
# Generated layout (glue flap on the left; mirrored when glue_side = "right"):
# four body panels side1 | end1 | side2 | end2 with {"top and bottom" if top else "bottom"} flaps.
# Side-panel flaps are fS high, end-panel flaps fE high.
#
# Allowances are NOMINAL starting values (inside dimensions + one caliper per panel,
# two calipers on the depth). Verify against physical samples before marking stable.

[component]
id = "fefco-{code}"
version = "0.1.0"
name = "{st["name"]}"
standard = "FEFCO {code}"
category = "02-slotted"
tier = "{st["tier"]}"
dimension_reference = "inside"
description = "{st["desc"]}"
'''
    rules = '''
[[rule]]
when = "W > L"
level = "warning"
message = "Width is larger than length; length is normally the longer side."
''' + st["rules"]
    doc = (header + COMMON_PARAMS + CONSTRUCTION + st["extra_params"] + "\n[derived]\n" + BASE_DERIVED + st["flaps"]
           + rules + '\n[layout]\nmirror_x = "glue_side == \'right\'"\n\n'
           + "# ── Cut outline ──\n" + outline(top) + "\n# ── Creases ──\n" + creases(top)
           + "\n# ── Panels and hinges ──\n" + panels_and_hinges(top))
    open(f"{out}/{code}.pfc.toml", "w").write(doc)
    print("wrote", code)
