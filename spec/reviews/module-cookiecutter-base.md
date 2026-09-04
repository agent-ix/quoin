---
id: SR-128
title: "Base review of the semantic-module cookiecutter"
type: SpecReview
analysis: base
scope: "StR-008; US-021; FR-076..FR-083; NFR-018; NFR-019; TC-1400..TC-1448"
review_set: all
---

# SR-128: Base review of the semantic-module cookiecutter

## Summary

ID formats, cross-links, and AC-to-TC coverage across StR-008, US-021, FR-076..FR-083, NFR-018/019, and TC-1400..1448 are sound; three documented error-path behaviors have no acceptance criterion or test case exercising them.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-077 Behavior states tsp compile failure SHALL fail the emit command carrying compiler diagnostics and leave committed output untouched, but no AC or TC exercises this path; AC-5 covers only the base/version-mismatch failure | FR-077 | missing-requirement |
| FND-002 | medium | FR-080 Behavior states that when the grammar package is not installed the rendered suite SHALL fail naming the install command rather than skipping schema checks, but no AC or TC covers this, only the engine-absent and missing-capability failures | FR-080 | missing-requirement |
| FND-003 | low | FR-076 Behavior requires recording every declared-but-unemitted target in the rendered README and Test Matrix, but no AC or TC asserts this; only the invalid-target abort path (AC-8, TC-1407) is tested | FR-076 | missing-requirement |

