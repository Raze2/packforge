# PackForge (working name)

Cross-platform packaging design — parametric boxes, die-lines, 3D folding, production
exports and pallet optimisation — from **one Rust core** running in the **browser (WASM)**,
on the **desktop (Windows / macOS / Linux, via Tauri)**, and as an **HTTP API** for
**Laravel/PHP** and other projects.

> Status: Stages 0–5 — core engine, 9 box styles (FEFCO 0200–0204, sleeve, tray, tuck-end
> cartons), SVG/DXF export, CLI, web designer, `<packforge-designer>` web component, HTTP API,
> Laravel package and the desktop app (Tauri) working.

## Quick start

```bash
cargo test --workspace
cargo run -p pf-cli -- render fefco-0201 -p L=300 -p W=200 -p H=150 -b C-flute
cargo run -p pf-cli -- export fefco-0201 -p L=300 -p W=200 -p H=150 -f dxf -o box.dxf
cargo build -p pf-wasm --target wasm32-unknown-unknown --release

# Web designer (needs wasm-bindgen-cli 0.2.129 and pnpm)
pnpm install
pnpm dev        # http://localhost:5173 and /embed-demo.html
pnpm build:web  # dist/app and dist/wc/packforge-designer.js

# Desktop app (Tauri)
pnpm --dir apps/desktop dev     # opens the native window with hot reload
pnpm --dir apps/desktop build   # installers in apps/desktop/src-tauri/target/release/bundle

# HTTP API
cargo run -p pf-api -- gen-key --id local-dev --prefix pk_test   # prints a secret + keys.toml entry
PF_KEYS_FILE=keys.toml cargo run -p pf-api                       # http://localhost:8080/v1/health

# Laravel package and example shop
(cd integrations/laravel && composer install && vendor/bin/phpunit)
(cd examples/laravel-shop && composer install && php artisan test)
```

## Documents

| # | Document | What it answers |
|---|---|---|
| 00 | [Clean-room policy](docs/00-clean-room-policy.md) | What we may and may not use as input |
| 01 | [Product spec](docs/01-product-spec.md) | What the app does, Free vs Premium, requirements |
| 02 | [Architecture](docs/02-architecture.md) | Crates, layers, tech choices, core API |
| 03 | [Flows](docs/03-flows.md) | User, data, Laravel, licensing and authoring flows (diagrams) |
| 04 | [Component format](docs/04-component-format.md) | How box styles are defined (`*.pfc.toml`) |
| 05 | [API & integrations](docs/05-api-and-integrations.md) | Web component, HTTP API, Laravel package, CLI |
| 06 | [Licensing](docs/06-licensing.md) | Plans, desktop tokens, web/API enforcement |
| 07 | [Implementation plan](docs/07-implementation-plan.md) | Stages 0–10, deliverables, done criteria, open decisions |
| ADR | [0001 — Stack](docs/adr/0001-stack.md) | Why Rust + WASM + Svelte + Tauri + axum |

## At a glance

```
packforge-core (Rust) ──► WASM ──► <packforge-designer> ──► any web page / Laravel Blade
        │                              └──► Tauri desktop (Win / macOS / Linux)
        ├──► pf-api (Docker) ──► packforge/laravel (composer) and any HTTP client
        └──► pf CLI
```
