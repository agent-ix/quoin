---
id: SR-20261117175249
title: "Gap analysis of PLAT-1640 Quoin ratings coverage"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; PLAT-1640; FR-235-AC-4, FR-235-AC-11; changed validation coverage implementation and focused fixture"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quoin/FR-235", type: references}
---

## Summary

Ticket: PLAT-1640. The computed matrix binds FR-235-AC-4 and FR-235-AC-11 to the new focused fixture. The fixture asserts concrete coverage values, invalid marker reporting, and exit changes when a high-risk marker is removed. Reverse-gap inspection found the new coverage functions tagged to those criteria; `string_field` is an untagged helper and is separately reported by code-review. No new source or test stubs were found. Plan completion: not assessed.

The Quoin coverage report lists 201 high-process-risk FRs as uncovered. The planner has marked ticket acceptance criterion 5 pending an owner decision about bulk self-test coverage, so this count is recorded as pending scope and is not treated as a defect in this focused checker PR. The other two repository counts were not independently measured in this review.

## Verdict

PASS for this focused implementation slice. The command behavior added in the PR is traceable to FR-235-AC-4 and AC-11, and the fixture exercises the principal coverage outcomes. The 201 uncovered high-risk FRs are recorded as the pending owner decision for ticket AC5, not as a defect in this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No gaps found (placeholder) | - |

## Coverage

- FR-235-AC-4 and FR-235-AC-11: tagged by the new `tc_235_ratings_coverage_distinguishes_high_not_high_unrated_and_invalid_markers` test.
- Ticket PLAT-1640 AC5: pending owner decision on bulk case coverage; Quoin reports 201 uncovered high-risk FRs. Quire-RS and Quire CLI counts were not independently checked in this pass.
- Reverse gap: new coverage behavior is owned by the two FR-235 criteria; the missing `implements` marker on `string_field` is recorded in the code-review artifact.
- Stubs and coverage inflation: none found in the changed test or source implementation.
- Semantic review: skipped; not requested and the acceptance gap is mechanical.
- Plan completion: not assessed.
