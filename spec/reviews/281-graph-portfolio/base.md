---
id: SR-098
title: "Base review of Quoin #281 graph portfolio requirements"
type: SpecReview
analysis: base
scope: "StR-007, US-019, FR-066, FR-067, TC-1293..TC-1316"
review_set: all
relationships:
  - target: "ix://agent-ix/quoin/StR-007"
    type: reviews
  - target: "ix://agent-ix/quoin/US-019"
    type: reviews
  - target: "ix://agent-ix/quoin/FR-066"
    type: reviews
  - target: "ix://agent-ix/quoin/FR-067"
    type: reviews
---
# Base review of Quoin #281 graph portfolio requirements

## Summary

The complete #281 requirement slice is identifier-safe, internally linked,
atomic, and covered one criterion per planned test row. Review corrected the
unrelated StR-004 trace, qualified the local/Quire FR-067 collision, removed a
premature local link to #152, and made identity and failure behavior explicit.

## Findings

Implementation reconciliation found that FR-062 deliberately requires three
explicit caller-owned inputs while FR-067 originally named only its export.
FR-067 now names repository-scoped export, premises, and audit mappings,
rejects conflicts before reads, and treats partial triples as local
incompatible gaps without discovery or implicit acceptance.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No unresolved base-review defects remain after the traced corrections | StR-007, US-019, FR-066, FR-067 |

## Coverage Review

| Rule | Result | Evidence |
| --- | --- | --- |
| Every criterion covered | PASS | StR-007-VC-1 and FR-066/067 ACs map to TC-1293..TC-1316 |
| Option permutations | PASS | Adapter names, premise mismatches, availability states, changed seeds, and mapping conflicts are enumerated |
| Constraint boundaries | PASS | TC-1304 and TC-1315 own execution/write boundaries; TC-1303 and TC-1314 own compatibility |
| Error paths | PASS | Both FRs define named error conditions and TC-1293..1299, TC-1308, TC-1312..1313 exercise them |
| State transitions | PASS | Plan active/inactive, measured/not-computed, compatible/incompatible, and current/history are explicit |
| Edge cases | PASS | Duplicate partitions, duplicate mappings, corrupt siblings, empty inputs, and input permutations are covered |

## Traceability

`StR-007 -> US-019 -> FR-066 -> FR-067`; FR-066 and FR-067 each carry explicit
verification evidence paths and the matrix carries all 24 unique TC rows.
