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

- Applies to: every numeric figure printed by a **surviving** measurement
  renderer. That population is named rather than inherited, because the corpus
  report this requirement was first written against no longer exists — its
  harness was disposed of under
  [quoin#388](https://github.com/agent-ix/quoin/issues/388) and FR-084..FR-092,
  NFR-021 and NFR-022 are withdrawn with it. A requirement scoped to a report
  nothing produces is vacuous in exactly the way this requirement exists to
  prevent, so the scope states its population and the population is checkable:

  | renderer | figures it prints |
  | --- | --- |
  | `src/measurement/report.ts:109` `renderMeasurementReport` | `observation.value` with `observation.unit` (`:132`), and the finding-recall figures at `:190,196,199` |
  | `src/measurement/portfolio.ts:142` `renderPortfolioReport` | `observation.value` with `observation.unit` (`:204`), in the `Metric / State-value / Plan / Collection` table |

  The figures themselves are `MeasurementObservation` fields — `value`, `unit`
  and `population` (`src/measurement/types.ts:19-22`) — so the artifact a figure
  came from is the MeasurementCollection carrying that observation, and the
  binding is checkable without a separate figure index.
- Out of scope: the corpus report and its figure index. FR-090, which defined
  that index, is withdrawn.
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
| NFR-023-AC-4 | A planted figure with no binding fails the check: a printed numeric figure whose observation names no collection, and a printed rate with no unit or no population identifier, are each reported rather than passed over. A run that finds no figure to check is reported as inconclusive, never as zero unbound figures. | Test |

## Dependencies

- **Upstream**: [FR-090](../functional/FR-090-publish-rates-with-unit-population-and-method.md)
