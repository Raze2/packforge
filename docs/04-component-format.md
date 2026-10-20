# 04 — Component Format (`*.pfc.toml`) — Draft v0

A component (box style) is a **data file**, not code. The core parses it, evaluates formulas
with `pf-expr` (sandboxed: no I/O, no loops, bounded steps), and produces a `DieLine` and a
`FoldModel`. Geometry here comes from public standards (FEFCO/ECMA) and first principles.

## 1. File structure

| Section | Purpose |
|---|---|
| `[component]` | Identity, version, standard reference, category, tier |
| `[[param]]` | User inputs (shown as the parameter form) |
| `[derived]` | Named formulas computed from params + board (ordered, acyclic) |
| `[[rule]]` | Validation rules with user-facing messages |
| `[[panel]]` | Rigid panels (for 3D folding and area calc) |
| `[[hinge]]` | Fold lines between panels with closed angle |
| `[layout]` | Whole-design transforms, e.g. `mirror_x` |
| `[[line]]` / `[[path]]` | Die-line entities with a line type (optional `when`) |
| `[[repeat]]` | *Planned:* repeat a block with an index variable (bounded) |

## 2. Expressions (`pf-expr`)

- Numbers are millimetres (`f64`). Literals may carry units: `10mm`, `2cm`, `0.1m`, `1in`.
- Operators: `+ - * / % ^ ( )`, comparisons, `and or not` (or `&& || !`), ternary `cond ? a : b`.
- Strings in single or double quotes, compared with `==` / `!=` (e.g. `glue_side == 'left'`).
- Functions: `min max clamp abs round floor ceil sqrt sin cos tan atan2 rad deg`.
  Trigonometry works in **degrees**; `rad()` / `deg()` convert.
- Names: params (`L`, `W`, `H`…), `board.caliper`, `board.id`, `board.flute`, and `[derived]`
  names. Derived values may appear in any order; they are sorted by dependency.
- Unknown names and circular dependencies are rejected when the file is loaded, with the
  location (e.g. `derived 'pL'`, `path #1.points[3].x`). Runtime errors (division by zero,
  non-finite results) are reported, never panic.
- Source length (4096 chars) and nesting depth (64) are capped, so evaluation always ends quickly.

## 3. Parameter types

| type | value | extra keys |
|---|---|---|
| `length` | f64 mm | `min`, `max`, `step` |
| `number` | f64 | `min`, `max` |
| `bool` | true/false | — |
| `enum` | string | `values = [...]` |
| `angle` | degrees | `min`, `max` |

Every param: `id`, `label` (i18n key or text), `type`, `default`, `group`, optional `help`.

## 4. Example — FEFCO 0201 (Regular Slotted Container), excerpt

The complete, working definition is [`library/fefco/0201.pfc.toml`](../library/fefco/0201.pfc.toml).

> Allowance values are a **starting point** and must be verified against physical
> samples per board grade before release.

```toml
[component]
id = "fefco-0201"
version = "0.1.0"
name = "Regular slotted container"
standard = "FEFCO 0201"
category = "02-slotted"
tier = "free"
dimension_reference = "inside"

[[param]]
id = "L"
label = "Inside length"
type = "length"
default = 300
min = 40
max = 2500
group = "Dimensions"

[[param]]
id = "glue_side"
label = "Glue flap side"
type = "enum"
values = ["left", "right"]
default = "left"
group = "Construction"

[derived]
t = "board.caliper"
pL = "L + t"                    # length panel width (crease to crease)
pW = "W + t"
pH = "H + 2 * t"                # body height (score to score)
f = "pW / 2"                    # flaps meet at the centre
s = "max(1.5 * t, 5mm)"         # slot width
hs = "s / 2"
X0 = "glue_width"
X1 = "X0 + pL"
yb = "f"
yt = "f + pH"

[[rule]]
when = "W > L"
level = "warning"
message = "Width is larger than length; length is normally the longer side."

[layout]
mirror_x = "glue_side == 'right'"   # whole die-line mirrored for a right-hand glue flap

[[path]]
type = "cut"
closed = true
points = [["0", "yb + taper"], ["0", "yt - taper"], ["X0", "yt"], ["X0 + hs", "yt"], ...]

[[line]]
type = "crease"
from = ["X1", "yb"]
to = ["X1", "yt"]

[[panel]]
id = "side1"
x = "X0"
y = "yb"
w = "pL"
h = "pH"

[[hinge]]
a = "side1"
b = "end1"
axis = ["X1", "yb", "X1", "yt"]   # x1, y1, x2, y2
closed_angle = 90
```

Optional `when = "<bool expr>"` on any `[[line]]` or `[[path]]` includes it conditionally.

### Path points: corners, rounded corners and arcs

Each entry of a path's `points` list is one of:

| Entry | Meaning |
|---|---|
| `["x", "y"]` | Sharp vertex |
| `{ at = ["x", "y"], round = "r" }` | Vertex replaced by a tangent arc of radius `r` (tuck tongues, dust flaps). Needs straight edges on both sides; the radius is reduced (with an info note) if it does not fit in half of each edge |
| `{ to = ["x", "y"], radius = "r", ccw = true, large = false }` | Circular arc from the previous point to `to`. `ccw` and `large` pick the centre like SVG's sweep/large-arc flags; a radius smaller than half the chord is enlarged |

Example (rounded tuck tongue): `R(X0 + ti, top)` → `{ at = ["X0 + ti", "yt + pW + tk"], round = "rt" }`.

### Optional component metadata

`default_board = "FBB-0.5"` in `[component]` suggests a board preset when the user picks the
component (used by folding cartons).

## 5. Validation the core always runs

- All param values within declared ranges; `[[rule]]` checks.
- At least one closed cut outline with non-zero area; all coordinates finite.
- The cut outline does not cross itself (touching at a shared vertex is allowed).
- Every tooling line lies within the cut outline's bounds.
- *Planned:* overlap checks between different lines.
- *Planned (Stage 6):* every hinge axis lies on a crease; panel graph is connected.
- Determinism check in CI: render twice ⇒ identical output.

## 6. Versioning

`version` is semver. A geometry change that alters output is a **minor** bump at least;
saved projects pin the component version, so old designs re-open identically.

## 7. Open questions (decide in ADRs)

- Do we need a scripting escape hatch (e.g. `rhai`) for complex styles, or can declarative +
  `[[repeat]]` cover the catalogue? Start declarative; revisit after 20 styles.
- Curved geometry (tuck-in locks, crash-lock bottoms): arc/bezier syntax details.
