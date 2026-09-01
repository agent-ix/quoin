---
id: SR-068
title: "Failure-domain review of operational evidence"
type: SpecReview
analysis: failure-domain
scope: "US-016; FR-059; FR-060; FR-061; TC-1223..TC-1243; TC-1244..TC-1248"
review_set: all
---

## Summary

The review exercised stale observations, impossible timestamp order, forged clock
labels, mode confusion, unavailable controls, invalid pin joins, raw-byte mismatch,
identity collision, mismatched release exports, and partial-pair persistence. Each
now has a refusal, rollback, or non-discharge outcome.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | Resolved: Quoin derives clock status from retained timestamps and immutable observation time instead of trusting a producer label. | FR-059-AC-6; FR-060-AC-7; TC-1228; TC-1238 | wrong-requirement |
| FND-002 | high | Resolved: a drill or unsuccessful exercise cannot discharge an obligation unless its mode is accepted and its successful, matched clock evidence is verified. | FR-060-AC-7; TC-1238 | missing-requirement |
| FND-003 | high | Resolved: unavailable, unknown, and not-applicable capabilities cannot render as affirmative control-exists evidence. | FR-060-AC-8; TC-1239 | wrong-requirement |
| FND-004 | high | Resolved: unsafe, missing, wrong-sized, or digest-mismatched raw evidence refuses intake without a record write. | FR-059-AC-9; FR-060-AC-2; TC-1231; TC-1233 | missing-requirement |
| FND-005 | high | Resolved: malformed or mismatched workflow/run/job artifacts and partial linked-pair writes are explicitly refused, while every non-success conclusion remains visible and non-discharging. | FR-061-AC-2; FR-061-AC-4; TC-1245; TC-1247; TC-1248 | missing-requirement |
