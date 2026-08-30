---
id: FR-053
title: "Score and disposition every declared semantic type"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-014"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-046"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-052"
    type: "depends_on"
---

# FR-053: Score and disposition every declared semantic type

## Description

For every type declaration in the inventory, the audit SHALL produce an evidence-backed semantic
fit assessment whose individual axes remain visible and whose disposition uses a closed vocabulary.

## Rationale

A single grade would conceal the difference between a sound domain concept with a placeholder schema
and a representation-specific artifact that mixes definitions with accumulated events.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-053-AC-1 | Every declaration is assessed on vocabulary fit, structural fit, definition-versus-occurrence fit, identity, versioning, provenance, lifecycle, relationships, round-trip suitability, generated-code suitability, and accumulation behavior. | Test (TC-1169) |
| FR-053-AC-2 | Every axis records `supported`, `partial`, `conflict`, `missing`, or `not-applicable`, a confidence from `high`, `medium`, or `low`, and one or more inventory evidence references or an explanation for `not-applicable`. | Test (TC-1170) |
| FR-053-AC-3 | Every declaration receives exactly one disposition from `fits`, `fits-with-mapping`, `incomplete`, `conflict`, `representation-local`, or `deferred`, with a reason derived from its axis results. | Test (TC-1171) |
| FR-053-AC-4 | An unconstrained or placeholder object schema is identified explicitly and cannot yield `supported` generated-code or round-trip suitability solely because it validates. | Test (TC-1172) |
| FR-053-AC-5 | Duplicate type names and structurally similar archetypes are compared without collapsing their module-qualified identities, and incompatible definitions produce conflict-ledger entries. | Test (TC-1173) |
| FR-053-AC-6 | Schema-encoded JSON/string blobs and free-form Markdown fields are evaluated for lost structure, identity, relationships, and round-trip ambiguity. | Test (TC-1174) |
| FR-053-AC-7 | Definition-shaped types carrying run, result, evidence, timestamps, state transitions, or accumulated observations are evaluated for plane confusion. | Test (TC-1175) |
| FR-053-AC-8 | The review explicitly evaluates whether run, result, evidence, report, relationship, identity, version, provenance, and lifecycle concepts are absent, overloaded, or already represented. | Test (TC-1176) |

## Constraints

- Scores inform design; they do not change current validity or establish a migration order by themselves.
- A low-confidence assessment remains visible and SHALL NOT be promoted to a definitive conflict.

## Dependencies

- **Upstream**: [FR-052](./FR-052-inventory-default-module-corpus.md)
- **Architecture**: [FR-046](./FR-046-record-semantic-data-planes.md) through
  [FR-050](./FR-050-reconcile-quire-decisions.md)
