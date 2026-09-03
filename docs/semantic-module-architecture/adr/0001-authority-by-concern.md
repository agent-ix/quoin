---
id: ARCH-SM-ADR-0001
title: "Assign semantic data authority by concern"
status: proposed
date: 2026-08-29
requirements:
  - FR-046
  - FR-048
  - FR-049
---

# ADR-0001: Assign semantic data authority by concern

## Context

Quoin and Quire begin from typed Markdown because that is the best durable authoring and reasoning
surface for people and agents. The wider system also uses extracted JSON, PostgreSQL, event and
evidence stores, Protobuf/JSON/Avro boundaries, Arrow/Parquet analysis, CSV/TSV exports, generated
Rust/TypeScript/Python types, and rendered reports. Calling any one of these universally canonical
would make it the wrong authority for several other concerns.

## Proposed decision

Assign authority by concern and data plane. Reviewed typed Markdown owns authored durable knowledge;
accepted schema/package source plus metadata owns shared semantic definitions; application and event
stores own transactional state; run/evidence/event stores own operational observations; versioned
interface packages own payload conformance. Generated code, wire bytes, analytical batches, exports,
and reports are projections unless explicitly authored under a separate concern.

Require every mapping or transformation to name its source, target, preservation level, failure
outcome, and provenance. Stop promotion when two sources claim the same concern until a projection,
concern split, or successor ADR/migration resolves it.

## Consequences

- Markdown remains first-class for humans and LLMs without becoming the runtime database.
- PostgreSQL remains first-class for owned application state without becoming the source of
  requirements or architecture.
- Protobuf, JSON, and Avro may be selected per wire boundary; Arrow/Parquet and CSV/TSV may be
  selected for analysis/export with declared loss.
- Generated Rust, TypeScript, and Python packages are strongly typed and reusable but never become
  independent contract authorities.
- Dynamic modules and finite packages can coexist because authority and consumption shape are
  separate questions.

## Alternatives considered

- **Markdown universally canonical:** rejected for operational and transactional occurrences.
- **PostgreSQL universally canonical:** rejected for reviewable authored knowledge and portable
  package definitions.
- **Generated language classes canonical:** rejected because it privileges one consumer language
  and permits hand-edited cross-language drift.
- **Modular JSON Schema as the authoring source:** rejected. TypeSpec is the structural schema
  source per filament-core-data ADR-0005 (owner decision on filament-core-data#4, 2026-09-03);
  JSON Schema is a generated projection.
- **Last writer wins between representations:** rejected because it hides competing ownership.

## Compatibility

No existing Avro, module, Markdown, database, API, or generated-binding behavior changes. Adoption
requires separate compiler, publication, mapping, and migration tickets.

## Promotion

Status remains proposed until the issue #289 PR receives named Quoin/Quire maintainer review. A
future successor must retain explicit concern authority, edit direction, provenance, and conflict
handling even if the selected schema technology changes.
