---
id: NFR-023
title: "Published figures carry their provenance"
type: NFR
quality_attribute: maintainability
relationships:
  - target: "ix://agent-ix/quoin/FR-090"
    type: "constrains"
---

# NFR-023: Published figures carry their provenance

## Statement

The corpus measurement SHALL emit, for every numeric figure its published report prints, a
machine-readable reference naming the result artifact and field that figure was taken from.

## Scope

- Applies to: every number in the human-readable report, including counts, rates and shares, and the
  figure index of FR-090 that binds them to their artifacts.
- Operational context: a reader who did not run the measurement, reading the report months later.

## Rationale

Three figures published by this programme were later found not to match the measurement they claimed
to summarise. Requiring each number in the prose report to name the artifact it came from turns that
class of defect into something a reviewer can check by opening one file.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Report figures without a named source artifact | 0 figures | 0 figures | Automated cross-check of report figures against result artifacts |
| Report figures disagreeing with their source artifact | 0 figures | 0 figures | Automated recomputation from the result artifacts |
| Rates published without unit and population | 0 rates | 0 rates | Automated report lint |

## Verification

A check recomputes every figure the report prints from the result artifacts it names and fails when a
figure is absent from those artifacts or disagrees with them.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| NFR-023-AC-1 | Every numeric figure printed by the report names the result artifact it was taken from. | Test (TC-1563) |
| NFR-023-AC-2 | Every numeric figure printed by the report equals the value recomputed from the artifact it names. | Test (TC-1564) |
| NFR-023-AC-3 | Every rate printed by the report carries its unit and its population identifier. | Test (TC-1565) |

## Dependencies

- **Upstream**: [FR-090](../functional/FR-090-publish-rates-with-unit-population-and-method.md)
