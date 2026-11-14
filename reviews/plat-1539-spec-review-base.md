---
id: SR-20261114002405
title: "Base specification review of PLAT-1539 quoin#893"
type: SpecReview
analysis: base
scope: "agent-ix/quoin; spec/functional/FR-171-security-analysis-emits-typed-controls.md"
review_set: subset
---

## Summary

Reviewed changed FR-171 behavior and AC-8 through AC-14 against PLAT-1539 scope, FR-180/FR-181, and spec-objects-security FR-021. The amendment largely preserves the ticket's no-profile and no-context-write boundaries, but its default-profile wording claims availability that FR-021 does not guarantee.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-171 says all three named profiles are “available defaults,” but FR-021 ships up to three and only adds a profile when its intended source set is published. The skill must name the three defined profile names without claiming all are currently available. | spec/functional/FR-171-security-analysis-emits-typed-controls.md:56 |

## Scope

- FR-171 Behavior lines 46-58 and AC-8 through AC-14 examined; FR-180, FR-181 and spec-objects-security FR-021 used as context.
- The edited FR-171 file validates independently. Full spec-tree validation is reported by the author to fail on the existing FR-113-AC-6 table-width issue; filament is unavailable.
