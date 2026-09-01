---
id: SR-099
title: "Failure-domain review of Quoin #281 graph portfolio requirements"
type: SpecReview
analysis: failure-domain
scope: "StR-007, FR-066, FR-067"
review_set: all
relationships:
  - target: "ix://agent-ix/quoin/FR-066"
    type: reviews
  - target: "ix://agent-ix/quoin/FR-067"
    type: reviews
---
# Failure-domain review of Quoin #281 graph portfolio requirements

## Summary

The slice fails closed at producer-contract and digest boundaries while
isolating repository and collection read failures. No callback, evaluation, or
unbounded traversal is introduced; FR-062 remains the owner of graph topology.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No unresolved failure-domain gap remains | FR-066, FR-067 |

## Failure-Domain Analysis

| Domain | Contract |
| --- | --- |
| Extension points | Adapter lookup is exact/versioned; unknown adapters fail before parsing and perform no fallback |
| Entity identity | Observation id, collection id, repository absolute path, population identity, and normalized partition tuple are explicit |
| Evaluation purity | Adapters and portfolio run no producer, Quire, Git, network, suite, scorer, or write operation |
| Topology | #281 consumes FR-062 report objects without walking or rebuilding graph relationships |
| Partial failure | Invalid adapter input constructs no partial collection; corrupt stores/exports become local gaps and preserve readable siblings |
| Multiplicity | Duplicate normalized partitions and conflicting graph-export mappings fail; equivalent repositories and changed seeds deduplicate |

## Resolved During Review

The initial text did not say whether one corrupt collection hid readable
history or how conflicting repository mappings behaved. FR-067 now specifies
both outcomes and assigns them to TC-1312 and TC-1313.
