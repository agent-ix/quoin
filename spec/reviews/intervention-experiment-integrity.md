---
id: SR-061
title: "Integrity review of intervention-experiment evidence"
type: SpecReview
analysis: integrity
scope: "US-015; FR-056; FR-057; TC-1195..TC-1216"
review_set: all
---

## Summary

The requirements are atomic and consistent after separating treatment-specific
changes from held constants and tightening conclusion preconditions. Schema rules,
semantic validation, acceptance criteria, and matrix rows now agree.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | Resolved: the former global variable/effect shape could not represent multiple treatments without ambiguous joins. | FR-056-AC-4; FR-056-AC-5 | wrong-requirement |
| FND-002 | high | Resolved: conclusion kinds now have consistent observation, sample, confidence, and confounder preconditions. | FR-056-AC-8; FR-057-CON-2 | wrong-requirement |
| FND-003 | low | The revised acceptance criteria each have a direct TC mapping, including the added raw-evidence refusal obligation. | FR-056; FR-057; TC-1195..TC-1216 | correct-requirement-no-evidence |
| FND-004 | medium | Resolved: the record now carries the observation timestamp required by the governing FR-044 collection contract. | FR-044-AC-1; FR-056-AC-1; TC-1195 | missing-requirement |
