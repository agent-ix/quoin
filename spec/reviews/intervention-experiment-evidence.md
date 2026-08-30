---
id: SR-063
title: "Evidence-method review of intervention-experiment evidence"
type: SpecReview
analysis: evidence
scope: "US-015; FR-056; FR-057; FR-058; TC-1195..TC-1216; TC-1217..TC-1221"
review_set: all
---

## Summary

`quoin advise --mismatch-only` reports no method mismatch for FR-056, FR-057, or FR-058
after correction. Unit, property, integration, and static evidence are allocated at
their corresponding schema, invariant, I/O, and non-execution boundaries.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | Resolved: the no-derived-score criterion is executable and mapped to a unit test rather than inspection. | FR-057-AC-9; TC-1212 | wrong-requirement |
| FND-002 | medium | Resolved: raw-evidence authenticity now has a property-test obligation over unsafe, missing, wrong-sized, and wrong-digest cases. | FR-057-AC-11; TC-1216 | correct-requirement-no-evidence |
| FND-003 | high | Resolved: the first real producer has integration, property, and end-to-end evidence over real-run parsing, refusal, digest derivation, honest conclusion, and no-process behavior. | TC-1217..TC-1221 | correct-requirement-no-evidence |
