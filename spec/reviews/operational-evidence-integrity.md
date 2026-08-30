---
id: SR-069
title: "Integrity review of operational evidence"
type: SpecReview
analysis: integrity
scope: "US-016; FR-059; FR-060; TC-1223..TC-1243"
review_set: all
---

## Summary

Schema rules, semantic integrity, intake behavior, reporting, and the matrix now
agree on observation time, clock support, final exercise records, pin identity,
and discharge preconditions.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | Resolved: `met`, `missed`, and `open` states now have mutually checkable temporal predicates and reject impossible ordering. | FR-059-AC-4; FR-059-AC-6; TC-1226; TC-1228 | wrong-requirement |
| FND-002 | medium | Resolved: unsupported capability clocks exclude event/deadline fields, while supported clocks require all three. | FR-059-AC-3; TC-1225 | wrong-requirement |
| FND-003 | medium | Resolved: pin controls require a matching, unique pin identity and linked capabilities must match kind, subject, scope, and control id. | FR-059-AC-7; FR-059-AC-9; TC-1229; TC-1231 | missing-requirement |
| FND-004 | medium | Resolved: the record now carries the observation timestamp required by FR-044 and used by deterministic clock validation. | FR-044-AC-1; FR-059-AC-1; TC-1223 | missing-requirement |
