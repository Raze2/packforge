# apps/web — PackForge designer

Svelte 5 + TypeScript UI running the Rust core through WebAssembly. One codebase produces:

| Output | Path | Use |
|---|---|---|
| Standalone app | `dist/app/` | Static site: `index.html` (designer), `embed-demo.html` |
| Web component | `dist/wc/packforge-designer.js` | One self-contained ES module (WASM + CSS inlined) |

## Requirements

Rust with the `wasm32-unknown-unknown` target, `wasm-bindgen-cli` matching `Cargo.lock`
(`cargo install wasm-bindgen-cli --version 0.2.129`), Node 22+, pnpm 9.

## Commands

```bash
pnpm dev          # builds WASM bindings, starts Vite on http://localhost:5173
pnpm build        # WASM + type check + app + web component
pnpm check        # svelte-check (fails on warnings)
```

Open `http://localhost:5173/embed-demo.html` to see the web component on a host page.

## Layout

```
src/lib/core.ts          typed wrapper around the WASM module
src/lib/Designer.svelte  main designer (used by the app and the web component)
src/lib/components/      DielineView (pan/zoom), ParamForm, Library, StatsBar
src/lib/i18n.ts          EN / FR / AR (RTL)
src/wc/                  <packforge-designer> custom element wrapper
scripts/build-wasm.mjs   cargo build + wasm-bindgen → src/lib/wasm/ (generated, git-ignored)
```
