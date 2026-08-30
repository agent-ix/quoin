---
id: NFR-015
title: "The semantic audit is complete and reproducible"
type: NFR
quality_attribute: reliability
relationships:
  - target: "ix://agent-ix/quoin/FR-051"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-052"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-053"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-054"
    type: "constrains"
---

# NFR-015: The semantic audit is complete and reproducible

## Statement

The semantic audit SHALL close and reconcile every declared denominator, retain all error states,
and reproduce the same canonical content from the same identified inputs.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Default modules inventoried / default modules declared | 100% | 100% | Test (TC-1188) |
| Type declarations with complete axis assessments / type declarations inventoried | 100% | 100% | Test (TC-1189) |
| Markdown paths assigned exactly one parse state / Markdown paths discovered | 100% | 100% | Test (TC-1190) |
| Canonical artifact bytes differing across equal-input reruns | 0 | 0 | Test (TC-1191) |

## Verification

Fixture tests exercise duplicates, absent instances, malformed Markdown, placeholder schemas,
provenance conflicts, and equal-input reruns. The retained campaign artifacts must pass the same
validator as the fixtures.

## Dependencies

- **Upstream**: [FR-051](../functional/FR-051-snapshot-semantic-audit-scope.md) through
  [FR-054](../functional/FR-054-publish-semantic-audit-artifacts.md)
