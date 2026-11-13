---
id: SR-202
title: "EARS review of quoin PR 836"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quoin@33a5d210; spec/functional/FR-063-change-assurance-record-integrity.md, spec/functional/FR-065-change-assurance-verification.md, spec/functional/FR-068-change-assurance-cli-surface.md, spec/usecase/US-017-verify-change-assurance-evidence.md"
review_set: subset
---

# EARS review of quoin PR 836

## Summary

Ticket: PLAT-1509. `quire validate` reports no EARS warning on any line this PR changes.
The three FR-068 warnings (lines 79, 105, 108) are on lines this PR leaves untouched,
so they are out of scope.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-002 | low | New Behavior bullet 'record-digest SHALL hash the stored file bytes unchanged and SHALL write nothing' has two shalls (quire ears:non-singular, FR-068 line 97). Split. | spec/functional/FR-068-change-assurance-cli-surface.md:97 | wrong-requirement |
