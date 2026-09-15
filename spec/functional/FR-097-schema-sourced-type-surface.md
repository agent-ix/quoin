---
id: FR-097
title: "Source every boundary type from one schema and refuse hand-written duplicates"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-009"
    type: "implements"
  - target: "ix://agent-ix/quoin/US-024"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-096"
    type: "requires"
---

# FR-097: Source every boundary type from one schema and refuse hand-written duplicates

> **⛔ Withdrawn — 2026-09-15 at Stage 9 native cutover (`33ca665`).** This
> requirement described the temporary TypeScript-facing `quoin-core` subprocess
> boundary: `quoin-schemas`, generated `src/core/types.ts`, and its npm-facing
> provenance gate. The final architecture has no such boundary or generated
> TypeScript surface. Keeping those criteria open would require recreating the
> Node surface that FR-102 deliberately removed.

## Disposition

The underlying schema-provenance intent remains, but its implementation is now
owned by the native crates that consume it:

- Rust domain types are defined and serialized in their owning crate; they are
  not mirrored into a generated TypeScript boundary.
- Vendored upstream schema bytes retain their source revision and SHA-256 pin
  beside the native reader. `quoin-quire`'s assurance-schema tests are the
  current executable evidence for that contract.
- `filament-core-data` remains the external schema authority where it publishes
  a type. There is no first-party TypeScript declaration surface left for this
  repository to generate or audit.

The retired TC-1612 through TC-1617 and TC-1691 rows remain in the matrix as
historical withdrawn criteria. Native executable delivery and unpublished Rust
crates are verified under FR-102 instead.

## Dependencies

- **Superseded by**: [FR-102](./FR-102-command-surface-and-oclif-retirement.md)
  for the native executable and delivery boundary.
- **Retained schema provenance**: `quoin-quire` native schema-pin tests.
