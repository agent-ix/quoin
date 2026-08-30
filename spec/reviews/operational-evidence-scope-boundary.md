---
id: SR-073
title: "Scope-boundary review of operational evidence"
type: SpecReview
analysis: scope-boundary
scope: "US-016; FR-059; FR-060; FR-061; TC-1223..TC-1243; TC-1244..TC-1248"
review_set: all
---

## Summary

Quoin owns validation, evidence-store intake, deterministic clock interpretation,
obligation matching, and reporting. External producers own control execution and
evidence capture; Quoin's first-party adapter only parses retained bytes. Authored
plans own definitions and accepted modes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No scope defect remains; Quoin verifies evidence but never invokes, drills, or alters an operational control. | FR-060 Inputs; FR-060-CON-1; TC-1242 |
| FND-002 | high | Resolved: FR-061 consumes retained workflow/API artifacts but owns no GitHub access, workflow dispatch, release publication, process execution, or control alteration. | FR-061 Constraints; TC-1248 |
