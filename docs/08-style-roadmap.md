# 08 — Style Roadmap

Which box styles to add after the core stages (6 3D preview, 7 production exports, 8 optimisation),
in what order, and what each wave needs from the engine.

Sources: the public **FEFCO** code (corrugated) and **ECMA** code (folding cartons), plus common
industry practice. Every style is designed from first principles (see
[00-clean-room-policy.md](00-clean-room-policy.md)) and verified with physical samples before it
is marked stable.

> **Codes to confirm:** FEFCO/ECMA numbers marked † are from memory and must be checked against
> the official catalogues before release. The style *descriptions* define what we build.

## Done (v0.1)

| Style | Code | Tier |
|---|---|---|
| Half slotted container | FEFCO 0200 | Free |
| Regular slotted container (RSC) | FEFCO 0201 | Free |
| Overlapping slotted container | FEFCO 0202 | Premium |
| Full overlap slotted container | FEFCO 0203 | Premium |
| Centre special slotted container | FEFCO 0204 | Premium |
| Sleeve (four-panel tube) | — | Free |
| Tray with glued corners | — | Premium |
| Straight tuck end carton | ECMA (tuck end) | Premium |
| Reverse tuck end carton | ECMA (tuck end) | Premium |

## Engine features needed

Some styles need capabilities the component format does not have yet. Each wave lists them.

| Feature | Needed for | Size |
|---|---|---|
| **Multi-part components** (e.g. lid + base in one design, separate blanks) | Telescope boxes, lid-and-base, inserts | M |
| **Slits and lock tabs** (cut lines inside panels, tab/slot pairs) | Mailers, lock bottoms, displays, trays | M |
| **Perforation and half-cut lines** in styles (types exist already) | Shelf-ready (tear-off fronts), tear strips | S |
| **Angled panels** (non-rectangular panels, slanted walls) | Displays, sloped trays, shelf-ready | S |
| **Repeated features** (`[[repeat]]`: n cells, n slots) | Partitions, multi-cell trays, dividers | M |
| **Glue areas** (new line type / region) | Crash-lock bottoms, ready-glued styles | S |
| **Fold model checks** (3D, Stage 6): panels must not collide when erected | All complex styles | M |
| **Per-board allowance tables** (measured, see below) | Every style before "stable" | M (ongoing) |

## Wave 1 — Core shipping and storage boxes (next)

Most orders in corrugated shops; mostly reuse the slotted layout. **Engine: multi-part.**

| # | Style | Code | Why | Tier |
|---|---|---|---|---|
| 1 | Pad / sheet (layer pad, slip sheet) | FEFCO 0110 † | Simplest product, used everywhere | Free |
| 2 | Slotted container with one-side half flaps | FEFCO 02xx † | Common variant | Premium |
| 3 | Telescope box: separate lid and base (trays with flaps) | FEFCO 0300 † | Very common for heavy goods and archive boxes | Free |
| 4 | Full telescope design box | FEFCO 0301 † | Lid covers the full height | Premium |
| 5 | Half-slotted box with cover (0200 + lid) | FEFCO 0310 † | Two-piece box | Premium |
| 6 | Bliss-style box (body + two end pieces) | FEFCO 06xx † | Heavy-duty | Premium |

## Wave 2 — E-commerce and die-cut folders

Fast-growing demand (mailers, subscription boxes). **Engine: slits and lock tabs.**

| # | Style | Code | Why | Tier |
|---|---|---|---|---|
| 7 | Roll end tuck top mailer with dust flaps (classic e-commerce box) | FEFCO 0427 | The most requested die-cut box | Free |
| 8 | Roll end tuck front (front-opening mailer) | FEFCO 04xx † | Mailer variant | Premium |
| 9 | Five-panel folder / wrap-around for flat items | FEFCO 0410 † | Books, flat goods | Premium |
| 10 | One-piece folder with lock tabs | FEFCO 0421 † | Self-locking, no glue | Premium |
| 11 | Pizza / food box (tuck lid, locked corners) | FEFCO 04xx † | Food delivery | Premium |
| 12 | Wrap-around blank (for case erectors) | FEFCO 0470 † | Automated packing lines | Premium |

## Wave 3 — Folding cartons (retail)

For printers and cosmetics/pharma customers. **Engine: glue areas, slits.**

| # | Style | Code | Why | Tier |
|---|---|---|---|---|
| 13 | Crash-lock (auto) bottom carton, tuck top | ECMA (auto-bottom) † | Fast to erect, strong bottom | Premium |
| 14 | Lock bottom (1-2-3 bottom), tuck top | ECMA † | No glue on the bottom | Premium |
| 15 | Seal end carton (glued both ends) | ECMA † | Cereal, food | Premium |
| 16 | Tuck top with auto-lock bottom and hang tab | ECMA † | Hanging retail | Premium |
| 17 | Carton sleeve with tray (drawer box) | ECMA † | Gifts, cosmetics | Premium |
| 18 | Pillow box | ECMA † | Gift packaging (curved creases: needs arcs as creases) | Premium |

## Wave 4 — Trays and shelf-ready packaging (SRP)

Retailers ask for shelf-ready cases. **Engine: perforations, angled panels, lock tabs.**

| # | Style | Code | Why | Tier |
|---|---|---|---|---|
| 19 | Four-corner glued tray with sloped front (display tray) | FEFCO 0700s / 04xx † | Fruit, vegetables | Premium |
| 20 | Die-cut tray with lock corners (no glue) | FEFCO 0422 † | Erected by hand | Premium |
| 21 | Ready-glued tray (pre-glued corners, flat-packed) | FEFCO 07xx † | Fast erection | Premium |
| 22 | Shelf-ready case: RSC with perforated tear-off front | — | Retail requirement | Premium |
| 23 | Shelf-ready tray + hood (two-part) | — | Retail requirement | Premium |
| 24 | Fruit/vegetable tray with stacking corners | — | Agriculture | Premium |

## Wave 5 — Inserts and dividers

Sold with almost every shipping box. **Engine: repeat, slits.**

| # | Style | Code | Why | Tier |
|---|---|---|---|---|
| 25 | Interlocking partitions (n × m cells, slotted strips) | FEFCO 09xx † | Bottles, glass, fragile goods | Premium |
| 26 | Liner / tube insert (folded sleeve inside a box) | FEFCO 09xx † | Reinforcement | Free |
| 27 | Corner protectors / edge boards | FEFCO 09xx † | Furniture, appliances | Free |
| 28 | Die-cut insert with product cut-outs (generic shapes) | — | Electronics, cosmetics | Premium |

## Wave 6 — Displays (POS)

Highest value per design, most complex geometry. **Engine: angled panels, multi-part, 3D checks.**

| # | Style | Why | Tier |
|---|---|---|---|
| 29 | Counter display box (tray with raised header card) | Retail checkout | Premium |
| 30 | Dump bin (open-top floor display) | Promotions | Premium |
| 31 | Floor display with shelves (multi-part) | Store displays | Premium |
| 32 | Pallet display skirt / quarter-pallet display | Supermarkets | Premium |

## Later / on request

| Style family | Notes |
|---|---|
| Furniture and X-board (honeycomb / heavy board) | Different materials and joints; separate project |
| Rigid boxes (setup boxes, chipboard + wrap) | Different production process |
| Customer-specific styles | Built by users with the component format (Premium "custom components") |

## How each style is added

1. Write the component file (`library/<family>/<code>.pfc.toml`) from the standard's drawing and
   first principles; use a generator in `scripts/library` when several styles share a layout.
2. Library-wide tests run automatically (defaults, parameter sweep, hinges on creases, connected
   panels, no self-crossing). Add a test for the style's blank-size formula.
3. Add a golden case, translations (EN/FR/AR) and a thumbnail check.
4. Export a 1:1 sample on each target board, fold it, measure inside dimensions, record the
   corrections in the board's allowance table.
5. Mark the style **stable** (version 1.0.0) only after step 4.

## Allowance tables (start collecting now)

Every style's accuracy depends on how much each board grade needs per crease. We will build our own
tables from **your** measured samples, which will fit your production better than generic values:

| Measure | For each board grade (E, B, C, EB, BC, folding boxboard…) |
|---|---|
| Inside → crease-to-crease allowance per panel | Length, width, height |
| Flap gap / slot width | Per caliper |
| Glue flap and tuck clearances | Per caliper |
| Measured caliper (actual, not nominal) | From your supplier's boards |

Suggested process: export FEFCO 0201 at 300×200×150 for each board, make the sample, measure the
inside dimensions, and send the numbers: they become the first allowance table.
