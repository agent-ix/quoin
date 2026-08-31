---
id: SR-092
title: "Integrity review of Quoin change-assurance contracts"
type: SpecReview
analysis: integrity
scope: "US-017, FR-063..FR-065, TC-1261..TC-1292"
review_set: all
---

# SR-092: Integrity review of Quoin change-assurance contracts

## Summary

Every story outcome traces to FR-063 sealing, FR-064 intake, or FR-065
verification, and each acceptance criterion has one collision-safe matrix row.
The review made the ix-flow dependency metadata agree with the body text.

## Findings

| ID      | Severity | Summary                                                                                                                                         | Refs                                                     | Escape Cause        |
| ------- | -------- | ----------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- | ------------------- |
| FND-001 | medium   | Resolved: the requirements relied on ix-flow FR-018 history semantics in prose but omitted that dependency from machine-readable relationships. | FR-063 relationships; FR-065 relationships; Dependencies | missing-requirement |

## Resolution

FR-063 and FR-065 now declare both ix-flow FR-013 and FR-018. The chain remains
acyclic: US-017 → FR-063 → FR-064 → FR-065, with FR-030/032 and ix-flow as
upstream inputs and no reverse ownership edge.
