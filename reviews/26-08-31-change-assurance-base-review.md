---
id: SR-090
title: "Base review of Quoin change-assurance contracts"
type: SpecReview
analysis: base
scope: "US-017, FR-063..FR-065, TC-1261..TC-1292"
review_set: all
---

# SR-090: Base review of Quoin change-assurance contracts

## Summary

The base checklist traced the reviewer story through three atomic contracts and
32 matrix rows. The review resolved two input-contract ambiguities before
planning; the resulting set is complete, testable, indexed, and structurally
valid.

## Findings

| ID      | Severity | Summary                                                                                                                                                                                   | Refs                                                                      | Escape Cause        |
| ------- | -------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- | ------------------- |
| FND-001 | high     | Resolved: raw duplicate proof selections were declared invalid but the verifier input was not defined in a form that preserved duplicates.                                                | FR-065 Selection and checks; FR-065-AC-3; TC-1283                         | missing-requirement |
| FND-002 | high     | Resolved: duplicate JSON names had to be refused although an already parsed object cannot retain them; the boundary now requires exact UTF-8 JSON bytes and duplicate-preserving parsing. | FR-063 Canonical integrity contract; FR-064 Intake and integrity; TC-1267 | wrong-requirement   |

## Resolution

FR-063 and FR-064 now define the raw-byte parsing boundary. FR-065 defines an
explicit selection-entry array, duplicate grouping, unknown-proof handling, and
the exact reason codes. Matrix rows TC-1267 and TC-1283 were aligned.
