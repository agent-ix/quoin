---
id: SR-063
title: "Evidence-method review of intervention-experiment evidence"
type: SpecReview
analysis: evidence
scope: "US-015; FR-056; FR-057; TC-1195..TC-1216"
review_set: all
---

## Summary

`quoin advise --mismatch-only` reports no method mismatch for FR-056 or FR-057
after correction. Unit, property, integration, and static evidence are allocated at
their corresponding schema, invariant, I/O, and non-execution boundaries.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | Resolved: the no-derived-score criterion is executable and mapped to a unit test rather than inspection. | FR-057-AC-9; TC-1212 | wrong-requirement |
| FND-002 | medium | Resolved: raw-evidence authenticity now has a property-test obligation over unsafe, missing, wrong-sized, and wrong-digest cases. | FR-057-AC-11; TC-1216 | correct-requirement-no-evidence |
