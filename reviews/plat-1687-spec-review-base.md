---
id: SR-20261113203419
title: "Base spec review of PLAT-1687 FR-185"
type: SpecReview
analysis: base
scope: "agent-ix/quoin; PLAT-1687 PR #888: spec/functional/FR-185-in-toto-test-result-export.md"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-185"
    type: references
---

## Summary

Base checklist review of FR-185's identifiers, interfaces, error behavior, criteria, and coverage. The requirement is specific and testable, and the matrix identifies a binder for every test criterion; the functional-requirements index omits this new FR, leaving it undiscoverable from the maintained index.

## Verdict

CONDITIONAL — the new FR must be added to `spec/functional/index.md`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-185 is absent from `spec/functional/index.md`, whose purpose is to index the repository's functional requirements; readers using the index cannot discover the new requirement. | spec/functional/index.md:166 |

## Checklist Evidence

- FR-185 is the next identifier after FR-184; criterion and constraint IDs are consistently parented and unique.
- Inputs, output, classification, error taxonomy, and dependencies are stated. `quire validate --scope . spec/functional/FR-185-in-toto-test-result-export.md --summary` exits 0 with the unrelated `spec-artifacts-app` unused-import notice; the grammar summary is clean.
- The `quire matrix` output identifies tags for AC-1 through AC-6 and AC-8, and a tagged static check for AC-7. The Quoin matrix discrepancy is recorded in the separate gap-analysis artifact.
- No duplicated upstream schema or protobuf files were introduced. The spec references the in-toto predicate URL.
- The caller-supplied digest requirement is the approved subject identity binding: Quoin validates and preserves it but computes no digest and records no additional digest.
