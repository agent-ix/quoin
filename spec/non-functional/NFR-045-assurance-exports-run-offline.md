---
id: NFR-045
title: "Assurance exports run without network access"
type: NFR
quality_attribute: reliability
relationships:
  - target: "ix://agent-ix/quoin/FR-221"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-226"
    type: "constrains"
---

# NFR-045: Assurance exports run without network access

## Statement

The GSN and SACM export paths SHALL make zero network requests at runtime.

## Scope

- Applies to: `quoin assurance --gsn` and `quoin assurance --sacm`, including
  well-formedness checking and local report generation.
- Operational context: export runs using local case and evidence inputs.

## Rationale

Assurance exports must be reproducible from local case data.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Runtime network requests made by either export | 0 | 0 | Test |

## Verification

Run both exports with network access disabled and assert that neither makes a
network request.

## Dependencies

- **Upstream**: [FR-221](../functional/FR-221-gsn-model-export.md), [FR-226](../functional/FR-226-sacm-export.md)
