---
id: NFR-045
title: "Assurance exports run offline without a copied SACM metamodel"
type: NFR
quality_attribute: reliability
relationships:
  - target: "ix://agent-ix/quoin/FR-221"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-222"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-226"
    type: "constrains"
---

# NFR-045: Assurance exports run offline without a copied SACM metamodel

## Statement

The GSN and SACM exports SHALL run without network access and SHALL NOT vendor
a copy of the SACM metamodel.

## Scope

- Applies to: `quoin assurance --gsn` and `quoin assurance --sacm`, including
  their well-formedness checks and local report generation.
- Operational context: the explicit external metamodel-conformance test may
  read a local model supplied by `QUOIN_SACM_METAMODEL`; normal export does
  not need that file.

## Rationale

Assurance exports must be reproducible from local case data, while the
normative metamodel remains owned and published by OMG.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Runtime network requests made by either export | 0 | 0 | Test |
| Copies of the SACM metamodel in the quoin repository | 0 | 0 | Inspection |
| New runtime dependencies outside the workspace other than the approved XML writer | 0 | 0 | Inspection |

## Verification

Run both exports with network access disabled. Inspect the repository for the
absence of a copied metamodel and inspect runtime dependencies to ensure that
only the approved XML writer can be outside the workspace.

## Dependencies

- **Upstream**: [FR-221](../functional/FR-221-gsn-model-export.md), [FR-222](../functional/FR-222-gsn-model-well-formedness.md), [FR-226](../functional/FR-226-sacm-export.md)
