# 01 — Product Specification (Functional)

Behaviour-level description of what PackForge does. No implementation detail.
See [00-clean-room-policy.md](00-clean-room-policy.md) for what may inform this document.

## 1. Vision

A fast, modern, cross-platform tool for designing packaging (corrugated boxes, folding
cartons, displays) that runs **in the browser, on the desktop (Windows / macOS / Linux),
and as a server service** callable from Laravel/PHP or any other stack — all from one core.

## 2. Users

| Persona | Needs |
|---|---|
| Packaging designer | Pick a style, set dimensions, see the die-line and 3D fold, export to the cutting table |
| Sales / estimator | Generate a box quickly for a quote, get blank size, board area, pallet load |
| Web shop / ERP integrator (Laravel) | Embed a configurator, generate DXF/PDF server-side per order |
| Production / CAD operator | Export to cutting-table formats with correct line types and tool layers |

## 3. Core concepts

- **Component (style)**: a parametric packaging template (e.g. FEFCO 0201). Has parameters,
  derived values, and rules that produce a die-line and a fold model.
- **Parameters**: user inputs — inner length/width/height, board grade, caliper, flap options,
  glue-flap side, etc. Each has type, unit, default, min/max, and a group.
- **Board / material**: name, flute or caliper (mm), allowance rules, cost per m² (optional).
- **Die-line**: the flat 2D drawing. Every entity has a **line type**: cut, crease, reverse
  crease, perforation, half-cut, bleed, dimension, annotation.
- **Fold model**: panels and hinge lines with fold angles, for the 3D preview.
- **Project**: a saved design = component id + version + parameter values + metadata.
- **Library**: browsable catalogue of components with categories, tags, search, thumbnails.

## 4. Features

Tier key: **F** = Free, **P** = Premium, **E** = Enterprise/API.

### 4.1 Library & design
| # | Feature | Tier |
|---|---|---|
| L1 | Browse library by category (FEFCO groups 02xx, 03xx, 04xx…, ECMA, displays), search, tags | F |
| L2 | Open a component, edit parameters in a grouped panel with live validation | F |
| L3 | Live 2D die-line preview with pan/zoom, line-type colours, dimensions overlay | F |
| L4 | Units: mm / cm / inch, switchable at any time | F |
| L5 | Board/material presets; custom boards | F |
| L6 | Save / open projects (local file, browser storage, or server) | F |
| L7 | Multiple languages (EN, FR, ES, IT, DE, AR at minimum; RTL-ready UI) | F |
| L8 | Starter library (~10 common styles) | F |
| L9 | Full library (all supported styles) | P |
| L10 | Create/edit custom components in the component format (code editor + live preview) | P |
| L11 | Import/share custom components (`.pfc` package) | P |

### 4.2 Visualisation
| # | Feature | Tier |
|---|---|---|
| V1 | 3D fold preview with fold-progress slider | F |
| V2 | Apply artwork/texture to panels in 3D | P |
| V3 | Export 3D: glTF/GLB (web viewer), OBJ, STL | P |
| V4 | Screenshot / PNG export of 2D and 3D | F |

### 4.3 Export & production
| # | Feature | Tier |
|---|---|---|
| X1 | Export SVG and PDF (watermarked in Free) | F |
| X2 | Export PDF without watermark, with title block | P |
| X3 | Export DXF (layers per line type, configurable layer names/colours) | P |
| X4 | Cutting-table formats: CF2, HPGL/PLT, and vendor profiles | P |
| X5 | Post-processor profiles: tool mapping per line type, origin, rotation, scaling | P |
| X6 | Nesting: multiple blanks on a sheet with gap/rotation rules | P |

### 4.4 Calculation & optimisation
| # | Feature | Tier |
|---|---|---|
| C1 | Blank size, board area, weight, perimeter of cut/crease lengths | F |
| C2 | Cost estimate from board price | P |
| C3 | Case optimisation: best box dims for N products of size a×b×c | P |
| C4 | Pallet loading: boxes per layer, layers, patterns, 3D view of the load | P |
| C5 | Reports (PDF/HTML) for a design or pallet plan, with branding | P |

### 4.5 Integration
| # | Feature | Tier |
|---|---|---|
| I1 | Embeddable web component `<packforge-designer>` | F (watermark) / P |
| I2 | REST API: list components, render die-line, export files, pallet calc | E |
| I3 | Laravel package (Facade, config, queue-friendly) | E |
| I4 | CLI for batch export | P |

## 5. Main user flows

1. **Design → export**: Library → pick style → set dims & board → preview 2D/3D → export.
2. **Quote**: pick style → dims → see blank size, area, cost, pallet load → PDF report.
3. **Integrator**: Laravel order → API call with style + params → DXF/PDF stored with the order.
4. **Custom style**: new component → write parameters & geometry → live preview → test → publish to library.

Diagrams: [03-flows.md](03-flows.md).

## 6. Non-functional requirements

| Area | Requirement |
|---|---|
| Accuracy | Geometry in `f64` millimetres; exported coordinates within 0.01 mm of spec; dimensions verified against physical samples per style |
| Performance | Re-generate die-line < 16 ms for standard styles (live slider dragging); WASM bundle < 3 MB gzipped |
| Platforms | Chrome/Edge/Firefox/Safari (last 2 versions); Windows 10+, macOS 12+, Ubuntu 22.04+ |
| Offline | Desktop and web designer work fully offline (except premium validation refresh) |
| Determinism | Same component version + params ⇒ byte-identical export (enables caching & tests) |
| Security | Server API: API keys, rate limits, input validation; components sandboxed (no I/O) |
| Accessibility | Keyboard navigable, WCAG AA contrast, RTL support |

## 7. Out of scope (v1)

Corrugator scheduling, ERP inventory, multi-user real-time co-editing, importing competitor
proprietary formats (see policy §Process 5).
