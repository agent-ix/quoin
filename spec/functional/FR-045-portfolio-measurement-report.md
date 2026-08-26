---
id: FR-045
title: "Portfolio report over repository measurement stores"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-044"
    type: "extends"
---

# FR-045: Portfolio report over repository measurement stores

## Description

When repository locations are supplied, `quoin report --portfolio` SHALL load
their existing assurance artifacts and measurement stores and SHALL render one
portfolio view. The command accepts locations and presentation options, never a
typed observation value.

The view SHALL retain repository, metric, definition, producer configuration,
and population boundaries. It SHALL reuse collection comparison semantics and
SHALL NOT create an aggregate quality score, policy verdict, evidence record, or
new metric.

## Rationale

The first QA program spans Quoin, Quire, qa-corpus, engineering-assurance, and
spec-artifacts-process. Reading their stores separately makes missing evidence
easy to overlook and invites unlike populations to be summarized as one score.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-045-AC-1 | One command accepts repeated repository locations and shows each readable repository's active AssuranceProfiles, active MeasurementPlans, latest collection provenance, corpus gaps, observations, and explicit `not_computed` rows. It accepts no observation value. | Test (TC-1013) |
| FR-045-AC-2 | Missing repositories, unreadable collections, missing stores, and repositories whose latest collection is more than 30 days behind the newest portfolio collection are named independently. A missing or unreadable value is never zero. | Test (TC-1011) |
| FR-045-AC-3 | Each observation names its plan path and collection path. Latest-pair comparisons reuse FR-044 compatibility results, including changed definitions, without combining values across repositories or assigning a verdict. | Test (TC-1011, TC-1012) |
| FR-045-AC-4 | Human and canonical JSON output render the same loaded report object deterministically, and the human view explicitly states that it computes no cross-repository aggregate. | Test (TC-1012) |

## Constraints

- The portfolio view reads and reports; it runs no producer and writes no store.
- Staleness is relative to the newest collection in the requested portfolio,
  so the same stores produce the same result without consulting a wall clock.
- Historical collections are structurally validated but remain readable after
  the active plan definition changes; the comparison reports that change.

## Dependencies

- FR-044 defines plan-governed collections, comparisons, and single-repository reports.
- FR-043 defines the controlled Tier-1 observations shown by the first portfolio.
