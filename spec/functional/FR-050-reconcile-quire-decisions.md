---
id: FR-050
title: "Reconcile the semantic architecture with Quire decisions"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-013"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-047"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-048"
    type: "references"
---

# FR-050: Reconcile the semantic architecture with Quire decisions

## Description

When the semantic-module architecture cites an existing Quire decision, the record SHALL assign a
preserved, clarified, partially superseded, or deferred disposition that names the current governing
source.

## Rationale

Quire's ADR corpus spans an earlier three-layer document pipeline, a unified archetype shape, direct
Markdown validation, rendering removal, and the accepted Quire/Quoin role split. A new architecture
must not silently reinterpret older language or make a historical proposal current by citation.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-050-AC-1 | ADR-0003's unified artifact/object archetype shape is preserved as a structural parsing model and is not treated as a universal semantic runtime base class. | Test (TC-1145) |
| FR-050-AC-2 | ADR-0004 and the current Quire specification preserve direct typed Markdown and canonical Markdown within the document boundary. | Test (TC-1146) |
| FR-050-AC-3 | The rendering responsibility described in draft ADR-0002 is marked historical for Quire and superseded by the current render-removed specification; byte-splicing remains preserved. | Test (TC-1147) |
| FR-050-AC-4 | Accepted ADR-0011 remains governing for Quire/Quoin validation levels and capability roles. | Test (TC-1148) |
| FR-050-AC-5 | The reconciliation states that Quire does not become a renderer or cross-language generator and Quoin does not become the parser or semantic compiler. | Test (TC-1149) |
| FR-050-AC-6 | Every cited external decision records repository, path, decision status, and reviewed revision or date so later drift is visible. | Test (TC-1150) |

## Constraints

- This requirement does not edit or supersede Quire's ADR files. Any normative Quire change belongs
  in `quire-rs` and requires its maintainers' review.

## Dependencies

- **Upstream**: [FR-047](./FR-047-allocate-semantic-module-ownership.md),
  [FR-048](./FR-048-declare-authority-by-concern.md)
- **External basis**: Quire ADR-0002, ADR-0003, ADR-0004, ADR-0011, and the current Quire specification
