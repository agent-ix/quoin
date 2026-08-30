---
id: SR-060
title: "Failure-domain review of intervention-experiment evidence"
type: SpecReview
analysis: failure-domain
scope: "US-015; FR-056; FR-057; FR-058; TC-1195..TC-1216; TC-1217..TC-1221"
review_set: all
---

## Summary

The review exercised ambiguous identity, incomplete observation, unverifiable raw
evidence, collision, and causal-overstatement paths. The record and intake contracts
now make each unsafe state rejectable with deterministic evidence.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | Resolved: every changed variable and measured effect now identifies its treatment, with unique semantic keys and no dangling treatment references. | FR-056-AC-4; FR-056-AC-5; TC-1198; TC-1199 | wrong-requirement |
| FND-002 | high | Resolved: zero-sample, unobserved, or uncontrolled/unknown records cannot assert causal effect, and no-effect conclusions require an observed comparison. | FR-056-AC-8; TC-1202 | wrong-requirement |
| FND-003 | high | Resolved: unsafe, missing, wrong-sized, or digest-mismatched raw evidence refuses intake without a record write. | FR-056-AC-9; FR-057-AC-11; TC-1203; TC-1216 | missing-requirement |
| FND-004 | medium | Resolved: randomized assignment requires a retained seed so a claimed assignment cannot be irreproducible. | FR-056-AC-3; TC-1197 | missing-requirement |
| FND-005 | high | Resolved: malformed, empty, unversioned, duplicate, or mismatched real-producer inputs fail without a record, and inadequate attribution cannot become a causal claim. | FR-058-AC-2; FR-058-AC-4; TC-1218; TC-1220 | missing-requirement |
