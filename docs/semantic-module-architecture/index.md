---
id: ARCH-SM-001
title: "Semantic-module architecture"
status: proposed
issue: "https://github.com/agent-ix/quoin/issues/289"
---

# Semantic-module architecture

This record applies the semantic-data architecture established in
`agent-ix/filament-core-data` to Quoin, Quire, domain modules, generated packages, and
their consumers. It is a meta-system guide: domain data is modeled once, while Markdown,
JSON, database rows, wire payloads, analytical batches, and UI reports remain explicit
authorities or projections according to their concern.

The record is architecture-only. It changes no Quoin behavior, Quire behavior, module
manifest, schema, generated package, publication process, database, or consumer. Its local
ADRs remain proposed until the issue #289 pull request receives the named Quoin/Quire
maintainer review required by [NFR-014](../../spec/non-functional/NFR-014-non-disruptive-architecture-record.md).

## Reading order and status

| Record                                                                                       | Status             | Purpose                                                                                     |
| -------------------------------------------------------------------------------------------- | ------------------ | ------------------------------------------------------------------------------------------- |
| [Planes and authority](planes-and-authority.md)                                              | proposed normative | Four data planes, concern-specific authority, projections, provenance, and conflicts.       |
| [Ownership and boundaries](ownership-and-boundaries.md)                                      | proposed normative | Positive and negative ownership for Quire, Quoin, the compiler, modules, and consumers.     |
| [Dynamic and generated modules](dynamic-and-generated.md)                                    | proposed normative | Compatibility between open runtime modules and finite generated packages.                   |
| [External decision ledger](decision-ledger.md)                                               | normative evidence | Exact external status and the local compatibility disposition for every relied-on decision. |
| [ADR index](adr/index.md)                                                                    | proposed           | Local decisions and their promotion gate.                                                   |
| [ADR-0001: Authority by concern](adr/0001-authority-by-concern.md)                           | proposed           | Reject one universally canonical representation.                                            |
| [ADR-0002: Preserve Quire and Quoin boundaries](adr/0002-preserve-quire-quoin-boundaries.md) | proposed           | Add compiler/package concerns without absorbing existing owners.                            |

## Authority of this record

After maintainer acceptance, the topical records and accepted local ADRs are normative for
future Quoin semantic-module work. The [external decision ledger](decision-ledger.md) is not
authority over another repository: it records the authority and status held there. A
provisional external decision remains provisional here, and an external gate remains an
external gate.

## Governing requirements

- [US-013](../../spec/usecase/US-013-reason-about-semantic-module-boundaries.md)
- [FR-046](../../spec/functional/FR-046-record-semantic-data-planes.md) through
  [FR-050](../../spec/functional/FR-050-reconcile-quire-decisions.md)
- [NFR-013](../../spec/non-functional/NFR-013-traceable-semantic-architecture.md) and
  [NFR-014](../../spec/non-functional/NFR-014-non-disruptive-architecture-record.md)
- [PLAN-002](../../plan/PLAN-002-semantic-module-architecture/plan.md)

## Non-goals

- Selecting one universal document, database, analytical, or wire representation.
- Retiring Avro or implementing the TypeSpec compiler; both belong to `filament-core-data`.
- Implementing or publishing a compiler, emitter, or generated package.
- Changing current module discovery, validation, extraction, persistence, or rendering.
- Moving a decision owned by Quire, `filament-core-data`, a module, or a consumer into Quoin.
