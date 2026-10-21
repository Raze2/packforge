# 05 — API & Integrations

Three ways to use PackForge from other projects:

1. **Web component** (browser, WASM) — embed the designer in any page.
2. **HTTP API** (`pf-api`, Docker) — server-side render/export from any language.
3. **Laravel package** (`packforge/laravel`) — idiomatic PHP client for the HTTP API.

## 1. Web component — `<packforge-designer>`

One self-contained ES module (`apps/web/dist/wc/packforge-designer.js`, ~430 KB gzipped,
WASM and CSS inlined). It renders into the light DOM, so the host page controls its size.

```html
<script type="module" src="/vendor/packforge/packforge-designer.js"></script>

<packforge-designer
  mode="configurator"
  component="fefco-0201"
  params='{"L":300,"W":200,"H":150}'
  board="C-flute"
  units="mm"
  locale="en"
  theme="auto"
  export-endpoint="/packforge/export"
  style="height: 640px"
></packforge-designer>

<script type="module">
  const d = document.querySelector('packforge-designer');
  d.addEventListener('pf:change', (e) => console.log(e.detail.project, e.detail.result?.stats));
  d.addEventListener('pf:save', (e) => {
    e.preventDefault(); // skip the browser download, store it yourself:
    fetch('/designs', { method: 'POST', headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify(e.detail.project) });
  });
</script>
```

| Attribute | Meaning |
|---|---|
| `component` | Initial style id (default `fefco-0201`) |
| `params` | Initial parameters as JSON, lengths in mm |
| `board` | Board preset id (`E-flute`, `B-flute`, `C-flute`, `EB-double`, `BC-double`, `FBB-0.5`) |
| `units`, `locale`, `theme` | `mm\|cm\|in`, `en\|fr\|ar`, `auto\|light\|dark` |
| `mode` | `full` (library + file actions) or `configurator` (parameters + preview + export) |
| `persist` | Remember the last design and preferences in this browser |
| `export-endpoint` | If set, exports are `POST {project, format}` to this URL on your backend and the response is downloaded (for premium exports via `pf-api`; never put API keys in the browser) |

| Event | `detail` | Cancelable |
|---|---|---|
| `pf:ready` | — | no |
| `pf:change` | `{ project, result, error }` | no |
| `pf:save` | `{ project }` | yes — `preventDefault()` skips the JSON download |
| `pf:export` | `{ project, format, filename, mime, bytes }` | yes — skips the download |
| `pf:error` | `{ code, message }` | no |

Methods: `el.getProject()`, `el.setProject(project)`,
`el.exportFile(format, options?)` with `format` one of `pdf`, `dxf`, `cf2`, `hpgl`, `svg`, `glb`.

The designer has a 2D/3D toggle (fold slider, orbit, zoom) and an **Export…** dialog with the
format, rotation, mirroring, PDF page, machine profile and sheet nesting (live blanks-per-sheet
preview). With `export-endpoint`, the options are posted to your backend along with the project.

Without UI, the same core is available to scripts through the generated bindings in
`apps/web/src/lib/wasm` (`render`, `exportFile`, `listComponents`, `describe`, `boards`);
publishing them as `@packforge/core` on npm is planned for Stage 10.

## 2. HTTP API — `pf-api` (v1)

Base URL: `https://packforge.yourhost/v1` · Auth: `Authorization: Bearer <API_KEY>` ·
JSON in/out unless a file is returned. `/v1/health` and `/v1/openapi.json` are public.

| Method | Path | Body | Returns |
|---|---|---|---|
| GET | `/health` | — | `{status, version}` |
| GET | `/openapi.json` | — | OpenAPI 3.1 document |
| GET | `/components` | — | catalogue (id, name, standard, category, tier, version) |
| GET | `/components/{id}` | — | parameter schema |
| GET | `/boards` | — | board presets |
| POST | `/render` | `{project}` | `RenderResult`: dieline, fold, stats, diagnostics |
| POST | `/export` | `{project, format, options?}` | file bytes with `Content-Disposition` and a strong `ETag` (`If-None-Match` → 304) |
| POST | `/nest` | `{project, nesting}` | blanks per sheet, grid, rotation, sheet utilisation |
| POST | `/fold` | `{project, progress?}` | 3D panel corners (mm) and normals; `progress` 0 = flat, 1 = erected |
| POST | `/pallet` | `{project, pallet?}` | pallet plan: pattern, per layer, layers, total, efficiencies, box positions of one layer |
| POST | `/pallet/report` | `{project, pallet?}` | A4 PDF: layer top view, stack side view, figures |
| POST | `/case-optimize` | `{product, count, component_id?, board?, clearance?, allow_rotation?, max_results?, pallet?, board_price_per_m2?}` | case sizes ranked by board per product (or by products per pallet when `pallet` is set) |

**Formats:** `svg`, `dxf` (layers per line type), `pdf` (1:1 or fitted, title block, legend),
`cf2` (die-makers, cutting tables), `hpgl` (`.plt`, plotters), `glb` (3D model of the erected box).

**Pallet request** (`pallet`, all optional): `pallet` (`eur` default, `eur-half`,
`iso-1200x1000`, `gma`, `asia`, or `{length, width, height}`), `max_height` (incl. pallet,
default 1800), `box_weight`, `max_weight` (kg), `overhang`, `allow_on_side`, `box_outer`
(`[L, W, H]` measured; default inside + 2 × caliper). Layer patterns: block, two-block, pinwheel;
alternate layers are mirrored when the pattern is not a block.

**Export options** (all optional):

| Option | Values | Notes |
|---|---|---|
| `rotate` | `0`, `90`, `180`, `270` | Counter-clockwise |
| `mirror` | `true` / `false` | Cut from the unprinted side |
| `nesting` | `{sheet_width, sheet_height, gap = 8, margin = 10, allow_rotation = true, max_count}` | mm; grid nesting in one orientation |
| `page` | `actual` (default), `a4`, `a3`, `letter` | PDF only |
| `watermark` | text | PDF only |
| `profile` | `standard`, `cut-crease` | Which line types reach the machine, DXF layer names, CF2 type codes, HPGL pens |
| `custom_profile` | full profile object | Overrides `profile`; see `crates/pf-export/src/profile.rs` |

> **CF2 / HPGL conventions vary by shop.** Line-type codes, rule thickness and pen numbers come
> from the output profile: check the defaults against your CAM import and adjust the profile.

`project` example:

```json
{
  "component_id": "fefco-0201",
  "params": {"L": 300, "W": 200, "H": 150, "glue_side": "left"},
  "board": "C-flute",
  "units": "mm"
}
```

`board` is a preset id or `{"id": "custom", "caliper_mm": 3.2}`.

Errors are RFC 7807 `application/problem+json` with `type`, `title`, `status`, `detail`,
`code` and, for invalid parameters, `errors: [{param, message}]`.

| Status | `code` |
|---|---|
| 400 | `invalid_request`, `invalid_params`, `unknown_board`, `unknown_format` |
| 401 | `unauthorized` (with `WWW-Authenticate: Bearer`) |
| 404 | `not_found`, `unknown_component` |
| 413 / 415 | `invalid_request` (body too large / not JSON) |
| 422 | `rule_violation`, `geometry_error`, `export_error` (bad options, blank does not fit), `fold_error` |
| 429 | `rate_limited` (with `Retry-After`) |

Successful authenticated responses carry `X-RateLimit-Limit` and `X-RateLimit-Remaining`.

### Running it

| Variable | Default | Meaning |
|---|---|---|
| `PF_BIND` | `0.0.0.0:8080` | Listen address |
| `PF_KEYS_FILE` | — | TOML file of hashed keys (required) |
| `PF_DEV_NO_AUTH` | — | `1` disables auth (local development only) |
| `PF_CORS_ORIGINS` | — | Comma-separated origins allowed to call from browsers |
| `PF_MAX_BODY` | `262144` | Max request body (bytes) |
| `RUST_LOG` | `info` | Log filter (`tower_http=debug` logs every request) |

API keys are stored only as SHA-256 hashes:

```bash
pf-api gen-key --id shop-prod --plan enterprise --rate-per-minute 600   # prints secret + [[key]] entry
pf-api hash-key pk_live_…                                                # hash for an existing secret
```

```toml
# keys.toml
[[key]]
id = "shop-prod"
hash = "sha256:…"
plan = "enterprise"
rate_per_minute = 600
```

Docker (stateless, scale horizontally; rate limits are per instance in v1):

```bash
docker build -t packforge/api .
docker run -p 8080:8080 -v $PWD/keys.toml:/etc/packforge/keys.toml:ro packforge/api
```

The image is distroless and runs as non-root; use `GET /v1/health` for liveness/readiness probes.

## 3. Laravel package — `packforge/laravel`

See [`integrations/laravel/README.md`](../integrations/laravel/README.md) for the full guide and
[`examples/laravel-shop`](../examples/laravel-shop) for a working shop.

```php
use PackForge\Laravel\Facades\PackForge;

$dxf = PackForge::component('fefco-0201')
    ->params(['L' => 300, 'W' => 200, 'H' => 150])
    ->board('C-flute')
    ->dxf();                                       // PackForge\Laravel\ExportedFile

$dxf->store("orders/{$order->id}/{$dxf->filename}");
$stats = PackForge::design($order->project)->render()->stats;   // blankWidth, blankAreaM2, …
```

```blade
<x-packforge-designer :project="$project" mode="configurator" theme="light" height="640px" />
```

Production formats and options:

```php
$design->pdf(['page' => 'a4']);                                    // fitted proof
$design->cf2(['rotate' => 90, 'profile' => 'cut-crease']);         // cutting table
$design->plt(['nesting' => ['sheet_width' => 2400, 'sheet_height' => 1600]]);
$design->glb();                                                    // 3D model
$design->nest(['sheet_width' => 2400, 'sheet_height' => 1600]);    // ['count' => 8, ...]
$design->pallet(['pallet' => 'eur', 'box_weight' => 8]);          // ['plan' => ['total' => 70, ...]]
$design->palletReport();                                           // A4 PDF
PackForge::caseOptimize([100, 80, 60], 12, ['pallet' => ['pallet' => 'eur']]);
```

Package contents: `Client` + facade, immutable `Design` builder, `RenderResult`/`Stats`/
`ExportedFile` DTOs, exceptions mapped from problem+json (validation, auth, 404, rate limit,
connection), retries for connection errors/5xx/429, the Blade component, and the
`POST /packforge/export` proxy route (CSRF-protected, throttled, key stays server-side).

## 4. CLI — `pf`

```bash
pf list
pf boards
pf describe fefco-0201
pf render fefco-0201 -p L=300 -p W=200 -p H=150 -b C-flute          # stats
pf render fefco-0201 -p L=300 -p W=200 -p H=150 --caliper 3.2 --json
pf export fefco-0201 -p L=300 -p W=200 -p H=150 -f dxf -o box.dxf   # -o - for stdout
pf export fefco-0201 -f pdf --page a4 --sheet 2400x1600 --watermark "Proof"
pf export fefco-0201 -f cf2 --rotate 90 --profile cut-crease
pf nest fefco-0201 --sheet 2400x1600 --gap 8 --margin 10
pf fold fefco-0201 --progress 0.5                                   # 3D panels as JSON
pf pallet fefco-0201 -p L=342 -p W=242 -p H=200 --box-weight 8 --report pallet.pdf
pf case 100x80x60 12 --pallet eur --price 0.65                      # case sizes for 12 products
pf batch orders.csv --format pdf --out ./exports                     # planned (Stage 3)
```

## 5. Other stacks

Any language can call the HTTP API. Node/Deno/Bun can also use `@packforge/core` (WASM)
directly without a server.
