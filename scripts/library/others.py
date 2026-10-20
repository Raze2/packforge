# Generates sleeve, tray and tuck-end carton component files.
import sys
root = sys.argv[1]

def q(s): return f'"{s}"'
def P(x, y): return f'[{q(x)}, {q(y)}]'
def R(x, y, r): return f'{{ at = {P(x, y)}, round = {q(r)} }}'
def path(points, kind="cut", closed=True):
    body = ",\n  ".join(points)
    return f'[[path]]\ntype = "{kind}"\nclosed = {"true" if closed else "false"}\npoints = [\n  {body},\n]\n'
def crease(a, b, kind="crease"):
    return f'[[line]]\ntype = "{kind}"\nfrom = {a}\nto = {b}\n'
def panel(i, x, y, w, h):
    return f'[[panel]]\nid = "{i}"\nx = {q(x)}\ny = {q(y)}\nw = {q(w)}\nh = {q(h)}\n'
def hinge(a, b, ax, angle=90):
    return f'[[hinge]]\na = "{a}"\nb = "{b}"\naxis = [{", ".join(q(v) for v in ax)}]\nclosed_angle = {angle}\n'
def param(i, label, default, lo, hi, group="Dimensions", kind="length"):
    return f'[[param]]\nid = "{i}"\nlabel = "{label}"\ntype = "{kind}"\ndefault = {default}\nmin = {lo}\nmax = {hi}\ngroup = "{group}"\n'
GLUE_SIDE = '[[param]]\nid = "glue_side"\nlabel = "Glue flap side"\ntype = "enum"\nvalues = ["left", "right"]\ndefault = "left"\ngroup = "Construction"\n'
MIRROR = '[layout]\nmirror_x = "glue_side == \'right\'"\n'

# ── Sleeve ────────────────────────────────────────────────────────────────────
sleeve = f'''# Sleeve — four-panel tube with a glue flap, open at both ends.
# Used as an outer wrap or a band. Allowances are NOMINAL; verify with samples.

[component]
id = "sleeve"
version = "0.1.0"
name = "Sleeve"
category = "09-sleeves"
tier = "free"
dimension_reference = "inside"
description = "Four-panel tube open at both ends, glued along one edge."

{param("L", "Inside length", 300, 30, 2500)}
{param("W", "Inside width", 200, 30, 2500)}
{param("H", "Sleeve depth", 150, 20, 2500)}
{param("glue_width", "Glue flap width", 35, 10, 60, "Construction")}
{GLUE_SIDE}
[derived]
t = "board.caliper"
pL = "L + t"
pW = "W + t"
pH = "H"
g = "glue_width"
taper = "min(g, 15mm, pH / 4)"
X0 = "g"
X1 = "X0 + pL"
X2 = "X1 + pW"
X3 = "X2 + pL"
X4 = "X3 + pW"

[[rule]]
when = "W > L"
level = "warning"
message = "Width is larger than length; length is normally the longer side."

{MIRROR}
{path([P("0", "taper"), P("0", "pH - taper"), P("X0", "pH"), P("X4", "pH"), P("X4", "0"), P("X0", "0")])}
{crease(P("X0", "0"), P("X0", "pH"))}
{crease(P("X1", "0"), P("X1", "pH"))}
{crease(P("X2", "0"), P("X2", "pH"))}
{crease(P("X3", "0"), P("X3", "pH"))}
{panel("glue", "0", "0", "g", "pH")}
{panel("side1", "X0", "0", "pL", "pH")}
{panel("end1", "X1", "0", "pW", "pH")}
{panel("side2", "X2", "0", "pL", "pH")}
{panel("end2", "X3", "0", "pW", "pH")}
{hinge("side1", "glue", ["X0", "0", "X0", "pH"])}
{hinge("side1", "end1", ["X1", "0", "X1", "pH"])}
{hinge("end1", "side2", ["X2", "0", "X2", "pH"])}
{hinge("side2", "end2", ["X3", "0", "X3", "pH"])}'''
open(f"{root}/sleeves/sleeve.pfc.toml", "w").write(sleeve)

# ── Glued-corner tray ─────────────────────────────────────────────────────────
#   Base (pL × pW) in the middle, side walls above/below, end walls left/right.
#   Corner flaps on the side walls fold onto the inside of the end walls and are glued.
tray_outline = [
    P("0", "Y0"), P("0", "Y1"),
    P("X0", "Y1"), P("X0", "Y1 + c"),
    P("X0 - gw", "Y1 + c"), P("X0 - gw", "Y1 + pH - c - tp"), P("X0 - gw + tp", "Y1 + pH - c"), P("X0", "Y1 + pH - c"),
    P("X0", "Y1 + pH"), P("X1", "Y1 + pH"),
    P("X1", "Y1 + pH - c"), P("X1 + gw - tp", "Y1 + pH - c"), P("X1 + gw", "Y1 + pH - c - tp"), P("X1 + gw", "Y1 + c"),
    P("X1", "Y1 + c"), P("X1", "Y1"),
    P("X1 + pH", "Y1"), P("X1 + pH", "Y0"),
    P("X1", "Y0"), P("X1", "Y0 - c"),
    P("X1 + gw", "Y0 - c"), P("X1 + gw", "Y0 - pH + c + tp"), P("X1 + gw - tp", "Y0 - pH + c"), P("X1", "Y0 - pH + c"),
    P("X1", "Y0 - pH"), P("X0", "Y0 - pH"),
    P("X0", "Y0 - pH + c"), P("X0 - gw + tp", "Y0 - pH + c"), P("X0 - gw", "Y0 - pH + c + tp"), P("X0 - gw", "Y0 - c"),
    P("X0", "Y0 - c"), P("X0", "Y0"),
]
tray = f'''# Glued-corner tray — open four-wall tray. The corner flaps on the side walls fold onto the
# inside of the end walls and are glued. Allowances are NOMINAL; verify with samples.

[component]
id = "tray-glued-corners"
version = "0.1.0"
name = "Tray with glued corners"
category = "04-trays"
tier = "premium"
dimension_reference = "inside"
description = "Open tray: base, four walls, and corner flaps glued to the end walls."

{param("L", "Inside length", 300, 40, 2000)}
{param("W", "Inside width", 200, 40, 2000)}
{param("H", "Wall height", 60, 10, 1000)}
{param("corner_flap", "Corner flap width", 30, 10, 400, "Construction")}

[derived]
t = "board.caliper"
pL = "L + t"                  # base length
pW = "W + t"                  # base width
pH = "H + t"                  # wall height
gw = "corner_flap"
c = "max(t, 1mm)"             # clearance so flaps sit below the wall top and clear the base
tp = "min(gw / 2, (pH - 2 * c) / 3, 8mm)"   # chamfer on the flap's outer corner
X0 = "pH"
X1 = "X0 + pL"
Y0 = "pH"
Y1 = "Y0 + pW"

[[rule]]
when = "2 * gw > pW"
level = "error"
message = "Corner flaps are too wide: two flaps meet on each end wall, so each must be at most half the width."

[[rule]]
when = "pH < 3 * c + 3mm"
level = "error"
message = "Walls are too low for corner flaps."

# ── Cut outline ──
{path(tray_outline)}
# ── Creases: base edges ──
{crease(P("X0", "Y0"), P("X1", "Y0"))}
{crease(P("X0", "Y1"), P("X1", "Y1"))}
{crease(P("X0", "Y0"), P("X0", "Y1"))}
{crease(P("X1", "Y0"), P("X1", "Y1"))}
# ── Creases: corner flaps ──
{crease(P("X0", "Y1 + c"), P("X0", "Y1 + pH - c"))}
{crease(P("X1", "Y1 + c"), P("X1", "Y1 + pH - c"))}
{crease(P("X0", "Y0 - pH + c"), P("X0", "Y0 - c"))}
{crease(P("X1", "Y0 - pH + c"), P("X1", "Y0 - c"))}
# ── Panels and hinges ──
{panel("base", "X0", "Y0", "pL", "pW")}
{panel("wall_back", "X0", "Y1", "pL", "pH")}
{panel("wall_front", "X0", "Y0 - pH", "pL", "pH")}
{panel("wall_left", "0", "Y0", "pH", "pW")}
{panel("wall_right", "X1", "Y0", "pH", "pW")}
{panel("flap_bl", "X0 - gw", "Y1 + c", "gw", "pH - 2 * c")}
{panel("flap_br", "X1", "Y1 + c", "gw", "pH - 2 * c")}
{panel("flap_fl", "X0 - gw", "Y0 - pH + c", "gw", "pH - 2 * c")}
{panel("flap_fr", "X1", "Y0 - pH + c", "gw", "pH - 2 * c")}
{hinge("base", "wall_back", ["X0", "Y1", "X1", "Y1"])}
{hinge("base", "wall_front", ["X0", "Y0", "X1", "Y0"])}
{hinge("base", "wall_left", ["X0", "Y0", "X0", "Y1"])}
{hinge("base", "wall_right", ["X1", "Y0", "X1", "Y1"])}
{hinge("wall_back", "flap_bl", ["X0", "Y1 + c", "X0", "Y1 + pH - c"])}
{hinge("wall_back", "flap_br", ["X1", "Y1 + c", "X1", "Y1 + pH - c"])}
{hinge("wall_front", "flap_fl", ["X0", "Y0 - pH + c", "X0", "Y0 - c"])}
{hinge("wall_front", "flap_fr", ["X1", "Y0 - pH + c", "X1", "Y0 - c"])}'''
open(f"{root}/trays/tray-glued-corners.pfc.toml", "w").write(tray)

# ── Tuck-end cartons ──────────────────────────────────────────────────────────
def lid_up(a, b, y):
    """Lid + tuck tongue rising from crease y on panel span [a, b] (left to right)."""
    return [P(a, y), P(a, f"{y} + pW"), P(f"{a} + ti", f"{y} + pW"),
            R(f"{a} + ti", f"{y} + pW + tk", "rt"), R(f"{b} - ti", f"{y} + pW + tk", "rt"),
            P(f"{b} - ti", f"{y} + pW"), P(b, f"{y} + pW"), P(b, y)]

def lid_down(a, b, y):
    """Lid + tuck tongue hanging from crease y on panel span [b, a] (right to left, a > b)."""
    return [P(a, y), P(a, f"{y} - pW"), P(f"{a} - ti", f"{y} - pW"),
            R(f"{a} - ti", f"{y} - pW - tk", "rt"), R(f"{b} + ti", f"{y} - pW - tk", "rt"),
            P(f"{b} + ti", f"{y} - pW"), P(b, f"{y} - pW"), P(b, y)]

def dust_up(a, b, y):
    return [P(a, y), R(f"{a} + da", f"{y} + d", "rd"), R(f"{b} - da", f"{y} + d", "rd"), P(b, y)]

def dust_down(a, b, y):
    return [P(a, y), R(f"{a} - da", f"{y} - d", "rd"), R(f"{b} + da", f"{y} - d", "rd"), P(b, y)]

def carton(code, name, reverse):
    top = lid_up("X0", "X1", "yt") + dust_up("X1", "X2", "yt")[1:] + [P("X3", "yt")] + dust_up("X3", "X4", "yt")[1:]
    bottom = [P("X4", "yb")] + dust_down("X4", "X3", "yb")[1:]
    if reverse:
        bottom += lid_down("X3", "X2", "yb")[1:]
    else:
        bottom += [P("X2", "yb")]
    bottom += dust_down("X2", "X1", "yb")[1:]
    if reverse:
        bottom += [P("X0", "yb")]
    else:
        bottom += lid_down("X1", "X0", "yb")[1:]
    outline = [P("0", "yb + taper"), P("0", "yt - taper")] + top + bottom
    blid = ("X2", "X3") if reverse else ("X0", "X1")
    bl, br = blid
    lines = [
        crease(P("X0", "yb"), P("X0", "yt")), crease(P("X1", "yb"), P("X1", "yt")),
        crease(P("X2", "yb"), P("X2", "yt")), crease(P("X3", "yb"), P("X3", "yt")),
        "# top closure", crease(P("X0", "yt"), P("X1", "yt")),
        crease(P("X0 + ti", "yt + pW"), P("X1 - ti", "yt + pW")),
        crease(P("X1", "yt"), P("X2", "yt")), crease(P("X3", "yt"), P("X4", "yt")),
        "# bottom closure", crease(P(bl, "yb"), P(br, "yb")),
        crease(P(f"{bl} + ti", "yb - pW"), P(f"{br} - ti", "yb - pW")),
        crease(P("X1", "yb"), P("X2", "yb")), crease(P("X3", "yb"), P("X4", "yb")),
    ]
    panels = [
        panel("glue", "0", "yb", "g", "pH"), panel("back", "X0", "yb", "pL", "pH"),
        panel("side1", "X1", "yb", "pW", "pH"), panel("front", "X2", "yb", "pL", "pH"),
        panel("side2", "X3", "yb", "pW", "pH"),
        panel("lid_top", "X0", "yt", "pL", "pW"), panel("tuck_top", "X0 + ti", "yt + pW", "pL - 2 * ti", "tk"),
        panel("dust_top1", "X1", "yt", "pW", "d"), panel("dust_top2", "X3", "yt", "pW", "d"),
        panel("lid_bottom", bl, "yb - pW", "pL", "pW"), panel("tuck_bottom", f"{bl} + ti", "yb - pW - tk", "pL - 2 * ti", "tk"),
        panel("dust_bottom1", "X1", "yb - d", "pW", "d"), panel("dust_bottom2", "X3", "yb - d", "pW", "d"),
    ]
    blid_body = "front" if reverse else "back"
    hinges = [
        hinge("back", "glue", ["X0", "yb", "X0", "yt"]), hinge("back", "side1", ["X1", "yb", "X1", "yt"]),
        hinge("side1", "front", ["X2", "yb", "X2", "yt"]), hinge("front", "side2", ["X3", "yb", "X3", "yt"]),
        hinge("back", "lid_top", ["X0", "yt", "X1", "yt"]),
        hinge("lid_top", "tuck_top", ["X0 + ti", "yt + pW", "X1 - ti", "yt + pW"]),
        hinge("side1", "dust_top1", ["X1", "yt", "X2", "yt"]), hinge("side2", "dust_top2", ["X3", "yt", "X4", "yt"]),
        hinge(blid_body, "lid_bottom", [bl, "yb", br, "yb"]),
        hinge("lid_bottom", "tuck_bottom", [f"{bl} + ti", "yb - pW", f"{br} - ti", "yb - pW"]),
        hinge("side1", "dust_bottom1", ["X1", "yb", "X2", "yb"]), hinge("side2", "dust_bottom2", ["X3", "yb", "X4", "yb"]),
    ]
    where = "the back panel at the bottom too" if not reverse else "the front panel at the bottom"
    doc = f'''# {name} — folding carton. The top lid and tuck hinge from the back panel; the bottom
# lid hinges from {where}. Dust flaps on the side panels sit under the lids.
# Allowances are NOMINAL for folding boxboard; verify with samples.

[component]
id = "{code}"
version = "0.1.0"
name = "{name}"
category = "ecma-tuck-end"
tier = "premium"
default_board = "FBB-0.5"
dimension_reference = "inside"
description = "Folding carton with tuck-in closures top and bottom ({"opposite" if reverse else "same"} side), side-seam glue flap and dust flaps."

{param("L", "Inside length", 80, 15, 600)}
{param("W", "Inside width", 50, 10, 600)}
{param("H", "Inside height", 150, 15, 800)}
{param("tuck", "Tuck flap depth", 15, 6, 60, "Construction")}
{param("glue_width", "Glue flap width", 15, 8, 30, "Construction")}
{GLUE_SIDE}
[derived]
t = "board.caliper"
pL = "L + t"
pW = "W + t"
pH = "H + 2 * t"
g = "glue_width"
taper = "min(g * 0.6, pH / 4)"
tk = "tuck"
ti = "max(t, 1mm)"                      # tongue inset so the tuck slides in
rt = "min(tk * 0.45, (pL - 2 * ti) / 4)" # rounded tongue corners (below half the tongue depth)
d = "min(pL * 0.45, pW)"                # dust flap depth (two must fit in the length)
da = "min(d * 0.3, pW * 0.2)"           # dust flap edge angle offset
rd = "min(3mm, da)"
X0 = "g"
X1 = "X0 + pL"
X2 = "X1 + pW"
X3 = "X2 + pL"
X4 = "X3 + pW"
yb = "pW + tk"                          # bottom score line
yt = "yb + pH"                          # top score line

[[rule]]
when = "board.caliper > 1.5"
level = "warning"
message = "Tuck-end cartons are normally made from folding boxboard up to about 1 mm."

[[rule]]
when = "tk > pH / 2"
level = "warning"
message = "The tuck flap is deeper than half the carton height."

{MIRROR}
# ── Cut outline ──
{path(outline)}
# ── Creases ──
''' + "\n".join(lines) + "\n# ── Panels and hinges ──\n" + "\n".join(panels) + "\n" + "\n".join(hinges)
    open(f"{root}/cartons/{code}.pfc.toml", "w").write(doc)

carton("tuck-end-straight", "Straight tuck end carton", reverse=False)
carton("tuck-end-reverse", "Reverse tuck end carton", reverse=True)
print("ok")
