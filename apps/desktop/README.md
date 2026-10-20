# apps/desktop — PackForge desktop app

A Tauri 2 shell around the web designer (`apps/web`). The live preview uses the same
WebAssembly core; files and exports go through native code (`pf-core`).

## Features

- Native **Open / Save / Save As** dialogs for `.pfproj` project files (JSON inside), with
  **unsaved-changes** prompt on close and the document name in the window title
- **Export DXF / SVG** with a save dialog, written by the native core
- **Recent files** menu, **drag-and-drop** of `.pfproj` files, and the **file association**
  (double-click a `.pfproj` in Finder/Explorer)
- Native menu with shortcuts: ⌘/Ctrl N, O, S, ⇧S, E, ⇧E, 0 (fit), plus Edit (copy/paste in inputs)

## Develop

```bash
pnpm install                   # at the repository root
pnpm --dir apps/desktop dev    # starts the web dev server and the native window
```

Requires Rust, `wasm-bindgen-cli` 0.2.129 and the platform webview (WebView2 on Windows, WebKitGTK
on Linux: `libwebkit2gtk-4.1-dev`).

## Build installers

```bash
pnpm --dir apps/desktop build                     # all bundle types for this OS
pnpm --dir apps/desktop tauri build --bundles app # macOS .app only
```

Output: `src-tauri/target/release/bundle/`. CI (`.github/workflows/desktop.yml`) builds Windows
(`.msi`, `.exe`), macOS universal (`.app`, `.dmg`) and Linux (`.AppImage`, `.deb`). Builds are
unsigned until Stage 10 (code signing, notarisation, auto-update).

## Layout

```
src-tauri/src/main.rs    commands, menu, window events (close prompt, drag-drop), file association
src-tauri/src/files.rs   project read/write, native export, recent list (unit-tested)
src-tauri/tauri.conf.json
../web/src/desktop.ts    frontend side: native platform adapter and menu/event wiring
```
