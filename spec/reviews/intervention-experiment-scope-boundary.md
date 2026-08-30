---
id: SR-065
title: "Scope-boundary review of intervention-experiment evidence"
type: SpecReview
analysis: scope-boundary
scope: "US-015; FR-056; FR-057; FR-058; TC-1195..TC-1216; TC-1217..TC-1221"
review_set: all
---

## Summary

Quoin owns record validation, evidence-store intake, and deterministic reporting.
External tooling owns experiment execution and supplies retained raw reports; the
first-party Quoin adapter computes a record from those bytes without spawning the
experiment. The authored plan supplies the governing definition.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No scope defect remains; intake, adapter, external-runner, and report responsibilities cover every exchange without making Quoin an experiment runner. | FR-057-CON-1; FR-058; TC-1214; TC-1221 |
