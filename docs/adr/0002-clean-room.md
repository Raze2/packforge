# ADR-0002 — Clean-room development

- **Status:** Accepted
- **Date:** 2026-10-18

## Context

PackForge enters a market with established commercial tools. Some of their binaries were
analysed before this project started. Using any of that material would make PackForge a
derivative work and put the product at legal risk.

## Decision

Follow [00-clean-room-policy.md](../00-clean-room-policy.md): implement only from our functional
spec, public standards, published literature and permissively licensed open source. No
competitor code, decompiler output, data or assets enter the repository.

## Consequences

- Geometry for every style is derived and verified independently (physical samples).
- The component library is built from scratch, so it grows over time.
- Every PR confirms compliance via the PR template checklist.
