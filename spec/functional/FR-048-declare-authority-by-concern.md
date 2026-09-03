---
id: FR-048
title: "Declare semantic data authority by concern"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-013"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/StR-002"
    type: "traces_to"
---

# FR-048: Declare semantic data authority by concern

## Description

Where a semantic concept has multiple representations, the architecture record SHALL identify the
authoritative source, derived representations, permitted edit direction, and provenance obligations
for each concern.

## Rationale

Markdown, JSON Schema, generated language types, database rows, wire messages, analytical batches,
and reports are all useful. None is universally canonical. Authority follows the concern so a
projection can be optimized without becoming a competing definition.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-048-AC-1 | Reviewed typed Markdown is authoritative for human/agent-authored durable knowledge; extracted JSON, graph records, search indexes, embeddings, and rendered views are derived. | Test (TC-1134) |
| FR-048-AC-2 | Accepted schema/package source and metadata are authoritative for shared definitions; JSON Schema and generated Rust, TypeScript, and Python packages are reproducible outputs and never independent authorities. | Test (TC-1135) |
| FR-048-AC-3 | The record cites filament-core-data ADR-0005 for the structural schema source (TypeSpec) and does not re-decide, hedge, or describe a fallback for that source. | Test (TC-1136) |
| FR-048-AC-4 | Owning PostgreSQL or event stores are authoritative for transactional state, and owning run/evidence/event stores are authoritative for operational observations; Markdown reports remain projections unless separately authored as new artifacts. | Test (TC-1137) |
| FR-048-AC-5 | Interface schema packages govern payload conformance while Protobuf, JSON, and Avro remain selectable wire projections; Arrow/Parquet and CSV/TSV remain analytical or export projections with declared loss. | Test (TC-1138) |
| FR-048-AC-6 | Apparent competing authorities stop promotion and require an explicit projection, concern split, or reviewed ADR/migration disposition; last-writer-wins is rejected. | Test (TC-1139) |

## Constraints

- This requirement does not retire Avro, select one universal wire format, or authorize a schema
  or data migration. The schema-source decision is owned by filament-core-data (ADR-0005).

## Dependencies

- **Upstream**: [US-013](../usecase/US-013-reason-about-semantic-module-boundaries.md),
  [StR-002](../stakeholder/StR-002-extensible-vocabulary.md)
- **External basis**: `filament-core-data` ARCH-003, ARCH-006, ARCH-009, and issue #4 evidence
