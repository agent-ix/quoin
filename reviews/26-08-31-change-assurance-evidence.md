---
id: SR-094
title: "Evidence-method review of Quoin change-assurance contracts"
type: SpecReview
analysis: evidence
scope: "FR-063-AC-1..FR-065-AC-12, TC-1261..TC-1292"
review_set: all
---

# SR-094: Evidence-method review of Quoin change-assurance contracts

## Summary

Deterministic catalog advice was compared with all 32 authored verification
obligations. Three combined static-boundary criteria were reclassified from
Inspection to Analysis; the target set now has zero advisor mismatches.

## Findings

| ID      | Severity | Summary                                                                                                                                                        | Refs                                    | Escape Cause      |
| ------- | -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------- | ----------------- |
| FND-001 | medium   | Resolved: three invariant/static-boundary criteria were authored as Inspection even though their matrix evidence and catalog advice classify them as Analysis. | FR-063-AC-11; FR-064-AC-9; FR-065-AC-12 | wrong-requirement |

## Evidence

`quoin advise --repo . --mismatch-only --json`, filtered to FR-063..FR-065,
returned an empty list after correction. Unit, property, integration, and static
rows remain explicit in TC-1261..TC-1292.
