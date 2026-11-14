---
id: SR-001
title: "Security control mapping gaps"
type: SpecReview
analysis: gap-analysis
scope: "spec/functional/FR-001-refresh-token-rotation.md, spec/functional/FR-002-request-authorization.md, spec/functional/FR-003-access-decision-audit.md"
review_set: subset
---
## Summary

The profile evaluation binds four clauses, leaves two unresolved, and marks two not binding. The requirements cover the refresh-token lifecycle clause; no requirement covers the other two binding clauses.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | medium | No requirement establishes or maintains access-control policy and procedures for this component. | AC-1 |
| FND-002 | medium | No requirement specifies canonical decoding of input before further processing. | V1.1.1 |
| FND-003 | low | Applicability of CP-9 cannot be resolved because the source-profile context omits `data_class`. | CP-9 |
| FND-004 | low | Applicability of V1.1.3 cannot be resolved because the source-profile context omits `chapter`. | V1.1.3 |
