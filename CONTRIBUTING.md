# Contributing

1. Read [docs/00-clean-room-policy.md](docs/00-clean-room-policy.md) first. It is binding.
2. Core crates (`pf-geom`, `pf-model`, `pf-expr`, `pf-component`, `pf-export`, `pf-fold3d`,
   `pf-optimize`, `pf-license`, `pf-core`) must stay pure: no filesystem, network, clock,
   threads or randomness. They must build for `wasm32-unknown-unknown`.
3. Before pushing:

   ```bash
   cargo fmt --all
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   cargo build -p pf-wasm --target wasm32-unknown-unknown
   ```

4. Golden files in `tests/golden/` change only on purpose. Regenerate with
   `UPDATE_GOLDEN=1 cargo test -p pf-core` and review the diff.
5. Architectural decisions get an ADR in `docs/adr/`.
