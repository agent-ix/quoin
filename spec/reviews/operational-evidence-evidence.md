---
id: SR-071
title: "Evidence-method review of operational evidence"
type: SpecReview
analysis: evidence
scope: "US-016; FR-059; FR-060; TC-1223..TC-1243"
review_set: all
---

## Summary

`quoin advise --mismatch-only` reports no method mismatch for FR-059 or FR-060.
Unit, property, integration, and static evidence are placed at their schema,
temporal-invariant, I/O, and no-control-execution boundaries.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | Resolved: the no-derived-score criterion is executable and mapped to a unit test rather than inspection. | FR-060-AC-9; TC-1240 | wrong-requirement |
| FND-002 | high | Resolved: temporal state, matching, raw-evidence authenticity, and pin/link integrity have property-test obligations over invalid combinations. | TC-1228; TC-1229; TC-1231; TC-1233; TC-1238 | correct-requirement-no-evidence |
