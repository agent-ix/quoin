---
id: SR-091
title: "Failure-domain review of Quoin change-assurance contracts"
type: SpecReview
analysis: failure-domain
scope: "FR-063..FR-065"
review_set: all
---

# SR-091: Failure-domain review of Quoin change-assurance contracts

## Summary

The review challenged malformed serialization, interrupted writes, collisions,
missing lineage, repeated selections, repeated decisions, and absent evidence.
One crash-consistency gap and one repeated-event ambiguity were resolved.

## Findings

| ID      | Severity | Summary                                                                                                                           | Refs                                              | Escape Cause        |
| ------- | -------- | --------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------- | ------------------- |
| FND-001 | high     | Resolved: “atomic write” did not say how an interruption between attestation and output persistence avoided exposing a half-pair. | FR-064 Intake and integrity; FR-064-AC-6; TC-1277 | missing-requirement |
| FND-002 | medium   | Resolved: exactly one review decision was required without stating the outcome when multiple exact matching events exist.         | FR-065 Selection and checks; FR-065-AC-7; TC-1287 | missing-requirement |

## Resolution

FR-064 now requires a fully staged, durably closed directory followed by one
atomic rename and cleanup of interrupted staging directories. FR-065 classifies
duplicate matching decisions as invalid with `decision_mismatch`.
