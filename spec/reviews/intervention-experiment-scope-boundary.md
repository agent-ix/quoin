---
id: SR-065
title: "Scope-boundary review of intervention-experiment evidence"
type: SpecReview
analysis: scope-boundary
scope: "US-015; FR-056; FR-057; TC-1195..TC-1216"
review_set: all
---

## Summary

Quoin owns record validation, evidence-store intake, and deterministic reporting.
The external producer owns experiment execution and supplies the record and retained
raw bytes; the authored plan supplies the governing definition. These boundaries
exclude producer execution from Quoin.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No scope defect found; inputs, outputs, responsibility allocation, and the no-execution constraint cover every cross-boundary exchange. | FR-057 Inputs; FR-057 Outputs; FR-057-CON-1; TC-1214 |
