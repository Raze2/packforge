# ADR-0001 — Technology stack

- **Status:** Proposed
- **Date:** 2026-10-18

## Context

PackForge must run in browsers, on Windows/macOS/Linux desktops, and as a server API callable
from Laravel/PHP and other stacks, with identical geometry and export results everywhere.

## Decision

- Core in **Rust**, pure and WASM-safe (no I/O, no clock), exposed via a facade crate `pf-core`.
- **WebAssembly** (`wasm-bindgen`) for browser and Node use.
- **Svelte + TypeScript** single UI, shipped as a standalone app, a custom element, and inside
  **Tauri 2** for desktop.
- **axum** HTTP service in Docker for server-side use; a **Laravel** composer package as client.
- Components defined as **declarative TOML** evaluated by our own sandboxed expression engine.

## Consequences

- One implementation of geometry ⇒ consistent results; golden tests cover all targets.
- Core crates cannot use crates that need threads/filesystem/OS APIs unless feature-gated.
- UI work is web tech; desktop-native look comes from Tauri menus/dialogs and careful design.
- PHP projects depend on the HTTP service for server-side generation (no native PHP extension in v1).

## Alternatives considered

- **egui/iced native UI**: pure Rust, but a second UI would be needed for the web embed.
- **Electron**: larger binaries; Tauri covers the need.
- **PHP FFI to a Rust cdylib**: possible later; HTTP is simpler to deploy and language-neutral.
- **General scripting engine (rhai/Lua) for components**: more power, but harder to sandbox,
  validate and keep deterministic; revisit if the declarative format proves insufficient.
