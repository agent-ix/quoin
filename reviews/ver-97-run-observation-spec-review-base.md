---
id: SR-20261113234705
title: "Base spec review of VER-97 read-time analysis observations"
type: SpecReview
analysis: base
scope: "spec/functional/FR-153-analysis-result-counting-by-disposition.md"
review_set: subset
---

# Base spec review of VER-97 read-time analysis observations

## Summary

I reviewed FR-153's read-time observation input contract against FR-030, FR-115, FR-116, FR-152, FR-184 and engineering-assurance FR-036/FR-037. AP-201 applies because this change affects retained-evidence report qualification/rendering; AP-201's required evidence, integrity and scope-boundary analyses are included in the enforced review run. AP-202 does not apply because the change neither evaluates Jev lenses nor changes whether a lens is wired into a skill. The checkout and selected HEAD values are one-shot FR-116 host observations; the amendment adds no persisted field, digest, ledger or pin.

The earlier wire-contract gap is closed in this amendment: JSON types and empty-value outcomes are stated, and `actor_authority` uses FR-116's `ActorAdmission` field names and types. The external FR-036/FR-037 signature-result schema remains referenced rather than restated.

The existing FR-153-AC-1 through AC-11 are tagged in `analysis_counting.rs`. The six new criteria, AC-12 through AC-17, and existing FR-153-CON-2 are untagged in the computed matrix. This is a real implementation-stage evidence gap, not a missing requirement in this spec-only change: tests and trace tags must land with the CLI implementation, and this review does not claim that coverage exists.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The computed matrix reports FR-153-AC-12 through AC-17 and FR-153-CON-2 as untagged. Add CLI tests that exercise the new typed input protocol, actor-authority boundaries and read-time failure cases, plus a trace tag for CON-2, in the implementation lane. This remains a downstream test-evidence gap; no existing test is tagged as covering behavior it does not exercise. | FR-153-AC-12–AC-17, FR-153-CON-2 | correct-requirement-no-evidence |

## Verdict

The specification is ready for implementation with FND-001 carried forward as an implementation-lane test obligation. AP-201 coverage is complete for this spec-review operation; its separate code-review and gap-analysis operations are not satisfied by this review. SHA values are transient FR-116 performed-revision and selected-source observations and are not stored or used as pins.

## Examined

FR-153 Inputs, Behavior, Constraints and AC-1 through AC-17; FR-030-AC-1; FR-115-AC-14; FR-116 Inputs, CON-1/CON-2 and AC-7/AC-8/AC-9/AC-18; FR-152-CON-1; FR-184-CON-1; engineering-assurance FR-036/FR-037 signature-result terms; `quire matrix --scope . --format tsv`.
