# 00 — Clean-Room Policy

PackForge is an **independent** packaging-design application. It competes in the same
category as existing commercial tools (e.g. treeDiM PackLib) but contains **none of their
code, data, or assets**. This policy is binding for everyone contributing to the project,
including AI assistants.

> This is an engineering policy, not legal advice. Have a lawyer review it before a
> commercial release.

## Allowed inputs

| Source | Examples |
|---|---|
| Public standards | FEFCO code (corrugated), ECMA code (folding cartons), DXF reference, CF2 spec, HPGL reference, ISO pallet sizes |
| Published literature | Packaging-engineering textbooks, bin/pallet-packing papers, public tutorials |
| Permissively licensed open source | MIT / Apache-2.0 / BSD crates and libraries (check license before adding) |
| Behavioural observation | *What* a competing product does: feature names, menu workflows, supported export formats, units, free-vs-premium split — written as plain-language behaviour |
| Our own engineering | Geometry derived from first principles, our own tests against physical samples |

## Forbidden inputs

- Decompiled, disassembled, or reverse-engineered **code** from any competing product,
  in any language, including "translated" or "paraphrased" versions.
- Their class/module structure, method breakdown, algorithms, formulas, or constants.
- Their component library: designs (`.des`, `.des3`), parametric plugin DLLs, thumbnails,
  PDFs, databases (`PicParam.db`), report templates, post-processor configs, images, icons.
- Their licensing scheme or keys.
- GPL/AGPL code, unless we deliberately decide to license a module compatibly (needs an ADR).

## Process

1. The **functional spec** ([01-product-spec.md](01-product-spec.md)) describes behaviour only.
   It is the single bridge between "what exists in the market" and "what we build".
2. Implementers work from the spec + public standards. They do not open competitor binaries
   or decompiler output while implementing.
3. Where behaviour is unclear, resolve it from standards, physical samples, or user feedback —
   never by inspecting competitor implementation.
4. Every new dependency is checked for license compatibility (`cargo deny` in CI).
5. Interoperability features (e.g. importing a competitor's file format) require a separate ADR
   and legal sign-off before any work starts.

## Status of existing reverse-engineering material

The extracted installer contents and analysis notes in `../PackLib_11.0_extracted` and the
scratch analysis are **not** project inputs. They must not be copied into this repository.
