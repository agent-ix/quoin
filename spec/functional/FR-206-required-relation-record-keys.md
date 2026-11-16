---
id: FR-206
title: "Load record keys on required relation declarations"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-115"
    type: "extends"
  - target: "ix://agent-ix/quire-rs/FR-076"
    type: "requires"
    cardinality: "1:1"
---

# FR-206: Load record keys on required relation declarations

## Description

The quoin matrix command SHALL load module manifests whose
`traceability.required_relations[]` entries contain the optional `record_keys`
field supported by quire-rs v0.61.0. Quoin delegates this manifest contract to
its linked quire-rs engine and SHALL NOT implement a second parser or reinterpret
the field.

When present, `record_keys` SHALL be a non-empty list of distinct, non-blank
strings. Quire-rs SHALL preserve each key's spelling and list order. When
`record_keys` is null, an empty list, a non-list value, contains a non-string
entry, contains a blank string, or contains a duplicate entry, quire-rs SHALL
refuse the declaration with an error naming the required relation. Omission SHALL
retain existing required-relation behavior. Other undeclared keys SHALL remain
rejected by the manifest's deny-unknown-fields contract.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-206-AC-1 | A module with a required relation whose `record_keys` contains distinct non-blank strings loads, and the engine preserves the key names and their order. | Test |
| FR-206-AC-2 | `null`, an empty list, a scalar instead of a list, a non-string list entry, a blank or whitespace-only string, and a duplicate key each fail while naming the relation. | Test |
| FR-206-AC-3 | A required relation with no `record_keys` loads and keeps existing behavior. | Test |
| FR-206-AC-4 | An undeclared key other than `record_keys` remains rejected. | Test |
| FR-206-AC-5 | `quoin matrix --repo <architecture-repository> --json` loads the current `connection-carries-classification.record_keys` declaration and computes its matrix without `CORE_REFUSED` / `QQ-1010`. | Test |
| FR-206-AC-6 | Quoin's linked quire-rs dependency is the released v0.61.0 contract; Quoin has no parallel `required_relations.record_keys` parser. | Inspection |

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-206-CON-1 | Quoin SHALL delegate required relation manifest parsing and validation to quire-rs without copying or vendoring its implementation. | Boundary | Inspection |

## Dependencies

- **Upstream**: [FR-115](./FR-115-evidence-backed-test-matrix.md), quire-rs [FR-076](ix://agent-ix/quire-rs/FR-076)
- **Downstream**: none
